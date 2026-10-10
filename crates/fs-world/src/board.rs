//! Boards: what they expect of their manager, and how patient they are.

use serde::{Deserialize, Serialize};

/// Starting confidence, and where it settles.
pub const NEUTRAL_CONFIDENCE: u8 = 60;

/// A club's board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    /// The league position the board expects, or better (1 is first).
    /// Zero when the club has no league season.
    pub target: u8,
    /// Confidence in the manager, 0-100. The manager is sacked when it
    /// falls too low.
    pub confidence: u8,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            target: 0,
            confidence: NEUTRAL_CONFIDENCE,
        }
    }
}
