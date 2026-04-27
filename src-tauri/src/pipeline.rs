//! Pipeline — the central coordinator of the VOCA audio→STT→inject flow.
//!
//! # Click-to-toggle flow (v0.1 MVP)
//! ```text
//! user clicks orb
//!   └─ Idle      → begin_recording()    → Listening
//!   └─ Listening → end_and_transcribe() → Transcribing
//!                                             │
//!                   whisper.cpp (blocking)    │
//!                                             ▼
//!                              inject text → Injected
//!                                     or  → clipboard + card
//!                                     or  → Idle (error/empty)
//! ```
//!
//! # VAD-driven flow (v0.5, not yet wired)
//! The `audio::vad::EnergyVad` will auto-detect speech boundaries and call
//! `end_and_transcribe()` without the user clicking.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::audio::AudioCapture;
use crate::inject::{inject, InjectionResult};
use crate::orb::{OrbEngine, OrbState};
use crate::stt::{SttConfig, WhisperEngine};

/// Alias used throughout the codebase.
pub type SharedPipeline = Arc<Pipeline>;

// ── Pipeline struct ───────────────────────────────────────────────────────────

pub struct Pipeline {
    /// The orb state machine.  Public so Tauri commands can query state.
    pub orb:  Arc<Mutex<OrbEngine>>,

    /// Active recording session.  `Some` while the orb is in Listening state.
    capture:  Mutex<Option<AudioCapture>>,

    /// Whisper engine — loaded lazily the first time transcription is needed.
    whisper:  Mutex<Option<Arc<WhisperEngine>>>,

    /// STT config (model path, language, post-process flag).
    stt_cfg:  SttConfig,

    /// Tauri AppHandle for emitting events to the frontend.
    pub app:  AppHandle,
}

// SAFETY: Pipeline is accessed only through Arc<Pipeline>.  All mutable state
// is guarded by Mutex.  AppHandle is Send+Sync.
unsafe impl Send for Pipeline {}
unsafe impl Sync for Pipeline {}

impl Pipeline {
    /// Create a new pipeline bound to the given Tauri app handle.
    pub fn new(app: AppHandle) -> Self {
        let orb = Arc::new(Mutex::new(OrbEngine::new(app.clone())));
        Self {
            orb,
            capture: Mutex::new(None),
            whisper: Mutex::new(None),
            stt_cfg: SttConfig::default(),
            app,
        }
    }

    // ── Public command API (called from Tauri commands and hotkey thread) ──────

    /// Toggle: Idle→Listening (start capture) or Listening→Transcribing (run STT).
    pub fn handle_toggle(&self) {
        let state = self.orb.lock().unwrap().state().clone();
        match state {
            OrbState::Idle      => self.begin_recording(),
            OrbState::Listening => self.end_and_transcribe(),
            OrbState::Muted     => log::debug!("Toggle ignored — orb is muted"),
            _                   => log::debug!("Toggle ignored in state {state:?}"),
        }
    }

    /// Explicit start (PTT press, or frontend button).
    pub fn handle_start(&self) {
        let state = self.orb.lock().unwrap().state().clone();
        if state == OrbState::Idle {
            self.begin_recording();
        }
    }

    /// Explicit stop → trigger transcription (PTT release).
    pub fn handle_stop(&self) {
        let state = self.orb.lock().unwrap().state().clone();
        if state == OrbState::Listening {
            self.end_and_transcribe();
        }
    }

    /// Mute / un-mute toggle.
    pub fn handle_mute_toggle(&self) {
        // If recording when muted, discard the capture
        let _ = self.capture.lock().unwrap().take();
        self.orb.lock().unwrap().toggle_mute();
    }

    /// Dismiss the clipboard card (Injected → Idle).
    pub fn handle_dismiss(&self) {
        let mut orb = self.orb.lock().unwrap();
        if *orb.state() == OrbState::Injected {
            orb.transition(OrbState::Idle);
        }
    }

    /// Return current state as a JSON-safe string for the frontend init sync.
    pub fn get_state_str(&self) -> String {
        self.orb.lock().unwrap().state().to_string()
    }

