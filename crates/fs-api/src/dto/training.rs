//! Training.

use super::common::{Formation, PlayerRef};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the week's training concentrates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TrainingFocus {
    Balanced,
    Fitness,
    Attacking,
    Defending,
    Tactics,
    Recovery,
}

/// How hard the squad trains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TrainingIntensity {
    Light,
    Normal,
    Hard,
}

/// One player's own training.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum IndividualTraining {
    /// Learn a position, by code such as `"WBL"`.
    Position { position: String },
    /// Work on an attribute, by name such as `"Finishing"`.
    Attribute { attribute: String },
}

/// A choice with its explanation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FocusInfo {
    pub focus: TrainingFocus,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IntensityInfo {
    pub intensity: TrainingIntensity,
    pub name: String,
    pub description: String,
}

/// How well the club knows a formation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FormationFamiliarity {
    pub formation: Formation,
    /// 0-100; 90 and above costs nothing.
    pub familiarity: u8,
    /// The formation the team plays now.
    pub current: bool,
}

/// A squad member's training.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TrainingPlayer {
    pub player: PlayerRef,
    pub age: u8,
    pub individual: Option<IndividualTraining>,
    /// For position training: familiarity with that position, 0-20.
    pub progress: Option<u8>,
    /// 0-100.
    pub fitness: u8,
    pub injured: bool,
}

/// The training screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TrainingView {
    pub focus: TrainingFocus,
    pub intensity: TrainingIntensity,
    pub focuses: Vec<FocusInfo>,
    pub intensities: Vec<IntensityInfo>,
    /// The schedule's effects against balanced, normal training, in
    /// percent: player growth, daily recovery, training injuries.
    pub growth: i16,
    pub recovery: i16,
    pub injury_risk: i16,
    /// Familiarity gained per week in the current formation, 0-100 scale.
    pub drill_per_week: u8,
    pub familiarity: Vec<FormationFamiliarity>,
    pub players: Vec<TrainingPlayer>,
    /// Position codes and attribute names for individual training.
    pub positions: Vec<String>,
    pub attributes: Vec<String>,
}
