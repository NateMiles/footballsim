//! Bridges the world and the match engine: player snapshots, team
//! selection, and playing the day's fixtures.

use crate::command::Event;
use crate::economy;
use crate::ids::{ClubId, LeagueId, PlayerId};
use crate::league::FixtureResult;
use crate::lineup::sheet_from_lineup;
use crate::player::{CONDITION_MAX, Injury, Severity};
use crate::staff;
use crate::world::World;
use fs_core::attributes::Position;
use fs_core::{GameDate, Stream};
use fs_match::{
    Formation, MatchEvent, MatchInput, MatchResult, PlayerSnapshot, Tactic, TeamSheet, Tier,
    TimedInstruction, pick_team,
};
use std::collections::BTreeMap;

/// A player's state at kickoff.
pub fn snapshot(world: &World, id: PlayerId) -> PlayerSnapshot {
    let p = world.players();
    let c = p.condition(id);
    PlayerSnapshot {
        id,
        attributes: *p.attributes(id),
        hidden: *p.hidden(id),
        positions: *p.positions(id),
        fitness: c.fitness,
        sharpness: c.sharpness,
        morale: c.morale,
    }
}

/// Snapshots of a club's players who can play today (not injured or
/// suspended). If fewer than eleven are available, unavailable players
/// are added back so the club can still field a team.
pub fn squad(world: &World, club: ClubId) -> Vec<PlayerSnapshot> {
    let today = world.date();
    let ids = world.clubs()[club].squad();
    let mut picked: Vec<PlayerId> = ids
        .iter()
        .copied()
        .filter(|&p| world.players().available(p, today))
        .collect();
    if picked.len() < 11 {
        let missing = 11 - picked.len();
        let extra: Vec<PlayerId> = ids
            .iter()
            .copied()
            .filter(|p| !picked.contains(p))
            .take(missing)
            .collect();
        picked.extend(extra);
    }
    picked.into_iter().map(|p| snapshot(world, p)).collect()
}

/// The tactic an AI manager picks: the formation that fits the squad best,
/// with sliders that give each club a stable, distinct style.
pub fn default_tactic(world: &World, club: ClubId, squad: &[PlayerSnapshot]) -> Tactic {
    let mut rng = world.seed().derive(Stream::Ai, u64::from(club.raw()), 1);
    let mut slider = || rng.range_i32(-1, 1) as i8;
    let style = Tactic {
        formation: Formation::F442,
        mentality: slider(),
        tempo: slider(),
        width: slider(),
        pressing: slider(),
        defensive_line: slider(),
        directness: slider(),
    };
    if squad.len() < 11 {
        return style;
    }
    let formation = Formation::ALL
        .into_iter()
        .max_by(|&a, &b| {
            sheet_quality(squad, a)
                .total_cmp(&sheet_quality(squad, b))
                .then((b as u8).cmp(&(a as u8)))
        })
        .unwrap_or(Formation::F442);
    Tactic { formation, ..style }
}

fn sheet_quality(squad: &[PlayerSnapshot], formation: Formation) -> f64 {
    let sheet = pick_team(squad, Tactic::balanced(formation));
    let slots = formation.slots();
    sheet
        .starters
        .iter()
        .zip(slots)
        .map(|(p, s)| fs_match::selection::suitability(p, s.position))
        .sum()
}

/// The club's team sheet for today: its tactic (the manager's, the AI's
/// cached one, or a freshly computed one) and the best available eleven.
///
/// # Panics
/// Panics if the club has fewer than eleven players in total.
pub fn team_sheet(world: &World, club: ClubId) -> TeamSheet {
    let squad = squad(world, club);
    let c = &world.clubs()[club];
    let tactic = c
        .tactic()
        .unwrap_or_else(|| default_tactic(world, club, &squad));
    match c.lineup() {
        Some(lineup) => sheet_from_lineup(&squad, tactic, lineup),
        None => pick_team(&squad, tactic),
    }
}

/// The match input for a scheduled league fixture, as a watching manager
/// sees it: both team sheets as they stand today and the fixture's random
/// stream, with automatic substitutions off for clubs managed live. When
/// the manager submits the instructions they gave
/// ([`crate::Command::PlayMatch`]), the world plays exactly this input
/// plus those instructions, so the result is the one they watched.
///
/// # Panics
/// Panics if the league has no season or the fixture does not exist.
pub fn fixture_input(world: &World, league: LeagueId, index: usize) -> MatchInput {
    let season = world.leagues()[league]
        .season()
        .expect("season in progress");
    let f = &season.fixtures[index];
    let mut input = MatchInput::new(
        team_sheet(world, f.home),
        team_sheet(world, f.away),
        world.seed(),
        match_key(season.year, league, index),
    );
    // A manager watching live makes their own substitutions.
    input.home.auto_subs = !world.watched_live(f.home);
    input.away.auto_subs = !world.watched_live(f.away);
    input
}

