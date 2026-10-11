//! Roles and duties, set-piece takers and opposition instructions.

use super::common::{ClubRef, Id, PlayerRef};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A player's role in their formation slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PlayerRole {
    Goalkeeper,
    SweeperKeeper,
    CentralDefender,
    BallPlayingDefender,
    FullBack,
    WingBack,
    DefensiveMidfielder,
    BallWinner,
    DeepPlaymaker,
    CentralMidfielder,
    BoxToBox,
    Playmaker,
    WideMidfielder,
    Winger,
    InsideForward,
    WideForward,
    AttackingMidfielder,
    ShadowStriker,
    Striker,
    TargetMan,
    Poacher,
    FalseNine,
}

/// How a player balances attack and defence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Duty {
    Defend,
    Support,
    Attack,
}

/// An instruction for one opposition player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Marking {
    Tight,
    CloseDown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RoleInfo {
    pub role: PlayerRole,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MarkingInfo {
    pub marking: Marking,
    pub name: String,
    pub description: String,
}

/// One formation slot's role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SlotRoleView {
    /// Position code of the slot, such as `"ML"`.
    pub position: String,
    pub role: PlayerRole,
    pub duty: Duty,
    /// Roles the slot can take, standard first.
    pub options: Vec<RoleInfo>,
    /// Whether the slot takes a duty (every slot but the keeper's).
    pub has_duty: bool,
}

/// An opposition instruction for the next match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MarkingView {
    pub player: PlayerRef,
    pub club: Option<ClubRef>,
    pub marking: Marking,
}

/// The manager's instructions beyond the tactic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InstructionsView {
    /// By formation slot, for the formation the team plays now.
    pub roles: Vec<SlotRoleView>,
    /// `None` leaves the choice to the best player for it on the pitch.
    pub penalty_taker: Option<Id>,
    pub free_kick_taker: Option<Id>,
    pub corner_taker: Option<Id>,
    /// For the next match only.
    pub marking: Vec<MarkingView>,
    pub markings: Vec<MarkingInfo>,
    /// At most this many opposition instructions.
    pub max_marking: u8,
}

/// A role and duty as the UI sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RoleChoice {
    pub role: PlayerRole,
    pub duty: Duty,
}

/// An opposition instruction as the UI sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MarkingChoice {
    pub player: Id,
    pub marking: Marking,
}
