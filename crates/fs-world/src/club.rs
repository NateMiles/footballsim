//! Clubs.

use crate::ids::{LeagueId, ManagerId, NationId, PlayerId};
use fs_match::Tactic;
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
    /// The league the club plays in, if any. Maintained by
    /// [`crate::World`].
    pub(crate) league: Option<LeagueId>,
    /// Tactic chosen by a human manager. AI clubs leave this empty.
    pub(crate) tactic: Option<Tactic>,
    /// The AI's current tactic, refreshed monthly. Used when `tactic` is
    /// empty.
    pub(crate) ai_tactic: Option<Tactic>,
    /// Everyone registered with the club, in the order they joined.
    /// Maintained by [`crate::World`] only, so it always agrees with each
    /// player's club.
    pub(crate) squad: Vec<PlayerId>,
}

impl Club {
    /// A club with an empty squad, no manager and no league.
    pub fn new(name: String, short_name: String, nation: NationId, reputation: u16) -> Self {
        Self {
            name,
            short_name,
            nation,
            reputation,
            manager: None,
            league: None,
            tactic: None,
            ai_tactic: None,
            squad: Vec::new(),
        }
    }

    /// Players registered with the club.
    pub fn squad(&self) -> &[PlayerId] {
        &self.squad
    }

    /// The club's league, if any.
    pub fn league(&self) -> Option<LeagueId> {
        self.league
    }

    /// The tactic the club will play: the manager's choice if set,
    /// otherwise the AI's.
    pub fn tactic(&self) -> Option<Tactic> {
        self.tactic.or(self.ai_tactic)
    }
}
