//! A match watched live.

use super::common::{Id, PlayerRef, Side, Tactic};
use super::competition::Fixture;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Everything the match screen needs before kickoff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LiveStart {
    pub fixture: Fixture,
    /// The side the viewer manages.
    pub side: Side,
    pub pitch: Pitch,
    /// Frames per second of match time.
    pub frame_hz: f32,
    /// Everyone in either squad today.
    pub players: Vec<LivePlayerInfo>,
    pub lineups: [LiveLineup; 2],
    pub shouts: Vec<Shout>,
}

/// Pitch size in metres. Frames use x along the length (0 at the home
/// goal) and y across (0 at the touchline on the home side's left).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Pitch {
    pub length: f32,
    pub width: f32,
}

/// A player in a live match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LivePlayerInfo {
    pub player: PlayerRef,
    pub side: Side,
}

/// A side's lineup at this moment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LiveLineup {
    pub tactic: Tactic,
    /// By formation slot; empty after a sending off.
    pub on_pitch: Vec<Option<OnPitch>>,
    pub bench: Vec<Id>,
    pub substitutions_left: u8,
    pub shout: Option<Shout>,
}

/// A player on the pitch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OnPitch {
    pub id: Id,
    pub slot: u8,
    /// 0-1.
    pub energy: f32,
    pub yellow: bool,
}

/// A touchline shout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Shout {
    Encourage,
    DemandMore,
    CalmDown,
    TightenUp,
    PushForward,
}

impl From<fs_match::Shout> for Shout {
    fn from(s: fs_match::Shout) -> Self {
        use fs_match::Shout as S;
        match s {
            S::Encourage => Shout::Encourage,
            S::DemandMore => Shout::DemandMore,
            S::CalmDown => Shout::CalmDown,
            S::TightenUp => Shout::TightenUp,
            S::PushForward => Shout::PushForward,
        }
    }
}

impl From<Shout> for fs_match::Shout {
    fn from(s: Shout) -> Self {
        use fs_match::Shout as S;
        match s {
            Shout::Encourage => S::Encourage,
            Shout::DemandMore => S::DemandMore,
            Shout::CalmDown => S::CalmDown,
            Shout::TightenUp => S::TightenUp,
            Shout::PushForward => S::PushForward,
        }
    }
}

/// A decision during the match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Instruction {
    Substitute { off: Id, on: Id },
    ChangeTactic { tactic: Tactic },
    Shout { shout: Shout },
}

/// The match's progress since the last chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LiveChunk {
    pub frames: Vec<Frame>,
    pub commentary: Vec<Commentary>,
    pub events: Vec<MatchEvent>,
    pub score: [u8; 2],
    /// Such as `"45+2"`.
    pub clock: String,
    /// Seconds since kickoff, counting stoppage time.
    pub elapsed: f64,
    pub finished: bool,
    pub stats: [TeamStats; 2],
    pub lineups: [LiveLineup; 2],
}

/// Where everything is at one instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    /// Seconds since kickoff.
    pub t: f32,
    /// x, y and height in metres.
    pub ball: [f32; 3],
    pub players: Vec<PlayerFrame>,
}

/// A player's position and pose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerFrame {
    pub id: Id,
    pub side: Side,
    pub x: f32,
    pub y: f32,
    /// Radians; 0 faces the away goal.
    pub facing: f32,
    pub pose: Pose,
}

/// What a player is doing, for animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Pose {
    Standing,
    Running,
    Passing,
    Shooting,
    Heading,
    Tackling,
    Diving,
    Celebrating,
}

/// How a commentary line should be styled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommentaryKind {
    Goal,
    Chance,
    Save,
    Card,
    Substitution,
    Injury,
    Foul,
    SetPiece,
    Play,
    Whistle,
}

/// A line of commentary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Commentary {
    /// Seconds since kickoff.
    pub t: f32,
    pub clock: String,
    pub kind: CommentaryKind,
    pub side: Option<Side>,
    pub text: String,
}

/// A notable moment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MatchEvent {
    Goal {
        minute: u8,
        side: Side,
        scorer: Id,
        assist: Option<Id>,
        /// `"openPlay"`, `"header"`, `"longShot"`, `"penalty"`,
        /// `"freeKick"` or `"corner"`.
        kind: String,
        xg: f32,
    },
    PenaltyMissed {
        minute: u8,
        side: Side,
        taker: Id,
    },
    Card {
        minute: u8,
        side: Side,
        player: Id,
        red: bool,
    },
    Substitution {
        minute: u8,
        side: Side,
        off: Id,
        on: Id,
    },
    Injury {
        minute: u8,
        side: Side,
        player: Id,
    },
}

/// Team totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TeamStats {
    /// 0-1.
    pub possession: f32,
    pub shots: u16,
    pub shots_on_target: u16,
    pub xg: f32,
    pub passes: u16,
    pub passes_completed: u16,
    pub crosses: u16,
    pub tackles: u16,
    pub fouls: u16,
    pub yellows: u16,
    pub reds: u16,
    pub corners: u16,
    pub offsides: u16,
    pub saves: u16,
}

/// The finished match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MatchReport {
    pub fixture: Fixture,
    pub stats: [TeamStats; 2],
    pub events: Vec<MatchEvent>,
    /// Everyone who played, by side.
    pub ratings: [Vec<PlayerRating>; 2],
}

/// One player's match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRating {
    pub player: PlayerRef,
    pub minutes: u8,
    pub goals: u8,
    pub assists: u8,
    pub rating: f32,
}
