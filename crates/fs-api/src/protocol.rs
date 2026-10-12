//! The request/response protocol.
//!
//! Every host speaks it: the web worker (through `fs-wasm`), the Tauri
//! shell, and later an online league server. Requests and responses are
//! JSON objects tagged by `type`.

use crate::dto::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Protocol version. Bump when a request or response changes shape.
pub const PROTOCOL_VERSION: u32 = 8;

/// Something the UI asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Request {
    /// Version handshake. Answered with [`Response::Hello`].
    Hello,

    /// Generate a new world (no game yet). Answered with
    /// [`Response::ClubChoices`].
    NewWorld { seed: u32, size: WorldSize },
    /// Take charge of a club in the new world and start the game.
    /// Answered with [`Response::Overview`].
    StartCareer {
        club: Id,
        first_name: String,
        last_name: String,
    },
    /// Answered with [`Response::Saved`].
    Save,
    /// Load a game from [`SaveGame::data`]. Answered with
    /// [`Response::Overview`].
    Load { data: String },

    /// Answered with [`Response::Overview`].
    Overview,
    /// Answered with [`Response::Inbox`].
    Inbox,
    /// Mark a message read (or every message, with `None`). Answered with
    /// [`Response::Inbox`].
    MarkRead { id: Option<u32> },
    /// The viewer's squad, or another club's. Answered with
    /// [`Response::Squad`].
    Squad { club: Option<Id> },
    /// Answered with [`Response::Player`].
    Player { player: Id },
    /// Answered with [`Response::Tactics`].
    Tactics,
    /// Choose a tactic, or `None` to let the assistant pick. Answered with
    /// [`Response::Tactics`].
    SetTactic { tactic: Option<Tactic> },
    /// Choose a lineup, or `None` to let the assistant pick. Answered with
    /// [`Response::Tactics`].
    SetLineup { lineup: Option<Lineup> },
    /// A league's fixtures (the viewer's league by default), optionally
    /// only one club's. Answered with [`Response::Fixtures`].
    Fixtures {
        league: Option<Id>,
        club: Option<Id>,
    },
    /// Answered with [`Response::Table`].
    Table { league: Option<Id> },
    /// A club's matches in every competition (the viewer's by default).
    /// Answered with [`Response::Schedule`].
    Schedule { club: Option<Id> },
    /// A cup (the viewer's national cup by default). Answered with
    /// [`Response::Cup`].
    Cup { cup: Option<Id> },
    /// Every league and cup, and continental coefficients. Answered with
    /// [`Response::Competitions`].
    Competitions,
    /// What the assistant knows about a club (the next opponent by
    /// default). Answered with [`Response::OppositionReport`].
    OppositionReport { club: Option<Id> },
    /// Play the next match many times with a tactic (the current one by
    /// default) to see how it might go. Answered with
    /// [`Response::Prediction`].
    Predict { tactic: Option<Tactic>, runs: u32 },
    /// How a league season might end (the viewer's league by default).
    /// Answered with [`Response::Projections`].
    Projections { league: Option<Id> },
    /// Answered with [`Response::Overview`].
    SetWatchMatches { watch: bool },
    /// Let the assistant handle contracts. Answered with
    /// [`Response::Overview`].
    SetDelegation { squad: bool },
    /// Renew a player's contract or sign a free agent. Answered with
    /// [`Response::Player`].
    OfferContract { player: Id, wage: u32, years: u8 },
    /// Answered with [`Response::Squad`].
    ReleasePlayer { player: Id },

    /// Move time on until there is something to look at: a match to
    /// watch, new messages, or a week passed. Answered with
    /// [`Response::Continued`].
    Continue,

    /// Start watching today's match. Answered with [`Response::LiveStart`].
    StartMatch,
    /// Play the watched match up to `until` seconds after kickoff.
    /// Answered with [`Response::LiveChunk`].
    LiveTick { until: f64 },
    /// Give an instruction now. Answered with [`Response::LiveChunk`]
    /// (with no new frames).
    LiveInstruct { instruction: Instruction },
    /// Play out the rest of the watched match, report it to the world and
    /// finish the day. Answered with [`Response::MatchReport`].
    FinishMatch,
    /// Don't watch today's match: the assistant manages it. Answered with
    /// [`Response::Overview`].
    SkipMatch,

    /// Answered with [`Response::Finances`].
    Finances,
    /// Answered with [`Response::Staff`].
    Staff,
    /// Hire a free staff member or renew one of yours. Answered with
    /// [`Response::Staff`].
    HireStaff { staff: Id, wage: u32, years: u8 },
    /// Answered with [`Response::Staff`].
    ReleaseStaff { staff: Id },
    /// Answered with [`Response::Transfers`].
    Transfers,
    /// Find players. Answered with [`Response::Search`].
    SearchPlayers { search: PlayerSearch },
    /// Bid for a player. Answered with [`Response::Transfers`].
    MakeOffer {
        player: Id,
        kind: OfferKind,
        wage: u32,
        years: u8,
        /// A full contract package for a transfer, replacing `wage` and
        /// `years`.
        #[serde(default)]
        #[ts(optional)]
        terms: Option<ContractTerms>,
        /// Percent of the player's next fee the seller keeps.
        #[serde(default)]
        #[ts(optional)]
        sell_on: Option<u8>,
    },
    /// Answer an offer for one of your players. Answered with
    /// [`Response::Transfers`].
    RespondToOffer { offer: u32, response: OfferResponse },
    /// Accept a selling club's price. Answered with
    /// [`Response::Transfers`].
    AcceptCounter { offer: u32 },
    /// Answered with [`Response::Transfers`].
    WithdrawOffer { offer: u32 },
    /// List or unlist one of your players. Answered with
    /// [`Response::Player`].
    SetListing {
        player: Id,
        listing: Option<Listing>,
    },
    /// Send the scouts. Answered with [`Response::Player`].
    ScoutPlayer { player: Id },
    /// Answered with [`Response::Player`].
    CancelScouting { player: Id },
    /// Vacant jobs. Answered with [`Response::Jobs`].
    Jobs,
    /// Take a vacant job. Answered with [`Response::Overview`].
    ApplyForJob { club: Id },
    /// Leave your club. Answered with [`Response::Overview`].
    Resign,
    /// Answered with [`Response::Training`].
    Training,
    /// Set the squad's training. Answered with [`Response::Training`].
    SetTraining {
        focus: TrainingFocus,
        intensity: TrainingIntensity,
    },
    /// Give a player their own training, or stop it. Answered with
    /// [`Response::Training`].
    SetIndividualTraining {
        player: Id,
        training: Option<IndividualTraining>,
    },
    /// The dressing room. Answered with [`Response::Dynamics`].
    Dynamics,
    /// Have a word with one of your players. Answered with
    /// [`Response::Talk`].
    TalkToPlayer { player: Id, talk: Talk },
    /// Accept or refuse a transfer request. Answered with
    /// [`Response::Dynamics`].
    AnswerTransferRequest { player: Id, accept: bool },
    /// Answered with [`Response::Dynamics`].
    SetCaptain { player: Id },
    /// Answered with [`Response::Dynamics`].
    SetSquadStatus { player: Id, status: SquadStatus },
    /// What a player (one of yours, or a free agent) wants in a contract.
    /// Answered with [`Response::Negotiation`].
    ContractTalks { player: Id },
    /// Offer a contract package. Answered with [`Response::Talks`].
    NegotiateContract { player: Id, terms: ContractTerms },
    /// Roles and duties by formation slot (empty for the standard ones),
    /// set-piece takers (`None` for the best on the pitch) and opposition
    /// instructions for the next match. Answered with
    /// [`Response::Tactics`].
    SetInstructions {
        roles: Vec<RoleChoice>,
        penalty_taker: Option<Id>,
        free_kick_taker: Option<Id>,
        corner_taker: Option<Id>,
        marking: Vec<MarkingChoice>,
    },

    /// Performance test: generate a world of this size beside any game.
    /// Answered with [`Response::Benchmark`]; time the call to measure.
    BenchmarkStart { size: WorldSize },
    /// Run the test world on (at most 30 days per call). Answered with
    /// [`Response::Benchmark`].
    BenchmarkRun { days: u16 },
    /// Drop the test world. Answered with [`Response::Benchmark`].
    BenchmarkEnd,
}

