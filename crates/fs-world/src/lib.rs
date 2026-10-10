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
pub use fs_core::ids;
pub mod manager;
pub mod matchday;
pub mod names;
pub mod nation;
pub mod player;
pub mod snapshot;
pub mod tick;
pub mod world;

pub use attributes::{Attr, Attributes, Hidden, HiddenAttr, Position, PositionRatings};
pub use club::Club;
pub use command::{Actor, Command, CommandError, CommandRecord, Event};
pub use game::{Game, ReplayError, replay};
pub use ids::{ClubId, ManagerId, NationId, PlayerId};
pub use manager::{Control, Manager};
pub use names::{NamePool, NameRef};
pub use nation::Nation;
pub use player::{Condition, Foot, PlayerBio, PlayerInit, Players};
pub use snapshot::SnapshotError;
pub use world::World;
