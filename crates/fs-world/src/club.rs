//! Clubs.

use crate::ids::{ManagerId, NationId, PlayerId};
use serde::{Deserialize, Serialize};

/// A football club.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Club {
    pub name: String,
    pub short_name: String,
    pub nation: NationId,
    /// Standing in the football world, 0-10000.
    pub reputation: u16,
    /// The manager in charge, if any.
    pub manager: Option<ManagerId>,
    /// First-team squad, in the order players joined. Maintained by
    /// [`crate::World`] only, so it always agrees with each player's club.
    pub(crate) squad: Vec<PlayerId>,
}

impl Club {
    /// A club with an empty squad and no manager.
    pub fn new(name: String, short_name: String, nation: NationId, reputation: u16) -> Self {
        Self {
            name,
            short_name,
            nation,
            reputation,
            manager: None,
            squad: Vec::new(),
        }
    }

    /// Players in the squad.
    pub fn squad(&self) -> &[PlayerId] {
        &self.squad
    }
}
