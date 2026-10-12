//! The yearly cycle: starting a season, and the 1 July rollover that ends
//! one season and begins the next.

use crate::command::Event;
use crate::competition::{double_round_robin, season_dates};
use crate::economy;
use crate::ids::{ClubId, LeagueId, PlayerId};
use crate::league::{Fixture, LeagueSeason, SeasonSummary, TableRow};
use crate::matchday::refresh_ai_tactics;
use crate::people::{Age, PlayerSpec, age_years, new_player, season_year};
use crate::player::{CareerSeason, SeasonStats, Status};
use crate::squad::maintain_squads;
use crate::world::World;
use fs_core::Stream;
use fs_core::attributes::Position;
use std::collections::BTreeMap;

/// Sets up the first season of a freshly built world: AI tactics,
/// league fixtures and cup draws for the season containing today's date.
pub fn start_first_season(world: &mut World) {
    refresh_ai_tactics(world);
    let year = season_year(world.date());
    start_season(world, year);
    crate::cup::start_editions(world, year, &mut Vec::new());
}

/// Creates fixtures and empty tables for every league.
pub(crate) fn start_season(world: &mut World, year: i32) {
    let seed = world.seed();
    for (lid, league) in world.leagues.as_mut_slice().iter_mut().enumerate() {
        let mut clubs = league.clubs.clone();
        let mut rng = seed.derive(Stream::Season, year as u64, lid as u64);
        rng.shuffle(&mut clubs);
        let rounds = double_round_robin(clubs.len());
        let dates = season_dates(year, rounds.len());
        let clubs = &clubs;
        let fixtures = rounds
            .iter()
            .zip(&dates)
            .flat_map(|(games, &date)| {
                games.iter().map(move |&(h, a)| Fixture {
                    date,
                    home: clubs[h],
                    away: clubs[a],
                    result: None,
                })
            })
            .collect::<Vec<_>>();
        league.season = Some(LeagueSeason {
            year,
            fixtures,
            next: 0,
            rows: league.clubs.iter().map(|&c| TableRow::new(c)).collect(),
        });
    }
}

/// The season year currently being played, if any league or cup has
/// one.
pub fn current_season(world: &World) -> Option<i32> {
    world
        .leagues()
        .iter()
        .find_map(|(_, l)| l.season().map(|s| s.year))
        .or_else(|| {
            world
                .cups()
                .iter()
                .find_map(|(_, c)| c.edition().map(|e| e.year))
        })
}

/// 1 July: close the old season and open the new one.
pub(crate) fn rollover(world: &mut World, events: &mut Vec<Event>) {
    let year = season_year(world.date());
    economy::close_accounts(world, year - 1);
    crate::board::season_end(world, events);
    close_season(world, events);
    crate::cup::close_editions(world);
    record_careers(world, year - 1);
    for (player, loan) in crate::transfer::end_loans(world) {
        if world.is_human_club(loan.parent) || world.is_human_club(loan.borrower) {
            events.push(Event::LoanEnded {
                player,
                from: loan.borrower,
                to: loan.parent,
            });
        }
    }
    expire_contracts(world, events);
    crate::staff::rollover(world, events);
    retirements(world, events);
    potential_drift(world);
    youth_intake(world, events);
    maintain_squads(world, events);
    refresh_ai_tactics(world);
    crate::happiness::rollover(world);
    crate::records::prune(world);
    start_season(world, year);
    crate::cup::start_editions(world, year, events);
    crate::board::set_targets(world);
    economy::invest(world);
    economy::set_budgets(world);
}

