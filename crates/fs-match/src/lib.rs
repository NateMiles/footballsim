//! Match engines.
//!
//! Two fidelity tiers share one input ([`MatchInput`]) and one output
//! ([`MatchResult`]):
//!
//! - [`tier1`]: event-level simulation. Possession moves through pitch
//!   zones; every pass, dribble, cross and shot is an attribute contest
//!   between specific players. Used for matches the player follows and for
//!   the match predictor.
//! - [`tier2`]: statistical simulation. Team strengths become expected
//!   goals, which are sampled directly. Coefficients are fitted to tier 1
//!   output, so both tiers agree on average. Used for distant leagues.
//!
//! See `docs/MATCH_ENGINE.md` for the design.

pub mod analysis;
pub mod fitted;
pub mod formation;
pub mod input;
pub mod instruction;
pub mod log;
pub mod predict;
pub mod result;
pub mod roles;
pub mod selection;
pub mod shootout;
pub mod skills;
pub mod strength;
pub mod tier1;
pub mod tier2;
pub mod view;

pub use formation::{Formation, Slot};
pub use input::{Decider, MatchInput, PlayerSnapshot, Tactic, TeamSheet};
pub use instruction::{Instruction, InstructionError, Shout, TimedInstruction};
pub use log::{ActionKind, ActionRecord, Outcome, Zone};
pub use result::{GoalKind, MatchEvent, MatchResult, PlayerMatchStats, Side, TeamStats};
pub use roles::{Duty, Instructions, Marking, Role, SlotRole};
pub use selection::pick_team;
pub use shootout::Shootout;
pub use strength::TeamStrength;

/// Which engine to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tier {
    /// Event-level simulation.
    Event,
    /// Statistical simulation.
    Statistical,
}

/// Plays a match with the chosen engine.
pub fn play(input: &MatchInput, tier: Tier) -> MatchResult {
    match tier {
        Tier::Event => tier1::play(input),
        Tier::Statistical => tier2::play(input),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Synthetic teams for unit tests.

    use crate::formation::Formation;
    use crate::input::{MatchInput, PlayerSnapshot, Tactic, TeamSheet};
    use fs_core::Seed;
    use fs_core::attributes::{ATTR_COUNT, Attributes, Hidden, PositionRatings};
    use fs_core::ids::PlayerId;

    /// A 4-4-2 side where every attribute is `level`, natural in their
    /// positions. IDs start at `first_id`.
    pub fn sheet(level: u16, first_id: u32) -> TeamSheet {
        let tactic = Tactic::balanced(Formation::F442);
        let slots = tactic.formation.slots();
        let make = |i: u32, pos| {
            let mut positions = PositionRatings::default();
            positions.set(pos, 20);
            PlayerSnapshot {
                id: PlayerId::new(first_id + i),
                attributes: Attributes([level; ATTR_COUNT]),
                hidden: Hidden::default(),
                positions,
                fitness: 10_000,
                sharpness: 8_000,
                morale: 5_000,
                cohesion: crate::input::DRILLED,
            }
        };
        let starters = (0..11)
            .map(|i| make(i, slots[i as usize].position))
            .collect();
        let bench = (0..7)
            .map(|i| make(11 + i, slots[1 + (i as usize % 10)].position))
            .collect();
        TeamSheet {
            tactic,
            starters,
            bench,
            auto_subs: true,
            instructions: Default::default(),
        }
    }

    pub fn input(home: u16, away: u16, key: u64) -> MatchInput {
        MatchInput::new(sheet(home, 0), sheet(away, 100), Seed(42), key)
    }
}
