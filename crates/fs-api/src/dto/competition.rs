//! Fixtures, tables, cups and competitions.

use super::common::{ClubRef, Date, Id, LeagueRef, PlayerRef, Side};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// League or cup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CompetitionKind {
    League,
    Cup,
}

/// A competition as fixtures refer to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionRef {
    pub kind: CompetitionKind,
    /// A league or cup ID, by `kind`.
    pub id: Id,
    pub name: String,
    pub short_name: String,
}

/// A fixture in any competition, played or not.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Fixture {
    pub competition: CompetitionRef,
    /// Index into the competition's fixtures this season.
    pub index: u32,
    pub date: Date,
    pub home: ClubRef,
    pub away: ClubRef,
    pub result: Option<FixtureResult>,
    /// Cup round and leg, such as `"Semi-finals, 2nd leg"` or
    /// `"Group stage, Group B"`.
    pub round: Option<String>,
    /// Played at a neutral ground.
    pub neutral: bool,
}

/// A played fixture's result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FixtureResult {
    /// Including extra time.
    pub goals: [u8; 2],
    pub xg: [f32; 2],
    pub scorers: Vec<Scorer>,
    pub extra_time: bool,
    /// Penalties scored in a shootout, home then away.
    pub penalties: Option<[u8; 2]>,
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
    /// Through to the next stage of a cup.
    Qualify,
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

/// A club's matches this season in every competition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    pub club: ClubRef,
    pub fixtures: Vec<Fixture>,
}

/// A cup as other views refer to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CupRef {
    pub id: Id,
    pub name: String,
    pub short_name: String,
    /// The nation, or the continent for continental cups.
    pub region: String,
    pub continental: bool,
}

/// A cup this season, stage by stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CupView {
    pub cup: CupRef,
    /// Such as `"2026/27"`.
    pub season: String,
    /// Stages drawn so far, first first.
    pub stages: Vec<CupStage>,
    /// Stages still to be drawn.
    pub upcoming: Vec<String>,
    pub winner: Option<ClubRef>,
    /// Past winners, most recent first.
    pub history: Vec<CupWinner>,
    /// The other cups the viewer can switch to.
    pub cups: Vec<CupRef>,
}

/// One stage of a cup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CupStage {
    pub name: String,
    /// Knockout ties (empty for a group stage).
    pub ties: Vec<Tie>,
    /// Groups (empty for a knockout round).
    pub groups: Vec<GroupView>,
    /// Clubs through without playing.
    pub byes: Vec<ClubRef>,
}

/// A knockout tie.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Tie {
    /// Home side of the first leg first.
    pub clubs: [ClubRef; 2],
    pub legs: Vec<Fixture>,
    /// Goals over both legs, in `clubs` order, once a two-legged tie has
    /// started.
    pub aggregate: Option<[u8; 2]>,
    pub winner: Option<Id>,
}

/// A group's table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GroupView {
    /// Such as `"Group A"`.
    pub name: String,
    pub rows: Vec<TableRow>,
    pub fixtures: Vec<Fixture>,
}

/// A season's winner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CupWinner {
    pub season: String,
    pub winner: Option<ClubRef>,
    pub runner_up: Option<ClubRef>,
}

/// How the viewer's club is doing in a cup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CupStatus {
    pub cup: CupRef,
    /// Such as `"Quarter-finals"`, `"Out in the Second round"` or
    /// `"Winners"`.
    pub status: String,
    /// Still in it (or won it).
    pub alive: bool,
}

/// Every competition in the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CompetitionsView {
    pub nations: Vec<NationCompetitions>,
    pub continents: Vec<ContinentView>,
}

/// One nation's leagues and cups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NationCompetitions {
    pub nation: String,
    pub code: String,
    pub leagues: Vec<LeagueRef>,
    pub cups: Vec<CupRef>,
}

/// A continent: its cups and its nations' coefficients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContinentView {
    pub name: String,
    pub cups: Vec<CupRef>,
    /// Best first.
    pub ranking: Vec<CoefficientRow>,
}

/// A nation's coefficient.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CoefficientRow {
    pub rank: u16,
    pub nation: String,
    pub code: String,
    /// The sum of the recent seasons.
    pub points: f32,
    /// Points per season, oldest first.
    pub seasons: Vec<f32>,
}

/// A trophy a club has won, and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Honour {
    pub competition: String,
    pub seasons: Vec<String>,
}
