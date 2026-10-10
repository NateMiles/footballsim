//! Nations.

use serde::{Deserialize, Serialize};

/// A footballing nation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nation {
    pub name: String,
    /// Three-letter code.
    pub code: String,
    /// Strength of the domestic game, 0-10000.
    pub reputation: u16,
}
