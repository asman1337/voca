//! VOCA — library root
//! Wires together all subsystems and registers Tauri commands.

mod audio;
mod hotkey;
mod inject;
mod orb;
mod stt;

use std::sync::{Arc, Mutex};
use tauri::Manager;

pub use orb::OrbEngine;

/// Type alias for shared orb state — accessible across Tauri commands.
pub type SharedOrb = Arc<Mutex<OrbEngine>>;

/// Application entry point (called by main.rs and mobile entry).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize the orb engine and store in app state
            let engine = OrbEngine::new(app.handle().clone());
            app.manage(Arc::new(Mutex::new(engine)));

            log::info!("VOCA started — orb engine ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_toggle_listening,
            cmd_start_listening,
            cmd_stop_listening,
            cmd_toggle_mute,
            cmd_get_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VOCA");
}

// ── Tauri commands ──────────────────────────────────────────────────────────

/// Toggle between Idle ↔ Listening (click-to-toggle mode).
#[tauri::command(rename_all = "snake_case")]
fn cmd_toggle_listening(
    orb: tauri::State<SharedOrb>,
) {
    let mut engine = orb.lock().expect("orb lock poisoned");
    engine.toggle_listening();
}

/// Explicitly start listening (PTT press).
#[tauri::command(rename_all = "snake_case")]
fn cmd_start_listening(
    orb: tauri::State<SharedOrb>,
) {
    let mut engine = orb.lock().expect("orb lock poisoned");
    engine.transition(orb::OrbState::Listening);
}

/// Explicitly stop listening (PTT release) → trigger transcription.
#[tauri::command(rename_all = "snake_case")]
fn cmd_stop_listening(
    orb: tauri::State<SharedOrb>,
) {
    let mut engine = orb.lock().expect("orb lock poisoned");
    engine.transition(orb::OrbState::Transcribing);
}

/// Toggle mute on/off.
#[tauri::command(rename_all = "snake_case")]
fn cmd_toggle_mute(
    orb: tauri::State<SharedOrb>,
) {
    let mut engine = orb.lock().expect("orb lock poisoned");
    engine.toggle_mute();
}

/// Return current orb state as a string (for UI init sync).
#[tauri::command(rename_all = "snake_case")]
fn cmd_get_state(orb: tauri::State<SharedOrb>) -> String {
    let engine = orb.lock().expect("orb lock poisoned");
    engine.state().to_string()
}
