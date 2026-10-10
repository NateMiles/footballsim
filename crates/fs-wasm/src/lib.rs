//! The simulation in the browser.
//!
//! One [`Engine`] wraps one `fs_api::Session`. The web app runs it inside
//! a web worker and passes JSON requests through [`Engine::call`]; see
//! `app/src/api/worker.ts`. Build with `scripts/build-wasm.sh`.

// wasm-bindgen's generated glue uses `unsafe`; nothing written here does.
#![allow(unsafe_code)]

use wasm_bindgen::prelude::*;

/// A game session.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct Engine {
    session: fs_api::Session,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine::default()
    }

    /// Handles one JSON request and returns the JSON response.
    pub fn call(&mut self, request: &str) -> String {
        self.session.call_json(request)
    }

    /// The protocol version this build speaks.
    #[wasm_bindgen(js_name = protocolVersion)]
    pub fn protocol_version() -> u32 {
        fs_api::PROTOCOL_VERSION
    }
}
