//! The transfer market and scouting.

use super::common::{ClubRef, Date, PlayerRef};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a buyer proposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OfferKind {
    Transfer {
        #[ts(type = "number")]
        fee: i64,
    },
    /// The borrower pays this percentage of the wages.
    Loan { wage_share: u8 },
}

impl From<fs_world::transfer::OfferKind> for OfferKind {
    fn from(k: fs_world::transfer::OfferKind) -> Self {
        match k {
            fs_world::transfer::OfferKind::Transfer { fee } => OfferKind::Transfer { fee },
            fs_world::transfer::OfferKind::Loan { wage_share } => OfferKind::Loan { wage_share },
        }
    }
}

impl From<OfferKind> for fs_world::transfer::OfferKind {
    fn from(k: OfferKind) -> Self {
        match k {
            OfferKind::Transfer { fee } => fs_world::transfer::OfferKind::Transfer { fee },
            OfferKind::Loan { wage_share } => fs_world::transfer::OfferKind::Loan { wage_share },
        }
    }
}

/// Where an offer stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OfferStatus {
    Pending,
    /// The selling club wants this fee.
    Countered {
        #[ts(type = "number")]
        fee: i64,
    },
    /// Agreed; completes in the next daily update.
    Agreed,
    Completed,
    /// Failed, with a reason for people.
    Refused {
        reason: String,
    },
}

impl From<fs_world::transfer::OfferStatus> for OfferStatus {
    fn from(s: fs_world::transfer::OfferStatus) -> Self {
        use fs_world::transfer::{OfferStatus as S, Refusal as R};
        match s {
            S::Pending => OfferStatus::Pending,
            S::Countered { fee } => OfferStatus::Countered { fee },
            S::Agreed => OfferStatus::Agreed,
            S::Completed => OfferStatus::Completed,
            S::Refused(r) => OfferStatus::Refused {
                reason: match r {
                    R::TooLow => "The offer was too low",
                    R::NotForSale => "Not for sale",
                    R::Expired => "No answer in time",
                    R::PlayerRefused => "The player turned it down",
                    R::BuyerCannotComplete => "The buyer could not complete",
                    R::Withdrawn => "Withdrawn",
                }
                .into(),
            },
        }
    }
}

/// An offer for a player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub id: u32,
    pub date: Date,
    pub player: PlayerRef,
    pub buyer: ClubRef,
    pub seller: ClubRef,
    pub kind: OfferKind,
    /// The contract offered to the player (transfers).
    pub wage: u32,
    pub years: u8,
    /// Percent of the player's next fee the seller would keep.
    pub sell_on: u8,
    pub status: OfferStatus,
    pub respond_by: Date,
}

/// How a club lists a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Listing {
    Transfer,
    Loan,
}

impl From<fs_world::transfer::Listing> for Listing {
    fn from(l: fs_world::transfer::Listing) -> Self {
        match l {
            fs_world::transfer::Listing::Transfer => Listing::Transfer,
            fs_world::transfer::Listing::Loan => Listing::Loan,
        }
    }
}

impl From<Listing> for fs_world::transfer::Listing {
    fn from(l: Listing) -> Self {
        match l {
            Listing::Transfer => fs_world::transfer::Listing::Transfer,
            Listing::Loan => fs_world::transfer::Listing::Loan,
        }
    }
}

/// A selling club's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OfferResponse {
    Accept,
    Reject,
    Counter {
        #[ts(type = "number")]
        fee: i64,
    },
}

/// A player out on loan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Loan {
    pub player: PlayerRef,
    pub parent: ClubRef,
    pub borrower: ClubRef,
    pub until: Date,
    pub wage_share: u8,
}

/// A completed move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferRecord {
    pub date: Date,
    pub player: PlayerRef,
    pub from: ClubRef,
    pub to: ClubRef,
    #[ts(type = "number")]
    pub fee: i64,
    pub loan: bool,
}

/// A player being scouted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub player: PlayerRef,
    pub club: Option<ClubRef>,
    /// 0-100.
    pub knowledge: u8,
}

/// The transfers screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransfersView {
    pub window_open: bool,
    /// When the window next opens or closes.
    pub window_changes: Date,
    #[ts(type = "number")]
    pub transfer_budget: i64,
    /// Offers for the viewer's players, newest first.
    pub incoming: Vec<Offer>,
    /// The viewer's offers, newest first.
    pub outgoing: Vec<Offer>,
    /// The viewer's listed players.
    pub listed: Vec<ListedPlayer>,
    pub loans_out: Vec<Loan>,
    pub loans_in: Vec<Loan>,
    pub scouting: Vec<Assignment>,
    /// Scouts available to watch players at once.
    pub scouts: u8,
    /// The biggest recent moves in the world.
    pub recent: Vec<TransferRecord>,
}

/// One of the viewer's listed players.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ListedPlayer {
    pub player: PlayerRef,
    pub listing: Listing,
}

/// Which players a search returns.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSearch {
    /// Best-position codes to include; empty for any.
    pub positions: Vec<String>,
    pub min_age: Option<u8>,
    pub max_age: Option<u8>,
    /// Estimated value at most this.
    #[ts(type = "number | null")]
    pub max_value: Option<i64>,
    /// Estimated ability at least this (0-100).
    pub min_ability: Option<u8>,
    /// Only players their clubs have listed.
    pub listed_only: bool,
    /// Only players without a club.
    pub free_agents_only: bool,
    /// Name contains this (case-insensitive).
    pub name: Option<String>,
}

/// A search result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SearchRow {
    pub player: PlayerRef,
    pub club: Option<ClubRef>,
    pub age: u8,
    pub nation: String,
    /// Estimated ability, 0-100, with its range.
    pub ability: u8,
    pub ability_low: u8,
    pub ability_high: u8,
    pub potential: u8,
    /// 0-100.
    pub knowledge: u8,
    /// Estimated value.
    #[ts(type = "number")]
    pub value: i64,
    pub wage_demand: u32,
    pub listing: Option<Listing>,
}

/// Search results, best first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SearchView {
    pub rows: Vec<SearchRow>,
    /// Matches before the limit.
    pub total: u32,
}