    // ── Internal audio pipeline ───────────────────────────────────────────────

    fn begin_recording(&self) {
        if self.capture.lock().unwrap().is_some() {
            log::warn!("begin_recording: already recording, ignoring");
            return;
        }

        match AudioCapture::start() {
            Ok(cap) => {
                *self.capture.lock().unwrap() = Some(cap);
                self.orb.lock().unwrap().transition(OrbState::Listening);
            }
            Err(e) => {
                log::error!("Microphone open failed: {e}");
                let _ = self.app.emit("orb-error", format!("Mic error: {e}"));
                // State remains Idle
            }
        }
    }

    fn end_and_transcribe(&self) {
        // Take the capture out — state becomes None (not recording)
        let capture = self.capture.lock().unwrap().take();
        let Some(capture) = capture else {
            log::warn!("end_and_transcribe: no active capture");
            return;
        };

        // Transition to Transcribing immediately so the UI spins
        self.orb.lock().unwrap().transition(OrbState::Transcribing);

        // Clone handles for the async task
        let whisper = self.get_or_init_whisper();
        let app     = self.app.clone();
        let orb     = Arc::clone(&self.orb);

        // Offload to the async runtime (tokio, wired by Tauri)
        tauri::async_runtime::spawn(async move {
            // stop_and_drain is cheap (stream drop + buffer clone) but needs a
            // blocking thread because cpal internals may block briefly.
            let audio = tokio::task::spawn_blocking(move || capture.stop_and_drain())
                .await
                .unwrap_or_default();

            // Guard against spurious very-short recordings (< 100 ms)
            if audio.len() < 1_600 {
                log::info!("Audio segment too short ({} samples), discarding", audio.len());
                orb.lock().unwrap().transition(OrbState::Idle);
                return;
            }

            let Some(engine) = whisper else {
                log::error!("Whisper engine unavailable — model missing?");
                orb.lock().unwrap().transition(OrbState::Idle);
                let _ = app.emit(
                    "orb-error",
                    "Model not found. Run scripts/download_model.ps1 to download it.",
                );
                return;
            };

            // Run whisper on a blocking thread (CPU-intensive; may take seconds)
            let result = tokio::task::spawn_blocking(move || engine.transcribe(&audio))
                .await
                .unwrap_or_else(|e| Err(e.to_string()));

            match result {
                Ok(text) if text.is_empty() => {
                    log::info!("STT: empty transcript");
                    orb.lock().unwrap().transition(OrbState::Idle);
                }

                Ok(text) => {
                    match inject(&text) {
                        InjectionResult::Injected => {
                            orb.lock().unwrap().transition(OrbState::Injected);
                        }
                        InjectionResult::Clipboard => {
                            // Show copy card in the frontend
                            let _ = app.emit("orb-clipboard-ready", &text);
                            orb.lock().unwrap().transition(OrbState::Injected);
                        }
                        InjectionResult::Failed(e) => {
                            log::error!("Injection failed: {e}");
                            // Last resort: put on clipboard silently
                            let _ = crate::inject::clipboard_fallback(&text);
                            let _ = app.emit("orb-clipboard-ready", &text);
                            orb.lock().unwrap().transition(OrbState::Injected);
                        }
                    }
                }

                Err(e) => {
                    log::error!("STT error: {e}");
                    orb.lock().unwrap().transition(OrbState::Idle);
                    let _ = app.emit("orb-error", format!("Transcription failed: {e}"));
                }
            }
        });
    }

    // ── Whisper lazy-load ─────────────────────────────────────────────────────

    /// Return the whisper engine, initialising it on first call.
    ///
    /// Returns `None` if the model file is not present.
    fn get_or_init_whisper(&self) -> Option<Arc<WhisperEngine>> {
        let mut guard = self.whisper.lock().unwrap();
        if guard.is_none() {
            match WhisperEngine::load(self.stt_cfg.clone()) {
                Ok(engine) => *guard = Some(Arc::new(engine)),
                Err(e)     => {
                    log::error!("Whisper init failed: {e}");
                    return None;
                }
            }
        }
        guard.clone()
    }
}
