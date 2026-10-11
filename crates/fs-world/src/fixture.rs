//! Fixtures across competitions: leagues and cups share one way of naming
//! a match, so match days, live matches and the UI treat them alike.

use crate::cup::CupFixture;
use crate::ids::{ClubId, CupId, LeagueId};
use crate::league::Fixture;
use crate::world::World;
use fs_core::GameDate;
use serde::{Deserialize, Serialize};

/// A scheduled match in some competition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FixtureRef {
    /// Index into the league season's fixtures.
    League { league: LeagueId, index: u32 },
    /// Index into the cup edition's fixtures.
    Cup { cup: CupId, index: u32 },
}

impl FixtureRef {
    /// The fixture, if it exists in the current season.
    pub fn get(self, world: &World) -> Option<&Fixture> {
        match self {
            FixtureRef::League { league, index } => world
                .leagues()
                .get(league)?
                .season()?
                .fixtures
                .get(index as usize),
            FixtureRef::Cup { cup, index } => world
                .cups()
                .get(cup)?
                .edition()?
                .fixtures
                .get(index as usize)
                .map(|f| &f.fixture),
        }
    }

    /// The cup fixture, for cup matches.
    pub fn cup_fixture(self, world: &World) -> Option<&CupFixture> {
        match self {
            FixtureRef::League { .. } => None,
            FixtureRef::Cup { cup, index } => world
                .cups()
                .get(cup)?
                .edition()?
                .fixtures
                .get(index as usize),
        }
    }

    /// A unique key for the match's random stream.
    pub fn match_key(self, world: &World) -> u64 {
        match self {
            FixtureRef::League { league, index } => {
                let year = world.leagues()[league].season().map_or(0, |s| s.year);
                ((year as u64) << 40) ^ (u64::from(league.raw()) << 20) ^ u64::from(index)
            }
            FixtureRef::Cup { cup, index } => {
                let year = world.cups()[cup].edition().map_or(0, |e| e.year);
                (1 << 62) ^ ((year as u64) << 40) ^ (u64::from(cup.raw()) << 20) ^ u64::from(index)
            }
        }
    }
}

/// Every unplayed fixture dated today or earlier: leagues in ID order,
/// then cups.
pub fn due(world: &World) -> Vec<FixtureRef> {
    let today = world.date();
    let mut out = Vec::new();
    for (league, l) in world.leagues().iter() {
        if let Some(s) = l.season() {
            let mut i = s.next;
            while i < s.fixtures.len() && s.fixtures[i].date <= today {
                if s.fixtures[i].result.is_none() {
                    out.push(FixtureRef::League {
                        league,
                        index: i as u32,
                    });
                }
                i += 1;
            }
        }
    }
    for (cup, c) in world.cups().iter() {
        if let Some(e) = c.edition() {
            let mut i = e.next;
            while i < e.fixtures.len() && e.fixtures[i].fixture.date <= today {
                if e.fixtures[i].fixture.result.is_none() {
                    out.push(FixtureRef::Cup {
                        cup,
                        index: i as u32,
                    });
                }
                i += 1;
            }
        }
    }
    out
}

/// A club's fixtures this season in every competition, by date.
pub fn of_club(world: &World, club: ClubId) -> Vec<FixtureRef> {
    let mut out: Vec<(GameDate, FixtureRef)> = Vec::new();
    if let Some(league) = world.clubs()[club].league()
        && let Some(s) = world.leagues()[league].season()
    {
        for (i, f) in s.fixtures.iter().enumerate() {
            if f.home == club || f.away == club {
                out.push((
                    f.date,
                    FixtureRef::League {
                        league,
                        index: i as u32,
                    },
                ));
            }
        }
    }
    for (cup, c) in world.cups().iter() {
        let Some(e) = c.edition() else { continue };
        if !e.involves(club) {
            continue;
        }
        for (i, f) in e.fixtures.iter().enumerate() {
            if f.fixture.home == club || f.fixture.away == club {
                out.push((
                    f.fixture.date,
                    FixtureRef::Cup {
                        cup,
                        index: i as u32,
                    },
                ));
            }
        }
    }
    out.sort();
    out.into_iter().map(|(_, r)| r).collect()
}

/// The club's fixture on a date, if any.
pub fn on(world: &World, club: ClubId, date: GameDate) -> Option<FixtureRef> {
    if let Some(league) = world.clubs()[club].league()
        && let Some(s) = world.leagues()[league].season()
    {
        let start = s.fixtures.partition_point(|f| f.date < date);
        for (i, f) in s.fixtures.iter().enumerate().skip(start) {
            if f.date > date {
                break;
            }
            if f.home == club || f.away == club {
                return Some(FixtureRef::League {
                    league,
                    index: i as u32,
                });
            }
        }
    }
    for (cup, c) in world.cups().iter() {
        let Some(e) = c.edition() else { continue };
        for (i, f) in e.fixtures.iter().enumerate() {
            if f.fixture.date == date && (f.fixture.home == club || f.fixture.away == club) {
                return Some(FixtureRef::Cup {
                    cup,
                    index: i as u32,
                });
            }
        }
    }
    None
}

/// The club's next unplayed fixture in any competition.
pub fn next_for(world: &World, club: ClubId) -> Option<FixtureRef> {
    of_club(world, club)
        .into_iter()
        .find(|r| r.get(world).is_some_and(|f| f.result.is_none()))
}
