//! Instructions a manager gives during a match.
//!
//! Instructions are keyed to the engine step at which they take effect, so
//! a match replayed with the same input and instructions is identical to
//! the one watched live. That is what lets the world (or a league server)
//! re-run a human's match from a few bytes of input.

use crate::input::Tactic;
use crate::result::Side;
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// A touchline shout. Each lasts fifteen match minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shout {
    /// Lifts the players: a small boost to everything.
    Encourage,
    /// More intensity: a bigger boost, but players tire faster.
    DemandMore,
    /// Fewer rash challenges: fewer fouls and cards.
    CalmDown,
    /// Sit deeper and protect what you have: mentality down a step.
    TightenUp,
    /// Commit more players forward: mentality up a step.
    PushForward,
}

impl Shout {
    /// Every shout.
    pub const ALL: [Shout; 5] = [
        Shout::Encourage,
        Shout::DemandMore,
        Shout::CalmDown,
        Shout::TightenUp,
        Shout::PushForward,
    ];
}

/// A manager's decision during a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instruction {
    /// Replace a player on the pitch with one from the bench.
    Substitute {
        off: PlayerId,
        on: PlayerId,
    },
    /// Change formation and sliders. Players keep their slots.
    ChangeTactic(Tactic),
    Shout(Shout),
}

/// An instruction pinned to the engine step it takes effect at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedInstruction {
    /// Engine step (see `tier1::LiveMatch::step_index`).
    pub step: u32,
    pub side: Side,
    pub instruction: Instruction,
}

/// Why an instruction could not be carried out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstructionError {
    /// The match has finished.
    MatchOver,
    /// The player to come off is not on the pitch.
    NotOnPitch(PlayerId),
    /// The player to come on is not an unused substitute.
    NotOnBench(PlayerId),
    /// All substitutions or substitution windows are used.
    NoSubstitutionsLeft,
}

impl core::fmt::Display for InstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InstructionError::MatchOver => write!(f, "the match is over"),
            InstructionError::NotOnPitch(p) => write!(f, "{p} is not on the pitch"),
            InstructionError::NotOnBench(p) => write!(f, "{p} is not an unused substitute"),
            InstructionError::NoSubstitutionsLeft => write!(f, "no substitutions left"),
        }
    }
}

impl std::error::Error for InstructionError {}