/// Recomputes every club's AI tactic from its current squad.
pub(crate) fn refresh_ai_tactics(world: &mut World) {
    let mut tactics: Vec<Option<Tactic>> = vec![None; world.clubs.len()];
    {
        let world = &*world;
        fs_core::par::for_each_heavy(&mut tactics, |i, slot| {
            let c = ClubId::new(i as u32);
            *slot = Some(default_tactic(world, c, &squad(world, c)));
        });
    }
    for (c, t) in world.clubs.ids().zip(tactics) {
        world.clubs[c].ai_tactic = t;
    }
}

/// Which engine plays a league's matches: the detailed one wherever a
/// human manager is watching, the fast one elsewhere.
pub fn league_tier(world: &World, league: LeagueId) -> Tier {
    if world.leagues()[league]
        .clubs()
        .iter()
        .any(|&c| world.is_human_club(c))
    {
        Tier::Event
    } else {
        Tier::Statistical
    }
}

/// A unique key for a fixture's random stream.
fn match_key(year: i32, league: LeagueId, index: usize) -> u64 {
    ((year as u64) << 40) ^ (u64::from(league.raw()) << 20) ^ index as u64
}

/// Plays every fixture scheduled for today and applies the results.
pub(crate) fn play_today(
    world: &mut World,
    events: &mut Vec<Event>,
    instructions: &BTreeMap<ClubId, Vec<TimedInstruction>>,
) {
    let today = world.date();
    let mut todo: Vec<(LeagueId, usize, i32)> = Vec::new();
    for (lid, league) in world.leagues.iter() {
        if let Some(season) = &league.season {
            let mut i = season.next;
            while i < season.fixtures.len() && season.fixtures[i].date <= today {
                todo.push((lid, i, season.year));
                i += 1;
            }
        }
    }
    if todo.is_empty() {
        return;
    }
    let tiers: Vec<Tier> = world.leagues.ids().map(|l| league_tier(world, l)).collect();
    let mut results: Vec<Option<MatchResult>> = vec![None; todo.len()];
    {
        let world = &*world;
        fs_core::par::for_each_heavy(&mut results, |k, slot| {
            let (lid, i, _) = todo[k];
            let f = &world.leagues()[lid].season().expect("season").fixtures[i];
            let mut input = fixture_input(world, lid, i);
            // Clubs whose manager watched get exactly their instructions
            // and no automatic substitutions; everyone else (including a
            // watcher who skipped the match) gets the assistant's.
            input.home.auto_subs = !instructions.contains_key(&f.home);
            input.away.auto_subs = !instructions.contains_key(&f.away);
            for club in [f.home, f.away] {
                if let Some(given) = instructions.get(&club) {
                    input.instructions.extend_from_slice(given);
                }
            }
            input.instructions.sort_by_key(|x| x.step);
            *slot = Some(fs_match::play(&input, tiers[lid.index()]));
        });
    }
    for ((lid, i, year), result) in todo.into_iter().zip(results) {
        let result = result.expect("played");
        apply_result(world, lid, i, year, tiers[lid.index()], &result, events);
    }
    for league in world.leagues.as_mut_slice() {
        if let Some(season) = &mut league.season {
            while season.next < season.fixtures.len()
                && season.fixtures[season.next].result.is_some()
            {
                season.next += 1;
            }
        }
    }
}

