//! Level of detail: how closely the world simulates each club.
//!
//! Clubs that human managers play against get every detail every time;
//! clubs far from any human are updated less often or with cheaper
//! models that are calibrated to give the same results on average (the
//! tier 2 match engine is fitted to tier 1). Nothing assumes one human:
//! a club matters if it matters to any of them.
//!
//! | | High | Medium | Low |
//! | --- | --- | --- | --- |
//! | Who | human clubs and their league | the rest of their nations, and their continental rivals | everyone else |
//! | League matches | tier 1 | tier 2 | tier 2 |
//! | AI tactic review | every 30 days | every 60 days | every 90 days |

use crate::cup::CupScope;
use crate::ids::ClubId;
use crate::world::World;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// How much detail a club gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Importance {
    Low,
    Medium,
    High,
}

impl Importance {
    /// Days between reviews of an AI club's tactic.
    pub const fn tactic_period(self) -> u32 {
        match self {
            Importance::High => 30,
            Importance::Medium => 60,
            Importance::Low => 90,
        }
    }
}

/// Every club's importance, indexed by club.
pub fn of_clubs(world: &World) -> Vec<Importance> {
    let mut out = vec![Importance::Low; world.clubs().len()];
    let humans: Vec<ClubId> = world
        .clubs()
        .ids()
        .filter(|&c| world.is_human_club(c))
        .collect();
    if humans.is_empty() {
        return out;
    }
    let nations: BTreeSet<_> = humans.iter().map(|&c| world.clubs()[c].nation).collect();
    for (id, club) in world.clubs().iter() {
        if nations.contains(&club.nation) {
            out[id.index()] = Importance::Medium;
        }
    }
    // Clubs in the same continental cups as a human club this season.
    for (_, cup) in world.cups().iter() {
        if !matches!(cup.scope, CupScope::Continent(_)) {
            continue;
        }
        let Some(e) = cup.edition() else { continue };
        if humans.iter().any(|&h| e.involves(h)) {
            for &c in e.entrants.iter().flatten() {
                out[c.index()] = out[c.index()].max(Importance::Medium);
            }
        }
    }
    for &h in &humans {
        out[h.index()] = Importance::High;
        if let Some(l) = world.clubs()[h].league() {
            for &c in world.leagues()[l].clubs() {
                out[c.index()] = Importance::High;
            }
        }
    }
    out
}

/// Whether a club reviews its tactic today: every club once per period,
/// spread over the days so no day carries them all.
pub fn reviews_tactic_today(world: &World, club: ClubId, importance: Importance) -> bool {
    let period = importance.tactic_period();
    (world.date().days() + club.raw().wrapping_mul(7)) % period == 0
}
