//! Running the club: money, staff, the board and jobs.

use super::common::{ClubRef, Date, Id, LeagueRef};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A season's money, by kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Ledger {
    pub matchday: i64,
    pub broadcast: i64,
    pub commercial: i64,
    pub prize_money: i64,
    pub player_sales: i64,
    pub player_wages: i64,
    pub staff_wages: i64,
    pub player_purchases: i64,
    pub running_costs: i64,
    pub investment: i64,
    pub income: i64,
    pub expenses: i64,
}

impl From<&fs_world::Ledger> for Ledger {
    fn from(l: &fs_world::Ledger) -> Self {
        Self {
            matchday: l.matchday,
            broadcast: l.broadcast,
            commercial: l.commercial,
            prize_money: l.prize_money,
            player_sales: l.player_sales,
            player_wages: l.player_wages,
            staff_wages: l.staff_wages,
            player_purchases: l.player_purchases,
            running_costs: l.running_costs,
            investment: l.investment,
            income: l.income(),
            expenses: l.expenses(),
        }
    }
}

/// A finished season's accounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SeasonAccounts {
    /// Such as `"2026/27"`.
    pub season: String,
    pub ledger: Ledger,
    pub closing_balance: i64,
}

/// Stadium and facilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Facilities {
    pub capacity: u32,
    /// 1-20.
    pub training: u8,
    pub youth: u8,
}

/// The finances screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FinancesView {
    pub club: ClubRef,
    pub balance: i64,
    pub transfer_budget: i64,
    /// Weekly.
    pub wage_budget: i64,
    /// Weekly, players.
    pub wage_bill: i64,
    /// Weekly, staff.
    pub staff_bill: i64,
    pub season: Ledger,
    /// Newest first.
    pub history: Vec<SeasonAccounts>,
    pub facilities: Facilities,
}

/// What the board expects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    /// Finish at this position or better.
    pub target: u8,
    /// Such as `"Finish in the top half"`.
    pub objective: String,
    /// 0-100.
    pub confidence: u8,
}

/// A staff role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StaffRole {
    AssistantManager,
    Coach,
    Scout,
    Physio,
}

impl From<fs_world::StaffRole> for StaffRole {
    fn from(r: fs_world::StaffRole) -> Self {
        match r {
            fs_world::StaffRole::AssistantManager => StaffRole::AssistantManager,
            fs_world::StaffRole::Coach => StaffRole::Coach,
            fs_world::StaffRole::Scout => StaffRole::Scout,
            fs_world::StaffRole::Physio => StaffRole::Physio,
        }
    }
}

/// Staff abilities, each 1-20.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StaffAttributes {
    pub coaching: u8,
    pub youth_coaching: u8,
    pub judging_ability: u8,
    pub judging_potential: u8,
    pub physiotherapy: u8,
    pub management: u8,
}

/// A staff member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StaffMember {
    pub id: Id,
    pub name: String,
    pub role: StaffRole,
    pub age: u8,
    pub nation: String,
    pub attributes: StaffAttributes,
    /// The rating that matters for the role, 1-20.
    pub rating: u8,
    /// Current weekly wage, for employed staff.
    pub wage: Option<u32>,
    pub expires: Option<Date>,
    /// What they ask for, per week.
    pub demand: u32,
    /// Whether they would join the viewer's club.
    pub would_join: bool,
}

/// A club's departments, each 1-20 (10 is average).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StaffRatings {
    pub coaching: f32,
    pub youth_coaching: f32,
    pub scouting_ability: f32,
    pub scouting_potential: f32,
    pub physiotherapy: f32,
    pub management: f32,
}

/// The staff screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StaffView {
    pub club: ClubRef,
    pub staff: Vec<StaffMember>,
    pub ratings: StaffRatings,
    /// Free staff, best first.
    pub available: Vec<StaffMember>,
    /// The most of each role a club may employ, in role order.
    pub limits: Vec<u8>,
}

/// A job going.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub club: ClubRef,
    pub league: LeagueRef,
    /// 1-5.
    pub stars: u8,
    /// Whether the board would take the viewer.
    pub would_hire: bool,
}

/// The jobs screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JobsView {
    /// The viewer's reputation, 0-100.
    pub reputation: u8,
    pub employed: bool,
    pub jobs: Vec<Job>,
}