fn apply_result(
    world: &mut World,
    lid: LeagueId,
    index: usize,
    year: i32,
    tier: Tier,
    r: &MatchResult,
    events: &mut Vec<Event>,
) {
    let today = world.date();
    let seed = world.seed();
    let (home, away) = {
        let f = &world.leagues[lid].season.as_ref().expect("season").fixtures[index];
        (f.home, f.away)
    };
    let clubs = [home, away];
    economy::gate_receipts(world, home, away, match_key(year, lid, index));
    for (side, &club) in clubs.iter().enumerate() {
        let opponents: Vec<PlayerId> = r.players[1 - side].iter().map(|p| p.player).collect();
        crate::scouting::after_match(world, club, &opponents);
    }

    // League record.
    let scorers: Vec<(u8, PlayerId)> = r
        .events
        .iter()
        .filter_map(|e| match e {
            MatchEvent::Goal { side, scorer, .. } => Some((side.index() as u8, *scorer)),
            _ => None,
        })
        .collect();
    {
        let season = world.leagues[lid].season.as_mut().expect("season");
        season.fixtures[index].result = Some(FixtureResult {
            goals: r.goals,
            xg: [r.stats[0].xg, r.stats[1].xg],
            tier,
            scorers,
        });
        for (side, &club) in clubs.iter().enumerate() {
            if let Some(row) = season.rows.iter_mut().find(|row| row.club == club) {
                row.record(r.goals[side], r.goals[1 - side]);
            }
        }
    }
    if world.leagues[lid]
        .clubs
        .iter()
        .any(|&c| world.is_human_club(c))
    {
        events.push(Event::MatchPlayed {
            league: lid,
            home,
            away,
            goals: r.goals,
        });
    }

    for (side, &club) in clubs.iter().enumerate() {
        let points = match r.goals[side].cmp(&r.goals[1 - side]) {
            core::cmp::Ordering::Greater => 3,
            core::cmp::Ordering::Equal => 1,
            core::cmp::Ordering::Less => 0,
        };
        crate::board::after_match(world, club, points, events);
    }

    for (side, &club) in clubs.iter().enumerate() {
        let scored = r.goals[side];
        let conceded = r.goals[1 - side];
        let morale_shift: i32 = match scored.cmp(&conceded) {
            core::cmp::Ordering::Greater => 250,
            core::cmp::Ordering::Equal => 30,
            core::cmp::Ordering::Less => -200,
        };
        let human = world.is_human_club(club);
        // Suspensions served: anyone banned before this match missed it.
        let squad = world.clubs[club].squad.clone();
        for &p in &squad {
            let i = p.index();
            if world.players.suspension[i] > 0 {
                world.players.suspension[i] -= 1;
            }
            let c = &mut world.players.condition[i];
            c.morale =
                (i32::from(c.morale) + morale_shift).clamp(0, i32::from(CONDITION_MAX)) as u16;
        }
        let mut played = vec![false; squad.len()];
        for ps in &r.players[side] {
            if ps.minutes == 0 {
                continue;
            }
            let p = ps.player;
            let i = p.index();
            if let Some(k) = squad.iter().position(|&q| q == p) {
                played[k] = true;
            }
            let st = &mut world.players.season[i];
            st.appearances += 1;
            st.starts += u16::from(ps.started);
            st.minutes += u32::from(ps.minutes);
            st.goals += u16::from(ps.goals);
            st.assists += u16::from(ps.assists);
            st.rating_sum += ps.rating;
            let keeps_clean = matches!(
                world.players.positions[i].best(),
                Position::Goalkeeper
                    | Position::DefenderCentre
                    | Position::DefenderLeft
                    | Position::DefenderRight
            );
            if conceded == 0 && ps.minutes >= 60 && keeps_clean {
                st.clean_sheets += 1;
            }
            // Bans: a red card, or every fifth yellow of the season.
            if ps.red {
                st.reds += 1;
                world.players.suspension[i] += if ps.yellow { 1 } else { 3 };
            } else if ps.yellow {
                st.yellows += 1;
                if st.yellows % 5 == 0 {
                    world.players.suspension[i] += 1;
                }
            }
            // Physical cost of playing; sharpness from game time.
            let mins = u32::from(ps.minutes);
            let stamina =
                u32::from(world.players.attributes[i].get(fs_core::attributes::Attr::Stamina));
            let c = &mut world.players.condition[i];
            let cost = mins * (2200 + 1600 * (1000 - stamina.min(1000)) / 1000) / 90;
            c.fitness = u32::from(c.fitness).saturating_sub(cost) as u16;
            c.fatigue = (u32::from(c.fatigue) + mins * 25).min(u32::from(CONDITION_MAX)) as u16;
            c.sharpness = (u32::from(c.sharpness) + mins * 45).min(u32::from(CONDITION_MAX)) as u16;
            if ps.rating >= 7.5 {
                c.morale = (c.morale + 150).min(CONDITION_MAX);
            }
            if ps.injured {
                let mut rng = seed.derive(
                    Stream::Injury,
                    match_key(year, lid, index),
                    u64::from(p.raw()),
                );
                let proneness = f64::from(
                    world.players.hidden[i].get(fs_core::attributes::HiddenAttr::InjuryProneness),
                ) / 1000.0;
                let treatment = staff::treatment_factor(&staff::ratings(world, club));
                let injury = sample_injury(&mut rng, today, proneness, treatment);
                world.players.injury[i] = Some(injury);
                if human {
                    events.push(Event::PlayerInjured {
                        player: p,
                        club,
                        until: injury.until,
                    });
                }
            }
        }
        // Squad players left out grow restless.
        for (k, &p) in squad.iter().enumerate() {
            if !played[k] {
                let c = &mut world.players.condition[p.index()];
                c.morale = c.morale.saturating_sub(40);
            }
        }
    }
}

/// Draws an injury's severity and length. Injury-prone players (0-1) are
/// out longer.
/// `treatment` scales the time out: 1 with an average medical team.
pub(crate) fn sample_injury(
    rng: &mut fs_core::Rng,
    today: GameDate,
    proneness: f64,
    treatment: f64,
) -> Injury {
    let roll = rng.unit_f64();
    let (severity, lo, hi) = if roll < 0.6 {
        (Severity::Minor, 3, 12)
    } else if roll < 0.9 {
        (Severity::Moderate, 14, 45)
    } else {
        (Severity::Serious, 60, 220)
    };
    let days = f64::from(rng.range_i32(lo, hi)) * (0.8 + 0.4 * proneness) * treatment;
    Injury {
        severity,
        since: today,
        until: today.add_days(days.round().max(1.0) as u32),
    }
}
