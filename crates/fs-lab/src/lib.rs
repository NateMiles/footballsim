//! The match engine laboratory: season simulations, calibration reports,
//! tier 2 fitting and match prediction.

pub mod calibrate;
pub mod choices;
pub mod compare;
pub mod fit;
pub mod predict;
pub mod season;

pub use calibrate::{Check, Report, report};
pub use predict::{Prediction, predict};
pub use season::{
    Engine, MatchRecord, SeasonOutcome, league_world, simulate_seasons, simulate_seasons_with,
};
