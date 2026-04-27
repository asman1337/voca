//! Text injection — types transcribed text into the currently focused window.
//!
//! # Platform strategy
//! | Platform | Primary method             | Fallback      |
//! |----------|---------------------------|---------------|
//! | Windows  | `SendInput` KEYEVENTF_UNICODE | Clipboard  |
//! | macOS    | TODO (AXUIElement)         | Clipboard     |
//! | Linux    | TODO (xdotool)             | Clipboard     |
//!
//! `KEYEVENTF_UNICODE` sends each UTF-16 code unit as a synthetic keystroke.
//! This works with virtually every Windows application (browsers, editors,
//! IDEs, native apps) without needing to know the focused element.
//!
//! The clipboard fallback places the text on the system clipboard and emits
//! an `orb-clipboard-ready` Tauri event so the frontend can show the copy card.

/// Outcome of an injection attempt.
#[derive(Debug)]
pub enum InjectionResult {
    /// Text was typed into the focused input via keyboard simulation.
    Injected,
    /// No injectable target found; text is on the clipboard (user pastes).
    Clipboard,
    /// Both injection and clipboard fallback failed.
    Failed(String),
}

/// Inject `text` into whatever the OS reports as the focused window.
///
/// Tries the platform-native method first; falls back to the system clipboard
/// if that fails.  All paths are logged.
pub fn inject(text: &str) -> InjectionResult {
    if text.is_empty() {
        return InjectionResult::Injected; // nothing to do
    }

    #[cfg(target_os = "windows")]
    {
        match inject_windows(text) {
            Ok(())  => return InjectionResult::Injected,
            Err(e)  => log::warn!("Windows SendInput failed: {e} — clipboard fallback"),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match inject_macos(text) {
            Ok(())  => return InjectionResult::Injected,
            Err(e)  => log::warn!("macOS injection failed: {e} — clipboard fallback"),
        }
    }

    #[cfg(target_os = "linux")]
    {
        match inject_linux(text) {
            Ok(())  => return InjectionResult::Injected,
            Err(e)  => log::warn!("Linux injection failed: {e} — clipboard fallback"),
        }
    }

    clipboard_fallback(text)
}

// ── Clipboard fallback (all platforms) ───────────────────────────────────────

pub(crate) fn clipboard_fallback(text: &str) -> InjectionResult {
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
        Ok(()) => {
            log::info!("inject: text placed on clipboard");
            InjectionResult::Clipboard
        }
        Err(e) => {
            log::error!("Clipboard fallback failed: {e}");
            InjectionResult::Failed(e.to_string())
        }
    }
}

// ── Windows — SendInput with KEYEVENTF_UNICODE ────────────────────────────────
//
// Each Unicode code point is encoded as one or two UTF-16 code units.
// We send a key-down then key-up INPUT event per code unit.
// The `KEYEVENTF_UNICODE` flag bypasses the keyboard layout mapping so any
// character is delivered correctly regardless of the user's locale.

#[cfg(target_os = "windows")]
fn inject_windows(text: &str) -> Result<(), String> {
    use std::mem::size_of;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY,
    };

    // Encode as UTF-16: BMP chars → 1 code unit; supplementary → surrogate pair
    let code_units: Vec<u16> = text.encode_utf16().collect();
    if code_units.is_empty() {
        return Ok(());
    }

    // Pre-allocate: one key-down + one key-up per code unit
    let mut inputs: Vec<INPUT> = Vec::with_capacity(code_units.len() * 2);

    for &cu in &code_units {
        // Key down
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk:         VIRTUAL_KEY(0),
                    wScan:       cu,
                    dwFlags:     KEYEVENTF_UNICODE,
                    time:        0,
                    dwExtraInfo: 0,
                },
            },
        });
        // Key up
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk:         VIRTUAL_KEY(0),
                    wScan:       cu,
                    dwFlags:     KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time:        0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    // SAFETY: inputs is a valid, fully-initialised Vec<INPUT>.
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };

    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(format!(
            "SendInput: {}/{} events delivered — OS blocked some inputs",
            sent, inputs.len()
        ))
    }
}

// ── macOS stub (t-p1-12) ──────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn inject_macos(_text: &str) -> Result<(), String> {
    // TODO (t-p1-12): use AXUIElementSetAttributeValue + CGEventPost
    Err("macOS text injection not yet implemented".into())
}

// ── Linux stub (t-p2-06) ─────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn inject_linux(_text: &str) -> Result<(), String> {
    // TODO (t-p2-06): shell out to `xdotool type --clearmodifiers`
    Err("Linux text injection not yet implemented".into())
}

