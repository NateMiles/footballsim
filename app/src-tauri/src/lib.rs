//! The native shell. The web UI runs in the system web view and sends
//! protocol requests through one command, `call`, to an `fs_api::Session`
//! running natively (with real threads, unlike the browser build). The UI
//! detects the shell and uses its `TauriTransport` instead of the web
//! worker; nothing else differs.

use std::sync::{Arc, Mutex};

/// The game session, shared by every `call`.
#[derive(Default)]
struct Core(Arc<Mutex<fs_api::Session>>);

/// Handles one JSON request. Runs off the main thread so a long
/// simulation never blocks the window.
#[tauri::command]
async fn call(request: String, core: tauri::State<'_, Core>) -> Result<String, String> {
    let session = Arc::clone(&core.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut session = session
            .lock()
            .map_err(|_| "the simulation stopped".to_owned())?;
        Ok(session.call_json(&request))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Core::default())
        .invoke_handler(tauri::generate_handler![call])
        .run(tauri::generate_context!())
        .expect("error while running footballsim");
}
