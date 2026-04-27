//! Speech-to-text subsystem — whisper.cpp bindings.
//!
//! # Planned implementation (v0.1 MVP)
//!
//! 1. Load a ggml model file from `models/` via `whisper-rs`.
//! 2. Accept a `Vec<f32>` audio buffer at 16 kHz.
//! 3. Run inference and return the transcript string.
//! 4. Apply post-processing (trim, artifact removal, optional capitalization).
//!
//! See tasks t-p1-09, t-p1-10, t-p1-11 in the dev spec.

// ── TODO (t-p1-09): Bind whisper.cpp ─────────────────────────────────────
// use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams};

pub mod postprocess;

/// Configuration for the STT engine.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SttConfig {
    /// Path to the ggml model file, e.g. "models/ggml-base.bin"
    pub model_path:  String,
    /// BCP-47 language code ("en", "fr", etc.) or "auto"
    pub language:    String,
    /// Strip Whisper artifacts and capitalize first letter
    pub postprocess: bool,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            model_path:  "models/ggml-base.bin".into(),
            language:    "en".into(),
            postprocess: true,
        }
    }
}

/// Stub: transcribes a raw audio buffer.
/// Returns the recognized text or an error string.
pub fn transcribe(_audio: &[f32], _config: &SttConfig) -> Result<String, String> {
    // TODO (t-p1-09): real whisper-rs call
    Err("STT not yet implemented — add whisper-rs and implement transcribe()".into())
}
