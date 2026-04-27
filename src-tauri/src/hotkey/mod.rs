//! Global hotkey daemon.
//!
//! Registers system-wide hotkeys for Push-to-Talk (PTT) and mute toggle.
//!
//! # Default bindings
//! - Right Cmd  (macOS)  / Right Ctrl (Windows/Linux) → PTT (hold)
//! - Cmd+Shift+M (macOS) / Ctrl+Shift+M (Win/Linux)   → Toggle mute
//!
//! See task t-p1-16 in the dev spec.
//!
//! # Planned crate
//! `global-hotkey` = "0.6" — works on macOS, Windows, Linux with
//! native OS hotkey registration (no background thread polling).

/// Stub: registers all global hotkeys.
/// Real implementation uses the `global-hotkey` crate (t-p1-16).
pub fn register(_app: &tauri::AppHandle) {
    log::info!("Hotkey daemon: stub — real global-hotkey registration not yet implemented");
    // TODO (t-p1-16):
    // let manager = GlobalHotKeyManager::new().unwrap();
    // let ptt = HotKey::new(Some(Modifiers::empty()), Code::ControlRight);
    // manager.register(ptt).unwrap();
    // GlobalHotKeyEvent::set_event_handler(Some(move |e| { ... }));
}
