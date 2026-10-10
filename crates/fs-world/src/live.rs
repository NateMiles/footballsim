//! Match days that human managers watch live.
//!
//! When a manager who watches matches has a fixture today, the day stops
//! after the morning's calendar business: no match is played until every
//! such manager has submitted the instructions they gave while watching
//! (or someone advances time, which plays the rest without them). Then
//! all of the day's fixtures are played together, the watched ones with
//! their instructions, so the result is the one the managers saw.

use crate::ids::{ClubId, LeagueId, ManagerId};
use crate::world::World;
use fs_match::TimedInstruction;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A match a human manager can watch today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchedFixture {
    pub manager: ManagerId,
    pub club: ClubId,
    pub league: LeagueId,
    /// Index into the league season's fixtures.
    pub index: usize,
}

/// A match day waiting for human managers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PendingMatchDay {
    /// Matches still to be watched.
    pub awaiting: Vec<WatchedFixture>,
    /// Instructions submitted so far, by club.
    pub instructions: BTreeMap<ClubId, Vec<TimedInstruction>>,
}

/// Today's fixtures involving managers who watch matches.
pub(crate) fn watched_fixtures(world: &World) -> Vec<WatchedFixture> {
    let today = world.date();
    let mut out = Vec::new();
    for (league, l) in world.leagues().iter() {
        let Some(season) = l.season() else { continue };
        for (index, f) in season.fixtures.iter().enumerate().skip(season.next) {
            if f.date > today {
                break;
            }
            for club in [f.home, f.away] {
                if let Some(manager) = world.clubs()[club].manager
                    && world.watched_live(club)
                {
                    out.push(WatchedFixture {
                        manager,
                        club,
                        league,
                        index,
                    });
                }
            }
        }
    }
    out
}

/// The league fixture `club` plays today, if any.
pub(crate) fn fixture_today(world: &World, club: ClubId) -> Option<(LeagueId, usize)> {
    let league = world.clubs()[club].league()?;
    let season = world.leagues()[league].season()?;
    let today = world.date();
    season
        .fixtures
        .iter()
        .enumerate()
        .skip(season.next)
        .take_while(|(_, f)| f.date <= today)
        .find(|(_, f)| f.date == today && (f.home == club || f.away == club))
        .map(|(i, _)| (league, i))
}
