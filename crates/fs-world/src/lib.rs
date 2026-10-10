//! World state for footballsim: entities, the command log, the daily tick
//! and save snapshots.
//!
//! The world only changes through [`Command`]s applied by [`Game::submit`].
//! Every applied command is recorded, so a world can be rebuilt exactly from
//! its starting snapshot plus its command log (see [`replay`]). This is the
//! basis for replays, bug reports and future online leagues.

pub use fs_core::attributes;
pub mod club;
pub mod command;
pub mod competition;
pub mod game;
pub mod league;
pub use fs_core::ids;
pub mod manager;
pub mod matchday;
pub mod names;
pub mod nation;
pub mod people;
pub mod player;
pub mod season;
pub mod snapshot;
pub mod squad;
pub mod tick;
pub mod world;

pub use attributes::{Attr, Attributes, Hidden, HiddenAttr, Position, PositionRatings};
pub use club::Club;
pub use command::{Actor, Command, CommandError, CommandRecord, Event};
pub use game::{Game, ReplayError, replay};
pub use ids::{ClubId, LeagueId, ManagerId, NationId, PlayerId};
pub use league::{Fixture, League, LeagueSeason, SeasonSummary, TableRow};
pub use manager::{Control, Manager};
pub use names::{NamePool, NameRef};
pub use nation::Nation;
pub use player::{
    Condition, Contract, Foot, Injury, PlayerBio, PlayerInit, Players, SeasonStats, Status,
};
pub use snapshot::SnapshotError;
pub use world::World;
