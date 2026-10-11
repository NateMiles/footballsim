//! Squad, player and tactics screens.

use super::common::{ClubRef, Date, Formation, Id, SeasonStats, Tactic};
use super::market::{Listing, Loan, Offer};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A club's squad list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SquadView {
    pub club: ClubRef,
    pub players: Vec<SquadPlayer>,
    /// Trophies won while the world has been running.
    pub honours: Vec<super::competition::Honour>,
}

/// Where a player stands in the current selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Selection {
    Starter,
    Bench,
    Reserve,
}

/// One row of the squad list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SquadPlayer {
    pub id: Id,
    pub name: String,
    pub number: Option<u8>,
    pub age: u8,
    /// Nation code, such as `"ENG"`.
    pub nation: String,
    /// Best position code.
    pub position: String,
    /// Every position they can play well, best first.
    pub positions: Vec<String>,
    /// Current ability, 0-100.
    pub ability: u8,
    /// Potential ability, 0-100.
    pub potential: u8,
    /// Percentages.
    pub fitness: u8,
    pub sharpness: u8,
    pub morale: u8,
    pub injury: Option<Injury>,
    /// Matches still to serve.
    pub suspension: u8,
    pub contract: Option<Contract>,
    pub season: SeasonStats,
    pub selection: Selection,
    /// Under 18: an academy player.
    pub academy: bool,
}

/// An injury.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Injury {
    /// `"minor"`, `"moderate"`, `"major"` or `"serious"`.
    pub severity: String,
    pub until: Date,
}

/// A contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Contract {
    /// Per week.
    pub wage: u32,
    pub expires: Date,
}

/// Everything about one player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub id: Id,
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub age: u8,
    pub birth_date: Date,
    pub nation: String,
    pub club: Option<ClubRef>,
    pub number: Option<u8>,
    /// `"left"`, `"right"` or `"both"`.
    pub foot: String,
    pub height_cm: u8,
    pub weight_kg: u8,
    pub position: String,
    /// Familiarity with every position, 0-20, in [`PositionInfo`] order.
    pub position_ratings: Vec<u8>,
    pub attributes: Vec<AttributeGroup>,
    /// What the viewer's club knows about the player, 0-100. At 100 the
    /// numbers are exact; below, they are the scouts' estimates.
    pub knowledge: u8,
    /// Estimated ability and potential, 0-100, with ranges.
    pub ability: u8,
    pub ability_low: u8,
    pub ability_high: u8,
    pub potential: u8,
    pub potential_low: u8,
    pub potential_high: u8,
    /// Estimated market value.
    pub value: i64,
    /// What the club would sell for, if it is an AI club and would sell.
    pub asking_price: Option<i64>,
    pub listing: Option<Listing>,
    pub loan: Option<Loan>,
    /// Whether the viewer's scouts are watching the player.
    pub scouting: bool,
    /// The viewer's open offer for the player.
    pub offer: Option<Offer>,
    pub fitness: u8,
    pub sharpness: u8,
    pub morale: u8,
    pub injury: Option<Injury>,
    pub suspension: u8,
    pub contract: Option<Contract>,
    /// What they would ask for in a new contract, per week.
    pub wage_demand: u32,
    pub season: SeasonStats,
    pub career: Vec<CareerRow>,
    pub retired: bool,
    /// Whether the viewer manages this player.
    pub own: bool,
}

/// A group of attributes, such as technical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttributeGroup {
    pub name: String,
    pub attributes: Vec<AttributeValue>,
}

/// One attribute on the 1-20 scale, with the range the viewer's
/// scouts give it (equal to the value when known exactly).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttributeValue {
    pub name: String,
    pub value: u8,
    pub low: u8,
    pub high: u8,
}

/// One finished season of a career.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CareerRow {
    /// Such as `"2026/27"`.
    pub season: String,
    pub club: Option<ClubRef>,
    pub ability: u8,
    pub stats: SeasonStats,
}

/// A position: its code and full name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PositionInfo {
    pub code: String,
    pub name: String,
}

/// A formation's shape, for drawing it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FormationInfo {
    pub formation: Formation,
    /// Eleven slots; slot 0 is the goalkeeper.
    pub slots: Vec<SlotInfo>,
}

/// A slot in a formation. `x` runs 0 (own goal) to 1 (opponent's goal),
/// `y` 0 (left touchline) to 1 (right).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SlotInfo {
    /// Index into [`PositionInfo`] order.
    pub position: u8,
    pub x: f32,
    pub y: f32,
}

/// A tactic slider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SliderInfo {
    /// Field name in [`Tactic`].
    pub key: String,
    pub name: String,
    /// What -2 and +2 mean.
    pub low: String,
    pub high: String,
}

/// The tactics and team selection screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TacticsView {
    pub club: ClubRef,
    pub tactic: Tactic,
    /// True when the manager chose the tactic, false when the assistant
    /// picks it.
    pub own_tactic: bool,
    /// True when the manager picked the lineup.
    pub own_lineup: bool,
    /// The team that would play today: player per formation slot.
    pub starters: Vec<Id>,
    pub bench: Vec<Id>,
    pub players: Vec<SelectablePlayer>,
    pub formations: Vec<FormationInfo>,
    pub positions: Vec<PositionInfo>,
    pub sliders: Vec<SliderInfo>,
    pub bench_size: u8,
}

/// A player as the team selection screen needs them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SelectablePlayer {
    pub id: Id,
    pub name: String,
    pub short_name: String,
    pub number: Option<u8>,
    pub position: String,
    pub available: bool,
    pub fitness: u8,
    /// How well they would play each position today, 0-100, in
    /// [`PositionInfo`] order.
    pub suitability: Vec<u8>,
}

/// A chosen lineup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Lineup {
    /// Eleven players by formation slot.
    pub starters: Vec<Id>,
    pub bench: Vec<Id>,
}