/// Archives every league's season and applies promotion and relegation.
fn close_season(world: &mut World, events: &mut Vec<Event>) {
    let mut summaries: Vec<Option<SeasonSummary>> = Vec::with_capacity(world.leagues.len());
    for (_, league) in world.leagues.iter() {
        summaries.push(league.season.as_ref().map(|s| {
            let mut goals: BTreeMap<PlayerId, u16> = BTreeMap::new();
            for f in &s.fixtures {
                if let Some(r) = &f.result {
                    for &(_, p) in &r.scorers {
                        *goals.entry(p).or_default() += 1;
                    }
                }
            }
            let top_scorer = goals
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
            SeasonSummary {
                year: s.year,
                table: s.table(),
                promoted: Vec::new(),
                relegated: Vec::new(),
                top_scorer,
            }
        }));
    }

    // Promotion and relegation between adjacent levels of each nation.
    let mut moves: Vec<(ClubId, LeagueId, LeagueId, bool)> = Vec::new();
    for (upper, ul) in world.leagues.iter() {
        let Some(lower) = world
            .leagues
            .iter()
            .find(|(_, l)| l.nation == ul.nation && l.level == ul.level + 1)
            .map(|(id, _)| id)
        else {
            continue;
        };
        let (Some(us), Some(ls)) = (&summaries[upper.index()], &summaries[lower.index()]) else {
            continue;
        };
        let n = usize::from(ul.swaps)
            .min(us.table.len() / 2)
            .min(ls.table.len() / 2);
        for row in us.table.iter().rev().take(n) {
            moves.push((row.club, upper, lower, false));
        }
        for row in ls.table.iter().take(n) {
            moves.push((row.club, lower, upper, true));
        }
    }
    for &(club, from, to, up) in &moves {
        world.move_club(club, to);
        if let Some(s) = summaries[from.index()].as_mut() {
            if up {
                s.promoted.push(club);
            } else {
                s.relegated.push(club);
            }
        }
        events.push(if up {
            Event::Promoted { club, from, to }
        } else {
            Event::Relegated { club, from, to }
        });
    }
    for (k, (league, summary)) in world
        .leagues
        .as_mut_slice()
        .iter_mut()
        .zip(summaries)
        .enumerate()
    {
        if let Some(s) = summary {
            if let Some(champion) = s.champion() {
                events.push(Event::SeasonEnded {
                    league: LeagueId::new(k as u32),
                    year: s.year,
                    champion,
                });
            }
            league.history.push(s);
            league.season = None;
        }
    }
}

/// Files the season's stats in each player's career and resets them.
fn record_careers(world: &mut World, year: i32) {
    let p = &mut world.players;
    for i in 0..p.len() {
        if p.status[i] == Status::Retired {
            continue;
        }
        let stats = std::mem::take(&mut p.season[i]);
        if stats.appearances > 0 || p.club[i].is_some() {
            let ability = p.positions[i].best().role_rating(&p.attributes[i]);
            p.career[i].push(CareerSeason {
                year,
                club: p.club[i],
                ability,
                stats,
            });
        }
        p.suspension[i] = 0;
        p.season[i] = SeasonStats::default();
    }
}

/// Releases everyone whose contract has run out.
fn expire_contracts(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let expired: Vec<(PlayerId, ClubId)> = world
        .players
        .ids()
        .filter_map(|p| {
            let c = world.players.contract(p)?;
            (c.expires < today).then_some((p, world.players.club(p)?))
        })
        .collect();
    for (p, club) in expired {
        world.release_player(p);
        if world.is_human_club(club) {
            events.push(Event::PlayerLeft { player: p, club });
        }
    }
}

/// Chance a player retires this summer.
fn retirement_chance(age: f64, ability: u16, free_agent: bool, seasons_unattached: u8) -> f64 {
    let mut p = match age as u32 {
        0..=30 => 0.0,
        31 => 0.03,
        32 => 0.08,
        33 => 0.15,
        34 => 0.3,
        35 => 0.45,
        36 => 0.6,
        _ => 0.85,
    };
    if free_agent {
        if age >= 29.0 {
            p = f64::max(p, 0.35);
        }
        p += 0.25 * f64::from(seasons_unattached);
    } else if ability >= 700 {
        p *= 0.6;
    }
    p.min(1.0)
}

fn retirements(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let seed = world.seed();
    let year = season_year(today) as u64;
    let mut retiring = Vec::new();
    for p in world.players.ids() {
        if world.players.status(p) != Status::Active {
            continue;
        }
        let club = world.players.club(p);
        let unattached = world.players.seasons_unattached[p.index()];
        let age = age_years(world.players.bio(p).birth_date, today);
        let chance = retirement_chance(age, world.players.ability(p), club.is_none(), unattached);
        let mut rng = seed.derive(Stream::Season, year, u64::from(p.raw()) | (1 << 40));
        if unattached >= 2 || rng.chance(chance) {
            retiring.push((p, club));
        } else if club.is_none() {
            world.players.seasons_unattached[p.index()] = unattached.saturating_add(1);
        }
    }
    for (p, club) in retiring {
        world.retire_player(p);
        if let Some(club) = club.filter(|&c| world.is_human_club(c)) {
            events.push(Event::PlayerRetired { player: p, club });
        }
    }
}

