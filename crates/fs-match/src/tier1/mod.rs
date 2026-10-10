//! Tier 1: event-level match simulation.
//!
//! The pitch is a grid of six bands (own goal to opponent goal, from the
//! attacking team's point of view) by three lanes (left, centre, right).
//! The team in possession repeatedly picks an action for the ball carrier:
//! short pass, long ball, through ball, cross, dribble or shot. Each action
//! is a contest between the carrier's skill and a nearby defender's,
//! resolved with a logistic curve. Who is "nearby" comes from each team's
//! formation: players near a zone are more often involved there.
//!
//! The engine also handles fouls, cards, penalties, set pieces, injuries,
//! in-match fatigue and substitutions. All tunable numbers live in
//! [`Params`] so the calibration tools can fit them.

use crate::input::MatchInput;
use crate::result::MatchResult;

mod actions;
mod engine;
mod live;
mod params;
mod team;
#[cfg(test)]
mod tests;

use engine::Engine;
pub use live::{LiveLineup, LiveMatch, LivePlayer};
pub use params::Params;

const BANDS: usize = 6;
const LANES: usize = 3;
const HALF_SECS: f64 = 45.0 * 60.0;
const LANE_Y: [f64; LANES] = [0.15, 0.5, 0.85];
const MAX_SUBS: u8 = 5;
const MAX_SUB_WINDOWS: u8 = 3;

/// Plays a match with the calibrated parameters.
pub fn play(input: &MatchInput) -> MatchResult {
    play_with(input, &Params::default())
}

/// Plays a match with custom parameters.
pub fn play_with(input: &MatchInput, params: &Params) -> MatchResult {
    debug_assert_eq!(input.validate(), Ok(()));
    Engine::new(input, params, false).run()
}
