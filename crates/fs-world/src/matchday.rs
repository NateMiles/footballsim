//! Bridges the world and the match engine: player snapshots, team
//! selection, and playing the day's fixtures.

use crate::command::Event;
use crate::economy;
use crate::fixture::{self, FixtureRef};
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
        cohesion: fs_match::input::DRILLED,
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
    // The shape that suits the squad best, allowing for how well the club
    // knows each one, so clubs stay with shapes they have drilled.
    let known = &world.clubs()[club].familiarity;
    let value = |f: Formation| {
        sheet_quality(squad, f)
            * (0.95 + 0.05 * f64::from(crate::training::cohesion(known.get(f))) / 10_000.0)
    };
    let formation = Formation::ALL
        .into_iter()
        .max_by(|&a, &b| {
            value(a)
                .total_cmp(&value(b))
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
    let sheet = match c.lineup() {
        Some(lineup) => sheet_from_lineup(&squad, tactic, lineup),
        None => pick_team(&squad, tactic),
    };
    with_cohesion(world, club, sheet)
}

/// Sets every player's cohesion from how well the club knows the sheet's
/// formation.
fn with_cohesion(world: &World, club: ClubId, mut sheet: TeamSheet) -> TeamSheet {
    let familiarity = world.clubs()[club].familiarity.get(sheet.tactic.formation);
    let cohesion = crate::training::cohesion(familiarity);
    for p in sheet.starters.iter_mut().chain(sheet.bench.iter_mut()) {
        p.cohesion = cohesion;
    }
    sheet
}

/// The club's team sheet if it played `tactic` today: its chosen lineup
/// when that fits the formation, otherwise the best available eleven
/// for it. For trying tactics out (predictions); changes nothing.
pub fn team_sheet_with(world: &World, club: ClubId, tactic: Tactic) -> TeamSheet {
    let squad = squad(world, club);
    let c = &world.clubs()[club];
    let sheet = match c.lineup() {
        Some(lineup) if c.tactic().is_some_and(|t| t.formation == tactic.formation) => {
            sheet_from_lineup(&squad, tactic, lineup)
        }
        _ => pick_team(&squad, tactic),
    };
    with_cohesion(world, club, sheet)
}

/// The match input for a scheduled fixture, as a watching manager sees
/// it: both team sheets as they stand today, the fixture's random stream
/// and, for cup ties, the venue and how a level tie is settled, with
/// automatic substitutions off for clubs managed live. When the manager
/// submits the instructions they gave ([`crate::Command::PlayMatch`]),
/// the world plays exactly this input plus those instructions, so the
/// result is the one they watched.
///
/// # Panics
/// Panics if the fixture does not exist.
pub fn fixture_input(world: &World, fixture: FixtureRef) -> MatchInput {
    let f = fixture.get(world).expect("fixture exists");
    let mut input = MatchInput::new(
        team_sheet(world, f.home),
        team_sheet(world, f.away),
        world.seed(),
        fixture.match_key(world),
    );
    if let FixtureRef::Cup { cup, index } = fixture {
        let (neutral, decider) = crate::cup::match_rules(world, cup, index as usize);
        input.neutral = neutral;
        input.decider = decider;
    }
    // A manager watching live makes their own substitutions.
    input.home.auto_subs = !world.watched_live(f.home);
    input.away.auto_subs = !world.watched_live(f.away);
    input
}

/// Recomputes every club's AI tactic from its current squad.
pub(crate) fn refresh_ai_tactics(world: &mut World) {
    let all: Vec<ClubId> = world.clubs.ids().collect();
    refresh_tactics_of(world, &all);
}

/// Daily: the clubs due a review of their AI tactic (more often the more
/// important they are; see [`crate::importance`]) recompute it.
pub(crate) fn review_ai_tactics(world: &mut World) {
    let importance = crate::importance::of_clubs(world);
    let due: Vec<ClubId> = world
        .clubs
        .ids()
        .filter(|&c| crate::importance::reviews_tactic_today(world, c, importance[c.index()]))
        .collect();
    refresh_tactics_of(world, &due);
}

fn refresh_tactics_of(world: &mut World, clubs: &[ClubId]) {
    let mut tactics: Vec<Option<Tactic>> = vec![None; clubs.len()];
    {
        let world = &*world;
        fs_core::par::for_each_heavy(&mut tactics, |i, slot| {
            let c = clubs[i];
            *slot = Some(default_tactic(world, c, &squad(world, c)));
        });
    }
    for (&c, t) in clubs.iter().zip(tactics) {
        world.clubs[c].ai_tactic = t;
    }
}

/// Which engine plays a league's matches: the detailed one in the league
/// of any human-managed club (its clubs are of high importance, see
/// [`crate::importance`]), the fast one elsewhere.
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

/// Which engine plays a fixture: leagues as [`league_tier`] says, cup
/// matches in detail when a human-managed club plays.
pub fn fixture_tier(world: &World, fixture: FixtureRef) -> Tier {
    match fixture {
        FixtureRef::League { league, .. } => league_tier(world, league),
        FixtureRef::Cup { .. } => {
            let f = fixture.get(world).expect("fixture exists");
            if world.is_human_club(f.home) || world.is_human_club(f.away) {
                Tier::Event
            } else {
                Tier::Statistical
            }
        }
    }
}

/// Plays every fixture scheduled for today and applies the results.
pub(crate) fn play_today(
    world: &mut World,
    events: &mut Vec<Event>,
    instructions: &BTreeMap<ClubId, Vec<TimedInstruction>>,
) {
    let todo = fixture::due(world);
    if todo.is_empty() {
        return;
    }
    let league_tiers: Vec<Tier> = world.leagues.ids().map(|l| league_tier(world, l)).collect();
    let tiers: Vec<Tier> = todo
        .iter()
        .map(|&r| match r {
            FixtureRef::League { league, .. } => league_tiers[league.index()],
            FixtureRef::Cup { .. } => fixture_tier(world, r),
        })
        .collect();
    let mut results: Vec<Option<MatchResult>> = vec![None; todo.len()];
    crate::profile::time("matches: play", || {
        let world = &*world;
        fs_core::par::for_each_heavy(&mut results, |k, slot| {
            let r = todo[k];
            let f = r.get(world).expect("fixture exists");
            let mut input = fixture_input(world, r);
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
            *slot = Some(fs_match::play(&input, tiers[k]));
        });
    });
    crate::profile::time("matches: apply", || {
        for ((r, tier), result) in todo.into_iter().zip(tiers).zip(results) {
            let result = result.expect("played");
            apply_result(world, r, tier, &result, events);
        }
    });
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
    fixture: FixtureRef,
    tier: Tier,
    r: &MatchResult,
    events: &mut Vec<Event>,
) {
    let today = world.date();
    let seed = world.seed();
    let key = fixture.match_key(world);
    let (home, away) = {
        let f = fixture.get(world).expect("fixture exists");
        (f.home, f.away)
    };
    let clubs = [home, away];
    let neutral = match fixture {
        FixtureRef::Cup { cup, index } => crate::cup::match_rules(world, cup, index as usize).0,
        FixtureRef::League { .. } => false,
    };
    if neutral {
        economy::neutral_receipts(world, home, away, key);
    } else {
        economy::gate_receipts(world, home, away, key);
    }
    for (side, &club) in clubs.iter().enumerate() {
        let opponents: Vec<PlayerId> = r.players[1 - side].iter().map(|p| p.player).collect();
        crate::scouting::after_match(world, club, &opponents);
    }

    // The competition's record.
    let scorers: Vec<(u8, PlayerId)> = r
        .events
        .iter()
        .filter_map(|e| match e {
            MatchEvent::Goal { side, scorer, .. } => Some((side.index() as u8, *scorer)),
            _ => None,
        })
        .collect();
    let record = FixtureResult {
        goals: r.goals,
        xg: [r.stats[0].xg, r.stats[1].xg],
        tier,
        scorers,
        extra_time: r.extra_time,
        penalties: r.shootout.as_ref().map(|s| s.score),
    };
    let reported = match fixture {
        FixtureRef::League { league, index } => {
            let season = world.leagues[league].season.as_mut().expect("season");
            season.fixtures[index as usize].result = Some(record);
            for (side, &club) in clubs.iter().enumerate() {
                if let Some(row) = season.rows.iter_mut().find(|row| row.club == club) {
                    row.record(r.goals[side], r.goals[1 - side]);
                }
            }
            world.leagues[league]
                .clubs
                .iter()
                .any(|&c| world.is_human_club(c))
        }
        FixtureRef::Cup { cup, index } => {
            crate::cup::record_result(world, cup, index as usize, record);
            world.is_human_club(home) || world.is_human_club(away)
        }
    };
    if reported {
        events.push(Event::MatchPlayed {
            fixture,
            home,
            away,
            goals: r.goals,
        });
    }

    if let FixtureRef::League { .. } = fixture {
        for (side, &club) in clubs.iter().enumerate() {
            let points = match r.goals[side].cmp(&r.goals[1 - side]) {
                core::cmp::Ordering::Greater => 3,
                core::cmp::Ordering::Equal => 1,
                core::cmp::Ordering::Less => 0,
            };
            crate::board::after_match(world, club, points, events);
        }
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
                let mut rng = seed.derive(Stream::Injury, key, u64::from(p.raw()));
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
