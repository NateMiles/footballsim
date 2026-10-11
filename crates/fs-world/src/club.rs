//! Clubs.

use crate::board::Board;
use crate::finance::{Facilities, Finances};
use crate::ids::{LeagueId, ManagerId, NationId, PlayerId, StaffId};
use crate::lineup::Lineup;
use crate::training::{Familiarity, Individual, Schedule};
use fs_match::{Formation, Instructions, Tactic};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// Lineup chosen by a human manager; automatic selection when empty.
    pub(crate) lineup: Option<Lineup>,
    /// Everyone registered with the club, in the order they joined.
    /// Maintained by [`crate::World`] only, so it always agrees with each
    /// player's club.
    pub(crate) squad: Vec<PlayerId>,
    /// Backroom staff. Maintained by [`crate::World`].
    pub(crate) staff: Vec<StaffId>,
    pub(crate) finances: Finances,
    pub facilities: Facilities,
    pub(crate) board: Board,
    /// The squad's training schedule.
    pub(crate) training: Schedule,
    /// How well the club knows each formation.
    pub(crate) familiarity: Familiarity,
    /// Players with their own training. Maintained by [`crate::World`]:
    /// only current squad members appear.
    pub(crate) individual: BTreeMap<PlayerId, Individual>,
    /// The captain. Maintained by [`crate::World`]: always a squad member.
    pub(crate) captain: Option<PlayerId>,
    /// The manager's roles, set-piece takers and opposition instructions,
    /// with the formation the roles were set for. Opposition instructions
    /// last one match.
    pub(crate) instructions: Option<(Formation, Instructions)>,
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
            lineup: None,
            squad: Vec::new(),
            staff: Vec::new(),
            finances: Finances::default(),
            facilities: Facilities::default(),
            board: Board::default(),
            training: Schedule::default(),
            familiarity: Familiarity::default(),
            individual: BTreeMap::new(),
            captain: None,
            instructions: None,
        }
    }

    /// Players registered with the club.
    pub fn squad(&self) -> &[PlayerId] {
        &self.squad
    }

    /// Backroom staff.
    pub fn staff(&self) -> &[StaffId] {
        &self.staff
    }

    pub fn finances(&self) -> &Finances {
        &self.finances
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    /// The squad's training schedule.
    pub fn training(&self) -> Schedule {
        self.training
    }

    /// How well the club knows each formation.
    pub fn familiarity(&self) -> &Familiarity {
        &self.familiarity
    }

    /// Players' own training.
    pub fn individual_training(&self) -> &BTreeMap<PlayerId, Individual> {
        &self.individual
    }

    /// The manager's instructions and the formation their roles are for.
    pub fn instructions(&self) -> Option<&(Formation, Instructions)> {
        self.instructions.as_ref()
    }

    /// The captain.
    pub fn captain(&self) -> Option<PlayerId> {
        self.captain
    }

    /// The club's league, if any.
    pub fn league(&self) -> Option<LeagueId> {
        self.league
    }

    /// The manager's chosen lineup, if any.
    pub fn lineup(&self) -> Option<&Lineup> {
        self.lineup.as_ref()
    }

    /// The tactic the manager chose, if any (as opposed to the AI's).
    pub fn manager_tactic(&self) -> Option<Tactic> {
        self.tactic
    }

    /// The tactic the club will play: the manager's choice if set,
    /// otherwise the AI's.
    pub fn tactic(&self) -> Option<Tactic> {
        self.tactic.or(self.ai_tactic)
    }
}
