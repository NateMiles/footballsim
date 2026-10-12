//! Full records of the matches human-managed clubs play: the result,
//! team and player statistics, and the post-match analysis from the
//! action log. Kept for about a year, for the analysis screens and season
//! statistics by competition.

use crate::fixture::FixtureRef;
use crate::ids::ClubId;
use crate::world::World;
use fs_core::GameDate;
use fs_match::analysis::MatchAnalysis;
use fs_match::{PlayerMatchStats, TeamStats};
use serde::{Deserialize, Serialize};

/// One match, in full.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchRecord {
    pub fixture: FixtureRef,
    pub date: GameDate,
    pub home: ClubId,
    pub away: ClubId,
    pub goals: [u8; 2],
    /// Home then away.
    pub stats: [TeamStats; 2],
    pub players: [Vec<PlayerMatchStats>; 2],
    pub analysis: MatchAnalysis,
}

impl MatchRecord {
    /// The club's side (0 home, 1 away), if it played.
    pub fn side_of(&self, club: ClubId) -> Option<usize> {
        if self.home == club {
            Some(0)
        } else if self.away == club {
            Some(1)
        } else {
            None
        }
    }
}

/// Days records are kept.
const KEEP_DAYS: i64 = 400;

/// Records of one club's matches since `from`, oldest first.
pub fn of_club(world: &World, club: ClubId, from: GameDate) -> Vec<&MatchRecord> {
    let mut v: Vec<&MatchRecord> = world
        .records
        .values()
        .filter(|r| r.date >= from && r.side_of(club).is_some())
        .collect();
    v.sort_by_key(|r| (r.date, r.fixture));
    v
}

/// Drops records over a year old (at the rollover).
pub(crate) fn prune(world: &mut World) {
    let today = world.date();
    world
        .records
        .retain(|_, r| today.days_since(r.date) <= KEEP_DAYS);
}
