//! Starting, saving and moving through a game.

use super::common::{ClubRef, Date, Id, LeagueRef, Side};
use super::competition::Fixture;
use super::management::BoardView;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How big a world to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorldSize {
    /// One league of 20 clubs.
    Tiny,
    /// About 120 leagues.
    Small,
    /// About 300 leagues.
    Medium,
}

/// A club a new manager can take over.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClubChoice {
    pub club: ClubRef,
    pub league: LeagueRef,
    /// 1 (minnows) to 5 (giants), by reputation within the world.
    pub stars: u8,
    /// Average ability of the best eleven, 0-100.
    pub squad_ability: u8,
}

/// The clubs on offer in a new world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClubChoices {
    pub clubs: Vec<ClubChoice>,
}

/// The manager's home screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub date: Date,
    /// Such as `"Saturday"`.
    pub weekday: String,
    /// Such as `"2026/27"`.
    pub season: String,
    pub manager: ManagerInfo,
    /// The viewer's club; none while out of work.
    pub club: Option<ClubRef>,
    pub league: Option<LeagueRef>,
    pub position: Option<u8>,
    pub next_fixture: Option<Fixture>,
    pub last_fixture: Option<Fixture>,
    /// Last five league results, oldest first: `"W"`, `"D"` or `"L"`.
    pub form: Vec<String>,
    pub unread: u32,
    /// Today's match, waiting to be watched.
    pub match_day: Option<MatchDay>,
    pub watch_matches: bool,
    pub delegate_squad: bool,
    pub board: Option<BoardView>,
    pub balance: Option<i64>,
    pub transfer_budget: Option<i64>,
    pub window_open: bool,
    /// The club's cups this season.
    pub cups: Vec<super::competition::CupStatus>,
}

/// The viewer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ManagerInfo {
    pub id: Id,
    pub name: String,
}

/// A match waiting to be watched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MatchDay {
    pub fixture: Fixture,
    pub side: Side,
}

/// Why [`crate::Request::Continue`] stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    /// A match to watch.
    MatchDay,
    /// New messages.
    Messages,
    /// Nothing happened for a week.
    Week,
}

/// The result of moving time on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Continued {
    pub date: Date,
    pub stopped: StopReason,
    pub new_messages: u32,
    pub match_day: Option<MatchDay>,
}

/// A performance test world, after generating it or running it on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkStatus {
    /// Whether a test world is loaded.
    pub active: bool,
    pub players: u32,
    pub clubs: u32,
    pub leagues: u32,
    pub cups: u32,
    pub date: Date,
    /// Matches played so far in all competitions.
    pub matches: u32,
    /// Memory the core uses, where the platform tells.
    #[ts(type = "number | null")]
    pub memory_bytes: Option<u64>,
    /// Threads the simulation can use (one in a browser).
    pub threads: u32,
}

/// A saved game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SaveGame {
    /// The save itself, base64.
    pub data: String,
    /// For listing saves without loading them.
    pub summary: SaveSummary,
}

/// What a save slot shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SaveSummary {
    pub date: Date,
    pub manager: String,
    pub club: String,
}
