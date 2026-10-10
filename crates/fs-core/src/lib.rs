//! Shared foundation types for the footballsim simulation.
//!
//! Everything in this crate must behave identically on every target the game
//! ships on (native desktop, iOS, WebAssembly). See `docs/ARCHITECTURE.md`,
//! section "Determinism".

pub mod date;
pub mod hash;
pub mod id;
pub mod math;
pub mod rng;

pub use date::GameDate;
pub use hash::{StateHash, state_hash};
pub use id::{Id, IdVec};
pub use rng::{Rng, Seed, Stream};
