//! Players, stored column by column.
//!
//! The daily tick touches a few fields of every player. Storing each field
//! group in its own vector keeps those passes cache-friendly and easy to
//! parallelise. All columns have the same length and are indexed by
//! [`PlayerId`]. Only [`Players::push`] adds rows, so they stay aligned.

use crate::attributes::{Attributes, Hidden, PositionRatings};
use crate::happiness::SquadStatus;
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
    /// Match sharpness. Rises with game time, fades without it.
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

/// How bad an injury is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Minor,
    Moderate,
    Serious,
}

/// A current injury.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Injury {
    pub severity: Severity,
    pub since: GameDate,
    /// First day the player is available again.
    pub until: GameDate,
}

/// A contract (players and staff).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contract {
    /// Weekly wage.
    pub wage: u32,
    /// Last day of the contract (always 30 June).
    pub expires: GameDate,
    /// Paid for each appearance, from the bench too.
    pub appearance_bonus: u32,
    /// Paid for each goal.
    pub goal_bonus: u32,
    /// A fee no club may refuse: an offer that meets it is accepted, and
    /// the player decides.
    pub release_clause: Option<crate::finance::Money>,
}

impl Contract {
    /// A plain contract: a wage and an end date.
    pub const fn new(wage: u32, expires: GameDate) -> Self {
        Self {
            wage,
            expires,
            appearance_bonus: 0,
            goal_bonus: 0,
            release_clause: None,
        }
    }
}

/// Whether a player is still playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Active,
    Retired,
}

/// Totals for the current season.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SeasonStats {
    pub appearances: u16,
    pub starts: u16,
    pub minutes: u32,
    pub goals: u16,
    pub assists: u16,
    pub yellows: u16,
    pub reds: u16,
    pub clean_sheets: u16,
    /// Sum of match ratings, for the average.
    pub rating_sum: f32,
}

impl SeasonStats {
    /// Average match rating, or `None` before the first appearance.
    pub fn average_rating(&self) -> Option<f32> {
        (self.appearances > 0).then(|| self.rating_sum / f32::from(self.appearances))
    }
}

/// One finished season in a player's career.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CareerSeason {
    /// The year the season started.
    pub year: i32,
    pub club: Option<ClubId>,
    /// Ability (best-position role rating, 0-1000) at the end of the
    /// season.
    pub ability: u16,
    pub stats: SeasonStats,
}

/// Everything needed to create a player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInit {
    pub bio: PlayerBio,
    pub club: Option<ClubId>,
    pub attributes: Attributes,
    pub hidden: Hidden,
    pub positions: PositionRatings,
    /// Hidden ceiling for the best-position role rating, 0-1000.
    pub potential: u16,
    pub condition: Condition,
    pub contract: Option<Contract>,
    pub squad_number: Option<u8>,
}

/// All players in the world, as parallel columns.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Players {
    pub(crate) bio: Vec<PlayerBio>,
    pub(crate) club: Vec<Option<ClubId>>,
    pub(crate) attributes: Vec<Attributes>,
    pub(crate) hidden: Vec<Hidden>,
    pub(crate) positions: Vec<PositionRatings>,
    pub(crate) potential: Vec<u16>,
    pub(crate) condition: Vec<Condition>,
    pub(crate) injury: Vec<Option<Injury>>,
    /// Matches still to serve.
    pub(crate) suspension: Vec<u8>,
    pub(crate) contract: Vec<Option<Contract>>,
    pub(crate) squad_number: Vec<Option<u8>>,
    pub(crate) status: Vec<Status>,
    /// Season rollovers spent without a club.
    pub(crate) seasons_unattached: Vec<u8>,
    pub(crate) season: Vec<SeasonStats>,
    pub(crate) career: Vec<Vec<CareerSeason>>,
    /// The role the player expects at their club.
    pub(crate) squad_status: Vec<SquadStatus>,
    /// Recent share of the club's match minutes, 0-10000, as a moving
    /// average over about six matches.
    pub(crate) playing_time: Vec<u16>,
    /// Happiness, -100 to 100, as of the last monthly review.
    pub(crate) happiness: Vec<i8>,
    /// Monthly reviews in a row spent very unhappy.
    pub(crate) unsettled: Vec<u8>,
}

