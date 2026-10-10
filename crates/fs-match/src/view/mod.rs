//! Renderer-neutral match viewing (docs/MATCH_ENGINE.md, section 7).
//!
//! The engines never draw. A watched match produces a stream of
//! [`Frame`]s: every player's position, facing and pose, and the ball's
//! position including height. Any renderer (the 2D canvas now, a 3D one
//! later) consumes the same stream.
//!
//! Tier 1 resolves play as discrete actions, so its frames come from a
//! [`Director`] that stages those actions on a pitch. A spatial engine
//! (tier 0) would produce frames directly and slot in behind
//! [`LiveView`] without the renderers noticing.

mod director;
mod frame;
mod live_view;

pub use director::Director;
pub use frame::{FRAME_HZ, Frame, PITCH_LENGTH, PITCH_WIDTH, PlayerFrame, Pose};
pub use live_view::{LiveChunk, LiveView};
