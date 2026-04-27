//! Text injection subsystem.
//!
//! Detects the currently focused input element and injects text into it.
//! Falls back to clipboard if no valid input is found.
//!
//! # Platform strategy
//! - macOS 13+  → AXUIElement accessibility API      (t-p1-12)
//! - Windows 10+ → UI Automation → SendInput fallback (t-p1-13)
//! - Linux X11   → xdotool type                       (t-p2-06)
//! - Clipboard   → arboard crate                      (t-p1-14)

/// Result of an injection attempt.
#[derive(Debug)]
pub enum InjectionResult {
    /// Text was injected directly into the focused input.
    Injected,
    /// No focused input found; text was placed on the clipboard.
    Clipboard,
    /// Injection failed entirely.
    Failed(String),
}

/// Inject `text` into the focused application input.
///
/// Tries native injection first; falls back to clipboard on failure.
pub fn inject(text: &str) -> InjectionResult {
    #[cfg(target_os = "macos")]
    {
        match inject_macos(text) {
            Ok(_)  => return InjectionResult::Injected,
            Err(e) => log::warn!("macOS injection failed: {} — falling back to clipboard", e),
        }
    }

    #[cfg(target_os = "windows")]
    {
        match inject_windows(text) {
            Ok(_)  => return InjectionResult::Injected,
            Err(e) => log::warn!("Windows injection failed: {} — falling back to clipboard", e),
        }
    }

    #[cfg(target_os = "linux")]
    {
        match inject_linux(text) {
            Ok(_)  => return InjectionResult::Injected,
            Err(e) => log::warn!("Linux injection failed: {} — falling back to clipboard", e),
        }
    }

    // Clipboard fallback (all platforms)
    clipboard_fallback(text)
}

fn clipboard_fallback(text: &str) -> InjectionResult {
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
        Ok(_) => {
            log::info!("Text placed on clipboard (no focused input)");
            InjectionResult::Clipboard
        }
        Err(e) => {
            log::error!("Clipboard fallback failed: {}", e);
            InjectionResult::Failed(e.to_string())
        }
    }
}

// ── macOS stub (t-p1-12) ──────────────────────────────────────────────────
#[cfg(target_os = "macos")]
fn inject_macos(_text: &str) -> Result<(), String> {
    // TODO (t-p1-12): implement AXUIElement injection
    Err("AXUIElement injection not yet implemented".into())
}

// ── Windows stub (t-p1-13) ───────────────────────────────────────────────
#[cfg(target_os = "windows")]
fn inject_windows(_text: &str) -> Result<(), String> {
    // TODO (t-p1-13): implement UI Automation / SendInput injection
    Err("Windows injection not yet implemented".into())
}

// ── Linux stub (t-p2-06) ─────────────────────────────────────────────────
#[cfg(target_os = "linux")]
fn inject_linux(_text: &str) -> Result<(), String> {
    // TODO (t-p2-06): shell out to xdotool type
    Err("xdotool injection not yet implemented".into())
}
