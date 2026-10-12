//! Views: read-only projections of the world, shaped for one screen each.
//!
//! Every function takes the world and, where it matters, the viewer, so
//! that per-club knowledge (scouting, fog of war) can be added without
//! changing callers.

pub mod club;
pub mod competition;
pub mod contracts;
pub mod cup;
pub mod game;
pub mod instructions;
pub mod management;
pub mod market;
pub mod people;
pub mod preparation;
pub mod refs;
pub mod training;