/// Each summer, every young player's ceiling moves a little: some kick on,
/// some stall. Youngsters who barely played lose a bit of potential.
fn potential_drift(world: &mut World) {
    let today = world.date();
    let seed = world.seed();
    let year = season_year(today) as u64;
    let p = &mut world.players;
    for i in 0..p.len() {
        if p.status[i] != Status::Active {
            continue;
        }
        let age = age_years(p.bio[i].birth_date, today);
        if age >= 24.0 {
            continue;
        }
        let mut rng = seed.derive(Stream::Development, year, i as u64 | (1 << 40));
        let minutes = p.career[i].last().map_or(0, |c| c.stats.minutes);
        let idle = if minutes < 900 { 20.0 } else { 0.0 };
        let current = f64::from(p.positions[i].best().role_rating(&p.attributes[i]));
        let pot = f64::from(p.potential[i]) + rng.normal(0.0, 30.0) - idle;
        p.potential[i] = pot.max(current).clamp(1.0, 980.0) as u16;
    }
}

/// Positions for youth intakes, roughly in squad proportions.
const YOUTH_POSITIONS: [Position; 12] = {
    use Position::*;
    [
        Goalkeeper,
        DefenderCentre,
        DefenderCentre,
        DefenderLeft,
        DefenderRight,
        DefensiveMidfielder,
        MidfielderCentre,
        MidfielderCentre,
        AttackingMidCentre,
        AttackingMidLeft,
        AttackingMidRight,
        Striker,
    ]
};

/// Three to five academy graduates join every club.
fn youth_intake(world: &mut World, events: &mut Vec<Event>) {
    let today = world.date();
    let seed = world.seed();
    let year = season_year(today) as u64;
    for club in world.clubs.ids() {
        let mut rng = seed.derive(Stream::Youth, year, u64::from(club.raw()));
        let count = 2 + rng.below(3);
        // An academy better than the club's standing turns out better
        // prospects: about 8 points of potential per point of youth
        // facilities above what such a club usually has.
        let (home_nation, level) = {
            let c = &world.clubs[club];
            let usual = crate::finance::usual_facilities(c.reputation);
            let academy = ((f64::from(c.facilities.youth) - usual) * 8.0) as i32;
            (c.nation, i32::from(c.reputation / 10) + academy)
        };
        let mut joined = Vec::new();
        for k in 0..count {
            let nation = if rng.chance(0.9) {
                home_nation
            } else {
                fs_core::ids::NationId::new(rng.below(world.nations.len() as u32))
            };
            let position = YOUTH_POSITIONS[rng.index(YOUTH_POSITIONS.len())];
            let youth_age = 15 + rng.below(2) as u8;
            let mut player = new_player(
                &world.nations,
                &mut rng,
                today,
                PlayerSpec {
                    nation,
                    position,
                    level,
                    age: Age::Youth(youth_age),
                    club: Some(club),
                    squad_number: None,
                },
            );
            player.squad_number = lowest_free_number(world, club, 30 + k as u8);
            joined.push(world.add_player(player));
        }
        if world.is_human_club(club) {
            events.push(Event::YouthIntake {
                club,
                players: joined,
            });
        }
    }
}

/// The lowest squad number at or above `from` that nobody at the club
/// wears.
pub(crate) fn lowest_free_number(world: &World, club: ClubId, from: u8) -> Option<u8> {
    let used: Vec<u8> = world.clubs()[club]
        .squad()
        .iter()
        .filter_map(|&p| world.players().squad_number(p))
        .collect();
    (from..=99).chain(1..from).find(|n| !used.contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retirement_rises_with_age() {
        assert!(retirement_chance(28.0, 500, false, 0).abs() < 1e-12);
        assert!(retirement_chance(34.5, 500, false, 0) > retirement_chance(32.5, 500, false, 0));
        assert!(retirement_chance(33.0, 800, false, 0) < retirement_chance(33.0, 500, false, 0));
        assert!(retirement_chance(30.0, 500, true, 1) > 0.5);
    }
}
