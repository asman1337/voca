//! VOCA — library root.
//!
//! Declares all subsystem modules, wires the [`Pipeline`] into Tauri's
//! managed state, registers IPC commands, and starts the hotkey daemon.

mod audio;
mod hotkey;
mod inject;
mod orb;
mod pipeline;
mod stt;

use std::sync::Arc;
use tauri::Manager;

use pipeline::{Pipeline, SharedPipeline};

/// Application entry point (also used by mobile via `tauri::mobile_entry_point`).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // ── 1. Create the central pipeline ────────────────────────────
            let pipeline: SharedPipeline = Arc::new(Pipeline::new(app.handle().clone()));
            app.manage(Arc::clone(&pipeline));

            // ── 2. Start global hotkey daemon ──────────────────────────────
            hotkey::setup(Arc::clone(&pipeline));

            log::info!("VOCA initialised — pipeline ready, hotkeys active");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_toggle_listening,
            cmd_start_listening,
            cmd_stop_listening,
            cmd_toggle_mute,
            cmd_get_state,
            cmd_dismiss,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VOCA");
}

// ── Tauri IPC commands ────────────────────────────────────────────────────────

/// Toggle: Idle → Listening (start capture) or Listening → Transcribing.
#[tauri::command]
fn cmd_toggle_listening(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_toggle();
}

/// Explicitly start listening (PTT press or UI button).
#[tauri::command]
fn cmd_start_listening(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_start();
}

/// Explicitly stop and begin transcription (PTT release).
#[tauri::command]
fn cmd_stop_listening(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_stop();
}

/// Toggle mute on / off.
#[tauri::command]
fn cmd_toggle_mute(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_mute_toggle();
}

/// Return current orb state string for frontend initialisation sync.
#[tauri::command]
fn cmd_get_state(pipeline: tauri::State<SharedPipeline>) -> String {
    pipeline.get_state_str()
}

/// Dismiss the clipboard card (Injected → Idle).
#[tauri::command]
fn cmd_dismiss(pipeline: tauri::State<SharedPipeline>) {
    pipeline.handle_dismiss();
}

