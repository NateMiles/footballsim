//! Contract negotiation.

use super::common::{ClubRef, Date, PlayerRef};
use super::people::{SquadStatus, StatusInfo};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A contract package, offered or asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContractTerms {
    /// Per week.
    pub wage: u32,
    pub years: u8,
    pub status: SquadStatus,
    pub appearance_bonus: u32,
    pub goal_bonus: u32,
    pub signing_fee: i64,
    pub release_clause: Option<i64>,
}

/// Where talks with a player stand, and what they want.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NegotiationView {
    pub player: PlayerRef,
    /// The club they play for now (none for a free agent).
    pub club: Option<ClubRef>,
    /// Whether this is a renewal.
    pub renewal: bool,
    /// The agent's opening ask.
    pub ask: ContractTerms,
    /// Whether they insist on a release clause.
    pub wants_clause: bool,
    /// Such as `"Reasonable"` or `"Hard bargainer"`.
    pub agent: String,
    /// The agent's fee as a percentage of the contract's wages.
    pub agent_fee_percent: u8,
    /// Offers left before the agent walks away.
    pub offers_left: u8,
    /// Talks broke down; they reopen on this day.
    pub closed_until: Option<Date>,
    /// The player wants to leave and will not renew.
    pub wants_out: bool,
    /// Weekly wages the club pays now and may pay in all.
    pub wage_bill: i64,
    pub wage_budget: i64,
    pub statuses: Vec<StatusInfo>,
}

/// How a round of talks went.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TalksOutcome {
    Agreed,
    Countered { counter: ContractTerms },
    BrokenOff { until: Date },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TalksResult {
    pub outcome: TalksOutcome,
    /// What the agent said.
    pub message: String,
    pub negotiation: NegotiationView,
}

/// A former club's share of a player's next fee.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SellOnView {
    pub club: ClubRef,
    pub percent: u8,
}
