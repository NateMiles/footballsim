//! Players, stored column by column.
//!
//! The daily tick touches a few fields of every player. Storing each field
//! group in its own vector keeps those passes cache-friendly and easy to
//! parallelise. All columns have the same length and are indexed by
//! [`PlayerId`]. Only [`Players::push`] adds rows, so they stay aligned.

use crate::attributes::{Attributes, Hidden, PositionRatings};
use crate::ids::{ClubId, NationId, PlayerId};
use crate::names::NameRef;
use fs_core::GameDate;
use serde::{Deserialize, Serialize};

/// Preferred foot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Foot {
    Left,
    Right,
    Both,
}

/// Rarely-changing biographical data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerBio {
    pub first_name: NameRef,
    pub last_name: NameRef,
    pub birth_date: GameDate,
    pub nation: NationId,
    pub foot: Foot,
    pub height_cm: u8,
    pub weight_kg: u8,
}

/// Day-to-day physical and mental state. Values are fixed-point, 0-10000,
/// so the daily tick uses integer maths only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    /// Physical freshness. 10000 is fully fit.
    pub fitness: u16,
    /// Match sharpness. Drops without game time.
    pub sharpness: u16,
    /// Accumulated load. High fatigue raises injury risk.
    pub fatigue: u16,
    /// Morale. 5000 is neutral.
    pub morale: u16,
}

/// Upper bound of [`Condition`] values.
pub const CONDITION_MAX: u16 = 10_000;

impl Default for Condition {
    fn default() -> Self {
        Self {
            fitness: CONDITION_MAX,
            sharpness: CONDITION_MAX / 2,
            fatigue: 0,
            morale: CONDITION_MAX / 2,
        }
    }
}

/// Everything needed to create a player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInit {
    pub bio: PlayerBio,
    pub club: Option<ClubId>,
    pub attributes: Attributes,
    pub hidden: Hidden,
    pub positions: PositionRatings,
    pub condition: Condition,
    pub squad_number: Option<u8>,
}

/// All players in the world, as parallel columns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Players {
    pub(crate) bio: Vec<PlayerBio>,
    pub(crate) club: Vec<Option<ClubId>>,
    pub(crate) attributes: Vec<Attributes>,
    pub(crate) hidden: Vec<Hidden>,
    pub(crate) positions: Vec<PositionRatings>,
    pub(crate) condition: Vec<Condition>,
    pub(crate) squad_number: Vec<Option<u8>>,
}

impl Players {
    /// Number of players.
    pub fn len(&self) -> usize {
        self.bio.len()
    }

    /// Whether there are no players.
    pub fn is_empty(&self) -> bool {
        self.bio.is_empty()
    }

    /// Every player ID in ascending order.
    pub fn ids(&self) -> impl ExactSizeIterator<Item = PlayerId> + use<> {
        (0..self.len() as u32).map(PlayerId::new)
    }

    /// Whether `id` refers to an existing player.
    pub fn contains(&self, id: PlayerId) -> bool {
        id.index() < self.len()
    }

    /// Adds a player. Use [`crate::World::add_player`] so club squads stay
    /// in sync.
    pub(crate) fn push(&mut self, p: PlayerInit) -> PlayerId {
        let id = PlayerId::new(u32::try_from(self.len()).expect("too many players"));
        self.bio.push(p.bio);
        self.club.push(p.club);
        self.attributes.push(p.attributes);
        self.hidden.push(p.hidden);
        self.positions.push(p.positions);
        self.condition.push(p.condition);
        self.squad_number.push(p.squad_number);
        id
    }

    /// Biography.
    pub fn bio(&self, id: PlayerId) -> &PlayerBio {
        &self.bio[id.index()]
    }

    /// Current club, if any.
    pub fn club(&self, id: PlayerId) -> Option<ClubId> {
        self.club[id.index()]
    }

    /// True attributes.
    pub fn attributes(&self, id: PlayerId) -> &Attributes {
        &self.attributes[id.index()]
    }

    /// True hidden traits.
    pub fn hidden(&self, id: PlayerId) -> &Hidden {
        &self.hidden[id.index()]
    }

    /// Position familiarity.
    pub fn positions(&self, id: PlayerId) -> &PositionRatings {
        &self.positions[id.index()]
    }

    /// Current condition.
    pub fn condition(&self, id: PlayerId) -> &Condition {
        &self.condition[id.index()]
    }

    /// Squad number, if assigned.
    pub fn squad_number(&self, id: PlayerId) -> Option<u8> {
        self.squad_number[id.index()]
    }

    /// Checks that every column has the same length.
    pub fn columns_aligned(&self) -> bool {
        let n = self.bio.len();
        [
            self.club.len(),
            self.attributes.len(),
            self.hidden.len(),
            self.positions.len(),
            self.condition.len(),
            self.squad_number.len(),
        ]
        .iter()
        .all(|&l| l == n)
    }
}
