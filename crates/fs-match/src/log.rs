//! A neutral record of what happened on the ball, action by action.
//!
//! Engines that resolve play as discrete actions (tier 1) emit these when
//! asked. The match view's director turns them into positions over time,
//! and the commentary turns them into words. A spatial engine (tier 0)
//! can emit the same records alongside its own frames.

use crate::result::Side;
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// A zone of the pitch in the acting side's frame: band 0 is their own
/// goal line, band 5 the opponent's penalty area; lane 0 is their left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zone {
    pub band: u8,
    pub lane: u8,
}

/// The kind of action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    ShortPass,
    LongPass,
    ThroughBall,
    Cross,
    Dribble,
    Shot,
    Header,
    Penalty,
    FreeKick,
    Corner,
    Foul,
}

/// How it turned out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Reached a team-mate, beat a defender, or (for a corner) found a head.
    Complete,
    /// Cut out by an opponent.
    Intercepted,
    /// The ball carrier was dispossessed.
    Tackled,
    /// Went out of play to the opponent.
    OutOfPlay,
    Offside,
    /// The actor was fouled (dribbles) or committed the foul (fouls).
    Foul,
    Goal,
    Saved,
    Missed,
    Blocked,
    /// A set piece played short or cleared without a shot.
    Cleared,
}

/// One action.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Engine step the action belongs to.
    pub step: u32,
    /// Seconds since kickoff when it started, counting stoppage time.
    pub at: f32,
    /// Seconds the ball was in play.
    pub duration: f32,
    /// Seconds of dead ball after it (restarts, celebrations).
    pub dead: f32,
    pub side: Side,
    pub kind: ActionKind,
    pub outcome: Outcome,
    pub actor: PlayerId,
    /// The intended receiver, if any.
    pub target: Option<PlayerId>,
    /// The opponent most involved, if any.
    pub opponent: Option<PlayerId>,
    pub from: Zone,
    pub to: Zone,
    /// Expected goals, for shots.
    pub xg: f32,
}
