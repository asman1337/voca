//! Global hotkey daemon.
//!
//! Registers system-wide hotkeys and drives the pipeline via them.
//!
//! # Default bindings
//! | Key               | Platform        | Action             |
//! |-------------------|-----------------|--------------------|
//! | Right Ctrl (tap)  | Windows / Linux | Push-to-talk toggle|
//! | Ctrl + Shift + M  | All             | Mute toggle        |
//!
//! # Architecture
//! The `global-hotkey` crate registers hotkeys with the OS and delivers
//! events through a lock-free channel.  A dedicated background thread
//! (`voca-hotkeys`) blocks on [`GlobalHotKeyEvent::receiver()`] and calls
//! into the [`Pipeline`] on each relevant event.
//!
//! The [`GlobalHotKeyManager`] must stay alive for as long as the hotkeys
//! should remain active.  We intentionally leak it (see comment in
//! [`setup`]) since its lifetime equals the process lifetime.

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};

use crate::pipeline::SharedPipeline;

/// Register all hotkeys and spawn the event-handler thread.
///
/// This function returns immediately; the hotkeys stay active in the
/// background for the lifetime of the process.
pub fn setup(pipeline: SharedPipeline) {
    let manager = GlobalHotKeyManager::new()
        .expect("Failed to initialise GlobalHotKeyManager");

    // Right Ctrl (no modifiers) → push-to-talk toggle
    let ptt = HotKey::new(None, Code::ControlRight);
    // Ctrl + Shift + M → mute toggle
    let mute = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyM);

    manager.register(ptt).unwrap_or_else(|e| {
        log::warn!("Could not register PTT hotkey: {e}");
    });
    manager.register(mute).unwrap_or_else(|e| {
        log::warn!("Could not register mute hotkey: {e}");
    });

    let ptt_id  = ptt.id();
    let mute_id = mute.id();

    log::info!("Hotkeys registered — PTT: Right Ctrl | Mute: Ctrl+Shift+M");

    // The manager MUST outlive the registered hotkeys.  Since it needs to live
    // as long as the process and there is no suitable owner, we intentionally
    // leak it.  This is not a bug — it is a deliberate OS resource lifetime
    // decision equivalent to `static mut` without the unsafety.
    std::mem::forget(manager);

    // Background thread: block on hotkey events and dispatch to the pipeline.
    std::thread::Builder::new()
        .name("voca-hotkeys".into())
        .spawn(move || {
            let receiver = GlobalHotKeyEvent::receiver();
            loop {
                match receiver.recv() {
                    Ok(event) => {
                        // Only act on key-press, not key-release
                        if event.state != HotKeyState::Pressed {
                            continue;
                        }
                        if event.id == ptt_id {
                            log::debug!("Hotkey: PTT");
                            pipeline.handle_toggle();
                        } else if event.id == mute_id {
                            log::debug!("Hotkey: Mute");
                            pipeline.handle_mute_toggle();
                        }
                    }
                    Err(e) => {
                        // Channel closed — this should never happen while the
                        // process is alive, but handle it gracefully.
                        log::error!("Hotkey event channel closed: {e}");
                        break;
                    }
                }
            }
        })
        .expect("Failed to spawn hotkey thread");
}

