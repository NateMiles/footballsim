//! Managers: the humans and AIs who issue commands.
//!
//! The world can hold any number of human managers. Nothing in the core
//! assumes a single "the player", which keeps online leagues possible
//! (see `docs/ARCHITECTURE.md`, section 11).

use crate::ids::ClubId;
use crate::names::NameRef;
use serde::{Deserialize, Serialize};

/// Who controls a manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Control {
    /// A person playing the game.
    Human,
    /// The game's AI.
    Ai,
}

/// A manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manager {
    pub first_name: NameRef,
    pub last_name: NameRef,
    pub control: Control,
    /// The club this manager runs, if employed.
    pub club: Option<ClubId>,
    /// For a human manager: let the assistant handle contracts, releases
    /// and free-agent signings (as the AI does for its own clubs).
    pub delegate_squad: bool,
}
