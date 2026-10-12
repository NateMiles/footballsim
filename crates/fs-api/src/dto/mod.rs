//! View models: the shapes the UI receives and sends.
//!
//! These mirror core types rather than exposing them, so the core can
//! change its internals without breaking any UI, and each type is shaped
//! for a screen. TypeScript declarations for all of them are generated
//! into `app/src/api/generated.ts` (see `tests/bindings.rs`).

pub mod club;
pub mod common;
pub mod competition;
pub mod contracts;
pub mod game;
pub mod inbox;
pub mod instructions;
pub mod live;
pub mod management;
pub mod market;
pub mod people;
pub mod preparation;
pub mod training;

pub use club::*;
pub use common::*;
pub use competition::*;
pub use contracts::*;
pub use game::*;
pub use inbox::*;
pub use instructions::*;
pub use live::*;
pub use management::*;
pub use market::*;
pub use people::*;
pub use preparation::*;
pub use training::*;
