//! Players as people: moods, squad status, talks and promises.

use super::club::PlayerView;
use super::common::{Date, PlayerRef};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The role a player expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SquadStatus {
    Key,
    Regular,
    Rotation,
    Backup,
    Prospect,
}

/// A squad status with its name and the playing time it expects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StatusInfo {
    pub status: SquadStatus,
    pub name: String,
    /// Expected share of the club's minutes, percent.
    pub expected: u8,
}

/// Something a manager can say to a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Talk {
    Praise,
    Criticise,
    Reassure,
    PromiseFootball,
    PromiseContract,
    Ask,
    Ambitions,
    Motivate,
    Discipline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TalkInfo {
    pub talk: Talk,
    pub name: String,
    pub description: String,
    /// `"listen"`, `"encourage"`, `"challenge"` or `"promise"`.
    pub group: String,
    /// What the manager says.
    pub line: String,
    /// Questions can be asked any time; the rest once a week.
    pub listening: bool,
    /// Whether it can be said to this player today.
    pub available: bool,
}

/// How a player took a talk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Reaction {
    Delighted,
    Pleased,
    Determined,
    Indifferent,
    Sceptical,
    Annoyed,
    Angry,
}

/// A player's character, as far as the viewer knows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Personality {
    pub name: String,
    pub description: String,
}

/// One reason for a player's mood.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MoodReason {
    pub text: String,
    /// Points of happiness, positive or negative.
    pub effect: i16,
}

/// How a player feels, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Mood {
    /// -100 to 100.
    pub happiness: i8,
    /// Such as `"Happy"` or `"Very unhappy"`.
    pub label: String,
    /// Largest first.
    pub reasons: Vec<MoodReason>,
}

/// A promise the manager has to keep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PromiseView {
    pub player: PlayerRef,
    /// `"football"` or `"contract"`.
    pub kind: String,
    pub text: String,
    pub made: Date,
    pub due: Date,
    /// Whether it would count as kept if it fell due today.
    pub on_track: bool,
}

/// A transfer request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequestView {
    pub date: Date,
    /// Whether the club agreed to sell.
    pub accepted: bool,
}

/// A player's place in the dressing room, for their own manager.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPeople {
    pub status: SquadStatus,
    /// Recent share of the club's minutes, percent.
    pub playing_time: u8,
    /// What the status expects, percent.
    pub expected: u8,
    pub mood: Mood,
    pub captain: bool,
    pub influential: bool,
    pub request: Option<RequestView>,
    pub promises: Vec<PromiseView>,
    pub last_talk: Option<Date>,
    /// Whether a talk is possible today.
    pub can_talk: bool,
    /// What can be said to them.
    pub talks: Vec<TalkInfo>,
}

/// One row of the dressing-room screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DynamicsRow {
    pub player: PlayerRef,
    pub age: u8,
    pub personality: String,
    pub status: SquadStatus,
    pub playing_time: u8,
    pub expected: u8,
    pub happiness: i8,
    pub label: String,
    /// The biggest reason, if any.
    pub reason: Option<MoodReason>,
    pub captain: bool,
    pub influential: bool,
    pub request: Option<RequestView>,
}

/// The dressing room: the captain, the leaders, everyone's mood, open
/// promises and transfer requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DynamicsView {
    pub captain: Option<PlayerRef>,
    /// The most influential players, most influential first.
    pub leaders: Vec<PlayerRef>,
    /// Mean happiness of senior players.
    pub happiness: i8,
    /// Such as `"Good"` or `"Tense"`.
    pub atmosphere: String,
    pub rows: Vec<DynamicsRow>,
    pub promises: Vec<PromiseView>,
    pub statuses: Vec<StatusInfo>,
    pub talks: Vec<TalkInfo>,
}

/// The outcome of a talk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TalkResult {
    pub reaction: Reaction,
    pub message: String,
    /// What the manager said.
    pub said: String,
    /// The player's answer, in their words.
    pub reply: String,
    pub expression: super::media::Expression,
    pub player: PlayerView,
}
