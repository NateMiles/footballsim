//! The frame stream format.

use crate::result::Side;
use fs_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// Pitch length in metres (goal line to goal line).
pub const PITCH_LENGTH: f32 = 105.0;
/// Pitch width in metres.
pub const PITCH_WIDTH: f32 = 68.0;
/// Frames per second of match time. Renderers interpolate between them.
pub const FRAME_HZ: f32 = 5.0;

/// What a player is doing, so a renderer can pick an animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pose {
    Standing,
    Running,
    Passing,
    Shooting,
    Heading,
    Tackling,
    Diving,
    Celebrating,
}

/// One player at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerFrame {
    pub id: PlayerId,
    pub side: Side,
    /// Metres from the home team's goal line (the left end on screen).
    pub x: f32,
    /// Metres from the top touchline.
    pub y: f32,
    /// Direction faced, radians (0 = toward the right-hand goal).
    pub facing: f32,
    pub pose: Pose,
}

/// Everything on the pitch at one instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Seconds since kickoff, counting stoppage time.
    pub t: f32,
    /// Ball position in metres: x, y as for players, z is height.
    pub ball: [f32; 3],
    /// Everyone on the pitch (sent-off players are absent).
    pub players: Vec<PlayerFrame>,
}
