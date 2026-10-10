//! Fixtures and tables.

use super::refs::*;
use crate::dto;
use fs_world::{ClubId, LeagueId, World};

/// Whether a league has a division below it in the same nation.
fn has_lower(world: &World, league: LeagueId) -> bool {
    let l = &world.leagues()[league];
    world
        .leagues()
        .iter()
        .any(|(_, o)| o.nation == l.nation && o.level == l.level + 1)
}

pub fn table(world: &World, league: LeagueId) -> dto::TableView {
    let l = &world.leagues()[league];
    let swaps = usize::from(l.swaps);
    let lower = has_lower(world, league);
    let (season, rows) = match l.season() {
        Some(s) => (season_label(s.year), s.table()),
        None => (String::new(), Vec::new()),
    };
    let n = rows.len();
    let rows = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let zone = if i == 0 && l.level == 1 {
                Some(dto::TableZone::Champion)
            } else if l.level > 1 && i < swaps {
                Some(dto::TableZone::Promotion)
            } else if lower && i + swaps >= n {
                Some(dto::TableZone::Relegation)
            } else {
                None
            };
            dto::TableRow {
                position: (i + 1) as u8,
                club: club_ref(world, r.club),
                played: r.played,
                won: r.won,
                drawn: r.drawn,
                lost: r.lost,
                scored: r.scored,
                conceded: r.conceded,
                goal_difference: r.goal_difference() as i16,
                points: r.points(),
                form: form(world, r.club, 5),
                zone,
            }
        })
        .collect();
    let mut leagues: Vec<(String, u8, LeagueId)> = world
        .leagues()
        .iter()
        .map(|(id, l)| (world.nations()[l.nation].name.clone(), l.level, id))
        .collect();
    leagues.sort();
    dto::TableView {
        league: league_ref(world, league),
        season,
        rows,
        leagues: leagues
            .into_iter()
            .map(|(_, _, id)| league_ref(world, id))
            .collect(),
    }
}

pub fn fixtures(world: &World, league: LeagueId, club: Option<ClubId>) -> dto::FixturesView {
    let fixtures = world.leagues()[league]
        .season()
        .map(|s| {
            s.fixtures
                .iter()
                .enumerate()
                .filter(|(_, f)| club.is_none_or(|c| f.home == c || f.away == c))
                .map(|(i, _)| fixture(world, league, i))
                .collect()
        })
        .unwrap_or_default();
    dto::FixturesView {
        league: league_ref(world, league),
        club: club.map(|c| club_ref(world, c)),
        fixtures,
    }
}

/// The club's next unplayed league fixture, and its last played one.
pub fn next_and_last(world: &World, club: ClubId) -> (Option<dto::Fixture>, Option<dto::Fixture>) {
    let Some(league) = world.clubs()[club].league() else {
        return (None, None);
    };
    let Some(season) = world.leagues()[league].season() else {
        return (None, None);
    };
    let mine = |f: &&fs_world::Fixture| f.home == club || f.away == club;
    let next = season
        .fixtures
        .iter()
        .enumerate()
        .find(|(_, f)| mine(f) && f.result.is_none())
        .map(|(i, _)| fixture(world, league, i));
    let last = season
        .fixtures
        .iter()
        .enumerate()
        .rfind(|(_, f)| mine(f) && f.result.is_some())
        .map(|(i, _)| fixture(world, league, i));
    (next, last)
}
