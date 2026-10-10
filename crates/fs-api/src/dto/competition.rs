//! Fixtures and tables.

use super::common::{ClubRef, Date, Id, LeagueRef, PlayerRef, Side};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A league fixture, played or not.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Fixture {
    pub league: Id,
    /// Index into the league season's fixtures.
    pub index: u32,
    pub date: Date,
    pub home: ClubRef,
    pub away: ClubRef,
    pub result: Option<FixtureResult>,
}

/// A played fixture's result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FixtureResult {
    pub goals: [u8; 2],
    pub xg: [f32; 2],
    pub scorers: Vec<Scorer>,
}

/// A goal in a result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Scorer {
    pub side: Side,
    pub player: PlayerRef,
}

/// Fixtures of a league, or of one club in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FixturesView {
    pub league: LeagueRef,
    pub club: Option<ClubRef>,
    pub fixtures: Vec<Fixture>,
}

/// Where a table position leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TableZone {
    Champion,
    Promotion,
    Relegation,
}

/// A league table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TableView {
    pub league: LeagueRef,
    /// Such as `"2026/27"`.
    pub season: String,
    pub rows: Vec<TableRow>,
    /// Every league, for switching.
    pub leagues: Vec<LeagueRef>,
}

/// A table row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub position: u8,
    pub club: ClubRef,
    pub played: u16,
    pub won: u16,
    pub drawn: u16,
    pub lost: u16,
    pub scored: u16,
    pub conceded: u16,
    pub goal_difference: i16,
    pub points: u16,
    /// Last five results, oldest first: `"W"`, `"D"` or `"L"`.
    pub form: Vec<String>,
    pub zone: Option<TableZone>,
}
