//! The manager's inbox.

use super::common::{Date, Id};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a message is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Match,
    Injury,
    Contract,
    Transfer,
    Youth,
    Season,
    Club,
    Finance,
    Board,
    Staff,
    Scouting,
    Cup,
}

/// Where a message leads in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Link {
    Player {
        player: Id,
    },
    /// The viewer's matches.
    Schedule,
    Table {
        league: Id,
    },
    Cup {
        cup: Id,
    },
    Squad,
    Tactics,
    Transfers,
    Staff,
    Jobs,
    Finances,
}

/// One message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: u32,
    pub date: Date,
    pub category: Category,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub link: Option<Link>,
}

/// The inbox, newest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InboxView {
    pub messages: Vec<Message>,
    pub unread: u32,
}
