//! The news and the press room.

use super::common::{ClubRef, Date, Id, Look, PlayerRef};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Which stories to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum NewsScope {
    /// About the viewer's club.
    Club,
    /// About clubs from the viewer's nation.
    Nation,
    /// The biggest stories anywhere, and the viewer's own.
    World,
}

/// What a story is about, for its label and look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ArticleKind {
    Result,
    Transfer,
    Rumour,
    Manager,
    Trophy,
    Injury,
    Unrest,
    Youth,
    Retirement,
    Quote,
}

/// A face as a story shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Expression {
    Neutral,
    Happy,
    Delighted,
    Determined,
    Annoyed,
    Angry,
    Sad,
    Sceptical,
    Talking,
}

/// A person in a story's picture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Figure {
    pub name: String,
    pub look: Look,
    pub expression: Expression,
    /// Whose shirt they wear.
    pub club: Option<ClubRef>,
}

/// What a story's picture shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Art {
    /// A scoreboard.
    Score {
        home: ClubRef,
        away: ClubRef,
        goals: [u8; 2],
        competition: String,
    },
    /// A person in front of their club's colours.
    Portrait { figure: Figure, caption: String },
    /// A player between two clubs.
    Move {
        figure: Figure,
        from: Option<ClubRef>,
        to: ClubRef,
        fee: String,
    },
    /// A trophy in the club's colours.
    Trophy { club: ClubRef, competition: String },
    /// A person at a press-room microphone.
    Microphone { figure: Figure },
}

/// Who wrote it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Byline {
    pub name: String,
    pub outlet: String,
    pub look: Option<Look>,
}

/// A news article.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    pub id: Id,
    pub date: Date,
    pub kind: ArticleKind,
    pub headline: String,
    /// One sentence under the headline.
    pub standfirst: String,
    pub body: Vec<String>,
    pub byline: Byline,
    pub art: Art,
    /// How big a story it is, 0-100.
    pub weight: u8,
    /// About the viewer's club.
    pub mine: bool,
    /// The player it is about, if any, to follow up.
    pub player: Option<Id>,
}

/// The news, newest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NewsView {
    pub scope: NewsScope,
    pub articles: Vec<Article>,
}

/// How a manager answers a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Confident,
    Cautious,
    Praise,
    Criticise,
    Deflect,
    Defiant,
    Humble,
    Dismissive,
}

/// A journalist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct JournalistView {
    pub name: String,
    pub outlet: String,
    /// `"broadsheet"`, `"tabloid"`, `"broadcaster"` or `"fanSite"`.
    pub kind: String,
    pub look: Look,
}

/// One answer on offer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AnswerOption {
    pub tone: Tone,
    /// Such as `"Confident"`.
    pub label: String,
    /// What the manager says.
    pub text: String,
}

/// Something an answer did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EffectView {
    pub text: String,
    /// Good for the viewer, bad, or neither.
    pub good: Option<bool>,
}

/// A question and, once answered, what was said and what it did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct QuestionView {
    pub index: u8,
    pub journalist: JournalistView,
    pub text: String,
    pub player: Option<PlayerRef>,
    pub options: Vec<AnswerOption>,
    pub answer: Option<AnswerOption>,
    pub effects: Vec<EffectView>,
}

/// A press conference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceView {
    pub id: Id,
    pub title: String,
    pub pre_match: bool,
    pub club: ClubRef,
    pub opponent: ClubRef,
    /// The last day it can be attended.
    pub closes: Date,
    pub questions: Vec<QuestionView>,
}

/// The press room.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PressView {
    pub conferences: Vec<ConferenceView>,
    /// -100 (hostile) to 100 (adored).
    pub rapport: i16,
    /// Such as `"Friendly"`.
    pub rapport_label: String,
    /// The assistant goes to every conference.
    pub delegated: bool,
    /// The viewer's most recent quotes.
    pub quotes: Vec<Article>,
}