/// The core's answer.
// One response per call, serialised straight away: size does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Response {
    Hello {
        protocol: u32,
        version: String,
        /// Whether a game is loaded.
        in_game: bool,
    },
    ClubChoices {
        choices: ClubChoices,
    },
    Saved {
        save: SaveGame,
    },
    Overview {
        overview: Overview,
    },
    Inbox {
        inbox: InboxView,
    },
    Squad {
        squad: SquadView,
    },
    Player {
        player: PlayerView,
    },
    Tactics {
        tactics: TacticsView,
    },
    Fixtures {
        fixtures: FixturesView,
    },
    Table {
        table: TableView,
    },
    Schedule {
        schedule: ScheduleView,
    },
    Cup {
        cup: CupView,
    },
    Competitions {
        competitions: CompetitionsView,
    },
    OppositionReport {
        report: OppositionReport,
    },
    Prediction {
        prediction: Prediction,
    },
    Projections {
        projections: ProjectionsView,
    },
    Continued {
        continued: Continued,
    },
    LiveStart {
        start: LiveStart,
    },
    LiveChunk {
        chunk: LiveChunk,
    },
    MatchReport {
        report: MatchReport,
    },
    Finances {
        finances: FinancesView,
    },
    Staff {
        staff: StaffView,
    },
    Transfers {
        transfers: TransfersView,
    },
    Search {
        search: SearchView,
    },
    Jobs {
        jobs: JobsView,
    },
    Benchmark {
        benchmark: BenchmarkStatus,
    },
    Training {
        training: TrainingView,
    },
    Dynamics {
        dynamics: DynamicsView,
    },
    Negotiation {
        negotiation: NegotiationView,
    },
    Talks {
        result: TalksResult,
    },
    Talk {
        result: TalkResult,
    },
    Error {
        error: ApiError,
    },
}

/// A failed request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "ApiErrorBody")]
pub struct ApiError {
    pub code: ErrorCode,
    /// For people.
    pub message: String,
}

/// What kind of failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// The request could not be parsed.
    BadRequest,
    /// The request needs a game (or a new world) that is not loaded.
    NoGame,
    /// Something referred to does not exist.
    NotFound,
    /// The world refused the command.
    Rejected,
    /// No match is being watched, or the match is over.
    NoMatch,
    /// The save could not be read.
    BadSave,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl From<fs_world::CommandError> for ApiError {
    fn from(e: fs_world::CommandError) -> Self {
        ApiError::new(ErrorCode::Rejected, e.to_string())
    }
}
