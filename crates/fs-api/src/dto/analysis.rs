//! Post-match analysis and season statistics by competition.

use super::common::{ClubRef, PlayerRef, Side};
use super::competition::{CompetitionKind, CompetitionRef, Fixture};
use super::live::TeamStats;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One shot, placed in the shooting side's frame: `band` 0 is their own
/// goal line and 5 the opponent's penalty area, `lane` 0 their left.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShotView {
    pub minute: u8,
    pub side: Side,
    pub player: PlayerRef,
    /// `"shot"`, `"header"`, `"penalty"` or `"free kick"`.
    pub kind: String,
    pub band: u8,
    pub lane: u8,
    pub xg: f32,
    /// `"goal"`, `"saved"`, `"missed"` or `"blocked"`.
    pub outcome: String,
}

/// Cumulative expected goals after a shot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct XgPoint {
    pub minute: u8,
    pub home: f32,
    pub away: f32,
}

/// A player in a pass network: where they were involved on average.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NetworkNode {
    pub player: PlayerRef,
    /// 0-5 from their own goal line.
    pub band: f32,
    /// 0-2 from their left.
    pub lane: f32,
    pub involvements: u16,
}

/// Completed passes between two players.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NetworkLink {
    pub from: u32,
    pub to: u32,
    pub count: u16,
}

/// One side's analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SideAnalysisView {
    pub club: ClubRef,
    /// Actions by zone: six bands of three lanes, in the side's frame.
    pub touches: Vec<Vec<u16>>,
    pub nodes: Vec<NetworkNode>,
    /// The most frequent connections.
    pub links: Vec<NetworkLink>,
}

/// One player's match in numbers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerLine {
    pub player: PlayerRef,
    pub started: bool,
    pub minutes: u32,
    pub goals: u32,
    pub assists: u32,
    pub shots: u32,
    pub xg: f32,
    pub passes: u32,
    pub passes_completed: u32,
    pub key_passes: u32,
    pub dribbles_won: u32,
    pub tackles: u32,
    pub interceptions: u32,
    /// Average for season lines.
    pub rating: f32,
}

/// A match analysed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MatchAnalysisView {
    pub fixture: Fixture,
    pub goals: [u8; 2],
    pub stats: [TeamStats; 2],
    pub shots: Vec<ShotView>,
    pub xg: Vec<XgPoint>,
    /// The home side's share of possession in each 15-minute spell.
    pub possession: Vec<f32>,
    pub sides: [SideAnalysisView; 2],
    pub players: [Vec<PlayerLine>; 2],
}

/// A club's season in one competition (or all of them).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionSeason {
    /// `None` for all competitions together.
    pub competition: Option<CompetitionRef>,
    pub played: u32,
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goals_for: u32,
    pub goals_against: u32,
    pub xg_for: f32,
    pub xg_against: f32,
    pub shots_for: u32,
    pub shots_against: u32,
    /// Mean share, 0-1.
    pub possession: f32,
    /// Players who appeared, most minutes first.
    pub players: Vec<PlayerLine>,
    /// The matches, newest first.
    pub matches: Vec<Fixture>,
}

/// A club's season statistics by competition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStatsView {
    pub club: ClubRef,
    /// All competitions first.
    pub competitions: Vec<CompetitionSeason>,
}

/// Identifies a fixture in a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FixtureKey {
    pub kind: CompetitionKind,
    pub id: u32,
    pub index: u32,
}
