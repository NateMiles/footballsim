//! Match days that human managers watch live.
//!
//! When a manager who watches matches has a fixture today, the day stops
//! after the morning's calendar business: no match is played until every
//! such manager has submitted the instructions they gave while watching
//! (or someone advances time, which plays the rest without them). Then
//! all of the day's fixtures are played together, the watched ones with
//! their instructions, so the result is the one the managers saw.

use crate::fixture::{self, FixtureRef};
use crate::ids::{ClubId, ManagerId};
use crate::world::World;
use fs_match::TimedInstruction;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A match a human manager can watch today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchedFixture {
    pub manager: ManagerId,
    pub club: ClubId,
    pub fixture: FixtureRef,
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
    let mut out = Vec::new();
    for r in fixture::due(world) {
        let f = r.get(world).expect("due fixture exists");
        for club in [f.home, f.away] {
            if let Some(manager) = world.clubs()[club].manager
                && world.watched_live(club)
            {
                out.push(WatchedFixture {
                    manager,
                    club,
                    fixture: r,
                });
            }
        }
    }
    out
}

/// The fixture `club` plays today, if any.
pub(crate) fn fixture_today(world: &World, club: ClubId) -> Option<FixtureRef> {
    fixture::on(world, club, world.date())
}
