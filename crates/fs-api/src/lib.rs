//! The boundary between the simulation and every user interface.
//!
//! A UI sends [`Request`]s and receives [`Response`]s, as JSON objects
//! tagged by `type`. Hosts carry them: `fs-wasm` in a web worker, the
//! Tauri shell natively, and later an online league server. Nothing
//! outside this crate needs to know the core's types.
//!
//! - [`protocol`]: the requests and responses.
//! - [`dto`]: view models, mirrored into TypeScript by `ts-rs`.
//! - [`query`]: builds views from the world, from a viewer's side.
//! - [`session`]: the host state behind the protocol.
//! - [`inbox`], [`commentary`], [`live`], [`save`]: what the session uses.

pub mod commentary;
pub mod dto;
pub mod inbox;
pub mod live;
pub mod protocol;
pub mod query;
pub mod save;
pub mod session;

pub use protocol::{ApiError, ErrorCode, PROTOCOL_VERSION, Request, Response};
pub use session::Session;
