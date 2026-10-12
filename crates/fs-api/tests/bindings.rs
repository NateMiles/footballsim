//! TypeScript declarations for the protocol, checked into the web app.
//!
//! The file is generated from the Rust types, so the UI can never drift
//! from the core. If this test fails, regenerate it:
//!
//! ```sh
//! UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings
//! ```

use fs_api::dto::*;
use fs_api::protocol::*;
use ts_rs::{Config, TS};

macro_rules! decls {
    ($cfg:expr; $($t:ty),* $(,)?) => {
        vec![$(<$t as TS>::decl($cfg)),*]
    };
}

fn generate() -> String {
    // Money is i64 in Rust; JSON numbers carry it exactly well past any
    // club's accounts, so it is a plain number in TypeScript.
    let cfg = Config::new().with_large_int("number");
    let cfg = &cfg;
    let mut decls = decls![cfg;
        // Protocol.
        Request, Response, ApiError, ErrorCode,
        // Common.
        ClubRef, Colors, LeagueRef, PlayerRef, Side, Formation, Tactic, SeasonStats,
        // Game.
        WorldSize, ClubChoice, ClubChoices, Overview, ManagerInfo, MatchDay, StopReason,
        Continued, SaveGame, SaveSummary, BenchmarkStatus,
        // Club.
        SquadView, Selection, SquadPlayer, Injury, Contract, PlayerView, AttributeGroup,
        AttributeValue, CareerRow, PositionInfo, FormationInfo, SlotInfo, SliderInfo,
        TacticsView, SelectablePlayer, Lineup,
        // Competition.
        Fixture, FixtureResult, Scorer, FixturesView, TableZone, TableView, TableRow,
        CompetitionKind, CompetitionRef, ScheduleView, CupRef, CupView, CupStage, Tie, GroupView,
        CupWinner, CupStatus, CompetitionsView, NationCompetitions, ContinentView,
        CoefficientRow, Honour,
        // Inbox.
        Category, Link, Message, InboxView,
        // Live.
        LiveStart, Pitch, LivePlayerInfo, LiveLineup, OnPitch, Shout, Instruction, LiveChunk,
        Frame, PlayerFrame, Pose, CommentaryKind, Commentary, MatchEvent, TeamStats,
        MatchReport, PlayerRating, Shootout, Kick,
        // Management.
        Ledger, SeasonAccounts, Facilities, FinancesView, BoardView, StaffRole, StaffAttributes,
        StaffMember, StaffRatings, StaffView, Job, JobsView,
        // Market.
        OfferKind, OfferStatus, Offer, Listing, OfferResponse, Loan, TransferRecord, Assignment,
        TransfersView, ListedPlayer, PlayerSearch, SearchRow, SearchView,
        // Preparation.
        OppositionReport, ReportPlayer, KeyPlayer, Record, Comparison, Prediction, ScoreChance,
        ProjectionsView, ProjectionZone, ProjectionRow,
        // Training.
        TrainingFocus, TrainingIntensity, IndividualTraining, FocusInfo, IntensityInfo,
        FormationFamiliarity, TrainingPlayer, TrainingView,
        // People.
        SquadStatus, StatusInfo, Talk, TalkInfo, Reaction, Personality, MoodReason, Mood,
        PromiseView, RequestView, PlayerPeople, DynamicsRow, DynamicsView, TalkResult,
        // Instructions.
        PlayerRole, Duty, Marking, RoleInfo, MarkingInfo, SlotRoleView, MarkingView,
        InstructionsView, RoleChoice, MarkingChoice,
        // Contracts.
        ContractTerms, NegotiationView, TalksOutcome, TalksResult, SellOnView,
        // Analysis.
        ShotView, XgPoint, NetworkNode, NetworkLink, SideAnalysisView, PlayerLine,
        MatchAnalysisView, CompetitionSeason, SeasonStatsView, FixtureKey,
    ];
    decls.sort();
    let mut out = String::from(
        "// Generated from crates/fs-api by tests/bindings.rs. Do not edit.\n\
         // Regenerate: UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings\n\n",
    );
    out.push_str(&format!(
        "export const PROTOCOL_VERSION = {PROTOCOL_VERSION};\n"
    ));
    for d in decls {
        out.push('\n');
        out.push_str("export ");
        out.push_str(d.trim());
        out.push('\n');
    }
    out
}

#[test]
fn typescript_bindings_are_current() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src/api/generated.ts");
    let fresh = generate();
    if std::env::var_os("UPDATE_BINDINGS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &fresh).unwrap();
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        current == fresh,
        "app/src/api/generated.ts is stale; run UPDATE_BINDINGS=1 cargo test -p fs-api --test bindings"
    );
}
