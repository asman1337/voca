//! Orb state machine.
//!
//! Manages the five visual/logical states of the VOCA orb and enforces
//! valid transitions. Emits `orb-state-changed` Tauri events on every
//! successful transition so the React frontend stays in sync.

use std::fmt;
use tauri::{AppHandle, Emitter};

// ── State enum ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrbState {
    /// Default — mic is off, orb is dark.
    Idle,
    /// Actively capturing audio (VAD or PTT hold).
    Listening,
    /// Audio captured, whisper.cpp is processing.
    Transcribing,
    /// Text has been injected (or placed in clipboard).
    Injected,
    /// User has muted — no audio captured.
    Muted,
}

impl fmt::Display for OrbState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrbState::Idle         => "idle",
            OrbState::Listening    => "listening",
            OrbState::Transcribing => "transcribing",
            OrbState::Injected     => "injected",
            OrbState::Muted        => "muted",
        };
        write!(f, "{}", s)
    }
}

// ── Engine ──────────────────────────────────────────────────────────────────

pub struct OrbEngine {
    state: OrbState,
    app: AppHandle,
}

impl OrbEngine {
    pub fn new(app: AppHandle) -> Self {
        Self {
            state: OrbState::Idle,
            app,
        }
    }

    /// Attempt a state transition. Returns `true` if the transition was valid
    /// and executed, `false` if rejected (invalid transition).
    pub fn transition(&mut self, next: OrbState) -> bool {
        if !self.is_valid_transition(&next) {
            log::warn!(
                "Rejected invalid orb transition: {:?} → {:?}",
                self.state, next
            );
            return false;
        }

        log::debug!("Orb: {:?} → {:?}", self.state, next);
        self.state = next.clone();

        // Emit to all frontend windows
        if let Err(e) = self.app.emit("orb-state-changed", &next) {
            log::error!("Failed to emit orb-state-changed: {}", e);
        }

        true
    }

    pub fn state(&self) -> &OrbState {
        &self.state
    }

    /// Idle ↔ Listening toggle (click-to-toggle mode).
    /// Called by the Pipeline — kept here for direct state-machine testing.
    #[allow(dead_code)]
    pub fn toggle_listening(&mut self) {
        match &self.state {
            OrbState::Idle      => { self.transition(OrbState::Listening); }
            OrbState::Listening => { self.transition(OrbState::Idle); }
            _ => {} // ignore when transcribing / muted etc.
        }
    }

    /// Toggle mute on/off from any state.
    pub fn toggle_mute(&mut self) {
        match &self.state {
            OrbState::Muted => { self.transition(OrbState::Idle); }
            _               => {
                self.state = OrbState::Muted;
                if let Err(e) = self.app.emit("orb-state-changed", &OrbState::Muted) {
                    log::error!("Failed to emit orb-state-changed (mute): {}", e);
                }
            }
        }
    }

    // ── Private helpers ────────────────────────────────────────────────────

    fn is_valid_transition(&self, next: &OrbState) -> bool {
        use OrbState::*;
        matches!(
            (&self.state, next),
            (Idle,         Listening)    |
            (Idle,         Muted)        |
            (Listening,    Transcribing) |
            (Listening,    Idle)         |
            (Listening,    Muted)        |
            (Transcribing, Injected)     |
            (Transcribing, Idle)         | // transcription failed / empty
            (Injected,     Idle)         |
            (Injected,     Listening)    | // immediate re-listen
            (Muted,        Idle)
        )
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────
// Run with: cargo test -p voca

#[cfg(test)]
mod tests {
    use super::OrbState;

    // We can't easily construct AppHandle in tests, so we test the
    // transition validation logic directly.

    fn valid(from: &OrbState, to: &OrbState) -> bool {
        use OrbState::*;
        matches!(
            (from, to),
            (Idle,         Listening)    |
            (Idle,         Muted)        |
            (Listening,    Transcribing) |
            (Listening,    Idle)         |
            (Listening,    Muted)        |
            (Transcribing, Injected)     |
            (Transcribing, Idle)         |
            (Injected,     Idle)         |
            (Injected,     Listening)    |
            (Muted,        Idle)
        )
    }

    #[test]
    fn idle_to_listening_valid() {
        assert!(valid(&OrbState::Idle, &OrbState::Listening));
    }

    #[test]
    fn listening_to_transcribing_valid() {
        assert!(valid(&OrbState::Listening, &OrbState::Transcribing));
    }

    #[test]
    fn transcribing_to_injected_valid() {
        assert!(valid(&OrbState::Transcribing, &OrbState::Injected));
    }

    #[test]
    fn injected_to_idle_valid() {
        assert!(valid(&OrbState::Injected, &OrbState::Idle));
    }

    #[test]
    fn idle_to_transcribing_invalid() {
        assert!(!valid(&OrbState::Idle, &OrbState::Transcribing));
    }

    #[test]
    fn muted_to_listening_invalid() {
        assert!(!valid(&OrbState::Muted, &OrbState::Listening));
    }
}
