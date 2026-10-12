//! World state for footballsim: entities, the command log, the daily tick
//! and save snapshots.
//!
//! The world only changes through [`Command`]s applied by [`Game::submit`].
//! Every applied command is recorded, so a world can be rebuilt exactly from
//! its starting snapshot plus its command log (see [`replay`]). This is the
//! basis for replays, bug reports and future online leagues.

pub use fs_core::attributes;
pub mod board;
pub mod club;
pub mod command;
pub mod competition;
pub mod contract;
pub mod cup;
pub mod economy;
pub mod finance;
pub mod fixture;
pub mod forecast;
pub mod format;
pub mod game;
pub mod happiness;
pub mod importance;
pub mod league;
pub mod lineup;
pub mod live;
pub use fs_core::ids;
pub mod manager;
pub mod market;
pub mod matchday;
pub mod names;
pub mod nation;
pub mod people;
pub mod personality;
pub mod player;
pub mod profile;
pub mod scouting;
pub mod season;
pub mod snapshot;
pub mod squad;
pub mod staff;
pub mod talk;
pub mod tick;
pub mod training;
pub mod transfer;
pub mod world;

pub use attributes::{Attr, Attributes, Hidden, HiddenAttr, Position, PositionRatings};
pub use board::Board;
pub use club::Club;
pub use command::{Actor, Command, CommandError, CommandRecord, Event, OfferResponse};
pub use cup::{Continent, Cup, CupEdition, CupScope};
pub use finance::{Facilities, Finances, Ledger, Money, SeasonAccounts};
pub use fixture::FixtureRef;
pub use game::{Game, ReplayError, replay};
pub use ids::{ClubId, CupId, LeagueId, ManagerId, NationId, PlayerId, StaffId};
pub use league::{Fixture, League, LeagueSeason, SeasonSummary, TableRow};
pub use lineup::Lineup;
pub use live::{PendingMatchDay, WatchedFixture};
pub use manager::{Control, Manager};
pub use names::{NamePool, NameRef};
pub use nation::Nation;
pub use player::{
    Condition, Contract, Foot, Injury, PlayerBio, PlayerInit, Players, SeasonStats, Status,
};
pub use snapshot::SnapshotError;
pub use staff::{Staff, StaffAttrs, StaffRole};
pub use world::{Rules, World};