impl Players {
    /// Number of players, retired included.
    pub fn len(&self) -> usize {
        self.bio.len()
    }

    /// Whether there are no players.
    pub fn is_empty(&self) -> bool {
        self.bio.is_empty()
    }

    /// Every player ID in ascending order, retired included.
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
        self.potential.push(p.potential);
        self.condition.push(p.condition);
        self.injury.push(None);
        self.suspension.push(0);
        self.contract.push(p.contract);
        self.squad_number.push(p.squad_number);
        self.status.push(Status::Active);
        self.seasons_unattached.push(0);
        self.season.push(SeasonStats::default());
        self.career.push(Vec::new());
        self.squad_status.push(SquadStatus::Prospect);
        self.playing_time.push(SquadStatus::Prospect.expected());
        self.happiness.push(0);
        self.unsettled.push(0);
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

    /// Hidden ability ceiling, 0-1000.
    pub fn potential(&self, id: PlayerId) -> u16 {
        self.potential[id.index()]
    }

    /// Ability: role rating in the player's best position, 0-1000.
    pub fn ability(&self, id: PlayerId) -> u16 {
        let i = id.index();
        self.positions[i].best().role_rating(&self.attributes[i])
    }

    /// Current condition.
    pub fn condition(&self, id: PlayerId) -> &Condition {
        &self.condition[id.index()]
    }

    /// Current injury, if any.
    pub fn injury(&self, id: PlayerId) -> Option<&Injury> {
        self.injury[id.index()].as_ref()
    }

    /// Matches of suspension left to serve.
    pub fn suspension(&self, id: PlayerId) -> u8 {
        self.suspension[id.index()]
    }

    /// Whether the player can be picked on `date`.
    pub fn available(&self, id: PlayerId, date: GameDate) -> bool {
        let i = id.index();
        self.status[i] == Status::Active
            && self.suspension[i] == 0
            && self.injury[i].is_none_or(|inj| inj.until <= date)
    }

    /// Current contract, if any.
    pub fn contract(&self, id: PlayerId) -> Option<&Contract> {
        self.contract[id.index()].as_ref()
    }

    /// Squad number, if assigned.
    pub fn squad_number(&self, id: PlayerId) -> Option<u8> {
        self.squad_number[id.index()]
    }

    /// Active or retired.
    pub fn status(&self, id: PlayerId) -> Status {
        self.status[id.index()]
    }

    /// Totals for the current season.
    pub fn season_stats(&self, id: PlayerId) -> &SeasonStats {
        &self.season[id.index()]
    }

    /// Finished seasons, oldest first.
    pub fn career(&self, id: PlayerId) -> &[CareerSeason] {
        &self.career[id.index()]
    }

    /// The role the player expects at their club.
    pub fn squad_status(&self, id: PlayerId) -> SquadStatus {
        self.squad_status[id.index()]
    }

    /// Recent share of the club's match minutes, 0-10000.
    pub fn playing_time(&self, id: PlayerId) -> u16 {
        self.playing_time[id.index()]
    }

    /// Happiness, -100 to 100, as of the last monthly review (or the last
    /// conversation with the manager).
    pub fn happiness(&self, id: PlayerId) -> i8 {
        self.happiness[id.index()]
    }

    /// Checks that every column has the same length.
    pub fn columns_aligned(&self) -> bool {
        let n = self.bio.len();
        [
            self.club.len(),
            self.attributes.len(),
            self.hidden.len(),
            self.positions.len(),
            self.potential.len(),
            self.condition.len(),
            self.injury.len(),
            self.suspension.len(),
            self.contract.len(),
            self.squad_number.len(),
            self.status.len(),
            self.seasons_unattached.len(),
            self.season.len(),
            self.career.len(),
            self.squad_status.len(),
            self.playing_time.len(),
            self.happiness.len(),
            self.unsettled.len(),
        ]
        .iter()
        .all(|&l| l == n)
    }
}
