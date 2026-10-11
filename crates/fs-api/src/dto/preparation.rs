//! Getting ready for a match: the opposition, predictions and season
//! odds.

use super::common::{ClubRef, LeagueRef, PlayerRef, Tactic};
use super::competition::{CupRef, Fixture};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the assistant knows about an opponent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OppositionReport {
    pub club: ClubRef,
    /// The match this report is for, if they are next.
    pub fixture: Option<Fixture>,
    pub league: Option<LeagueRef>,
    pub position: Option<u8>,
    /// Their likely tactic.
    pub tactic: Tactic,
    /// How they play, in words.
    pub style: Vec<String>,
    /// Their likely eleven, by formation slot.
    pub likely_xi: Vec<ReportPlayer>,
    /// Last five results in every competition, oldest first.
    pub form: Vec<String>,
    /// This season in every competition.
    pub record: Record,
    pub key_players: Vec<KeyPlayer>,
    /// Regulars who are injured or suspended.
    pub absent: Vec<KeyPlayer>,
    /// Area by area against your team.
    pub comparison: Vec<Comparison>,
    /// Expected goals in the match: theirs, then yours.
    pub expected_goals: Option<[f32; 2]>,
    /// How well your scouts know their players, 0-100.
    pub knowledge: u8,
}

/// A player in a report, as your scouts see them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReportPlayer {
    pub player: PlayerRef,
    /// The position they would play.
    pub position: String,
    /// Estimated ability, 0-100.
    pub ability: u8,
    /// Goals this season, every competition.
    pub goals: u16,
}

/// A player to watch, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct KeyPlayer {
    pub player: PlayerRef,
    pub reason: String,
}

/// Results so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub played: u16,
    pub won: u16,
    pub drawn: u16,
    pub lost: u16,
    pub scored: u16,
    pub conceded: u16,
}

/// One area of the game compared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    /// Such as `"Attack"`.
    pub area: String,
    /// -2 (they are much weaker) to +2 (much stronger).
    pub edge: i8,
    /// Such as `"Their attack is stronger than yours"`.
    pub verdict: String,
}

/// A fixture played many times over.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Prediction {
    pub fixture: Fixture,
    /// The tactic your side used.
    pub tactic: Tactic,
    pub runs: u32,
    /// Chances from the home side's view, 0-1.
    pub home_win: f32,
    pub draw: f32,
    pub away_win: f32,
    pub goals: [f32; 2],
    pub xg: [f32; 2],
    /// The likeliest scores, most likely first.
    pub scores: Vec<ScoreChance>,
}

/// A final score and its chance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScoreChance {
    pub goals: [u8; 2],
    pub chance: f32,
}

/// How a league season might end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionsView {
    pub league: LeagueRef,
    pub season: String,
    /// Seasons simulated.
    pub runs: u32,
    /// Likeliest finish first.
    pub rows: Vec<ProjectionRow>,
    /// Places that lead somewhere, best first.
    pub zones: Vec<ProjectionZone>,
}

/// A run of table positions that leads somewhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionZone {
    /// Such as `"Champions"`, `"Champions Cup"`, `"Relegation"`.
    pub name: String,
    pub from: u8,
    pub to: u8,
    pub cup: Option<CupRef>,
}

/// One club's outlook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionRow {
    pub club: ClubRef,
    /// Position today.
    pub position: u8,
    pub points: u16,
    pub expected_points: f32,
    pub expected_position: f32,
    /// Chance of finishing in each zone, in [`ProjectionsView::zones`]
    /// order, 0-1.
    pub zones: Vec<f32>,
    /// Chance of each final position, first first.
    pub positions: Vec<f32>,
}
