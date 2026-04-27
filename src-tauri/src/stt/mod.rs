//! Speech-to-text via whisper.cpp (whisper-rs bindings).
//!
//! # Usage
//! ```ignore
//! let engine = WhisperEngine::load(SttConfig::default())?;
//! let text   = engine.transcribe(&audio_16khz_f32)?;
//! ```
//!
//! # Model
//! Download a GGML model with `scripts/download_model.ps1` before running.
//! The default path is `models/ggml-base.bin`.

pub mod postprocess;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SttConfig {
    /// Path to the GGML model file, e.g. `"models/ggml-base.bin"`.
    pub model_path:  String,
    /// BCP-47 language hint (`"en"`, `"fr"`, …) or `"auto"` for detection.
    pub language:    String,
    /// Strip Whisper artefacts and capitalise the first letter.
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

// ── WhisperEngine ─────────────────────────────────────────────────────────────

/// Loaded whisper.cpp context.
///
/// Creating an instance is expensive (model mmap + GPU setup).  Create once
/// and reuse for all transcription calls throughout the session.
pub struct WhisperEngine {
    ctx:    WhisperContext,
    config: SttConfig,
}

// SAFETY: whisper-rs marks WhisperContext as Send + Sync since 0.11.
// The underlying C context is not re-entrant, so calls go through &self with
// an internal state object (create_state) per transcription.
unsafe impl Send for WhisperEngine {}
unsafe impl Sync for WhisperEngine {}

impl WhisperEngine {
    /// Resolve a (possibly relative) model path against several candidate
    /// directories so the app works whether launched via `tauri dev`, `cargo
    /// run`, or as an installed binary.
    ///
    /// Search order:
    /// 1. The path as-is (absolute paths or CWD-relative work immediately).
    /// 2. Next to the running executable.
    /// 3. `../models/` relative to the executable (dev layout: exe lives in
    ///    `src-tauri/target/debug/`, models live in `src-tauri/../models/`
    ///    i.e. the workspace root `models/`).
    /// 4. `../../models/` relative to the executable (another common layout).
    fn resolve_model_path(raw: &str) -> Option<std::path::PathBuf> {
        let raw_path = std::path::Path::new(raw);
        if raw_path.is_absolute() {
            return if raw_path.exists() { Some(raw_path.to_owned()) } else { None };
        }
        // 1. CWD-relative
        if raw_path.exists() {
            return Some(raw_path.to_owned());
        }
        // Derive candidates from the exe location
        if let Ok(exe) = std::env::current_exe() {
            let exe_dir = exe.parent().unwrap_or(std::path::Path::new("."));
            // 2. Next to exe: <exe_dir>/models/ggml-base.bin
            let c2 = exe_dir.join(raw);
            if c2.exists() { return Some(c2); }
            // 3. One level up: <exe_dir>/../models/ggml-base.bin
            let c3 = exe_dir.join("..").join(raw);
            if c3.exists() { return Some(c3); }
            // 4. Two levels up: <exe_dir>/../../models/ggml-base.bin
            let c4 = exe_dir.join("../..").join(raw);
            if c4.exists() { return Some(c4); }
            // 5. Three levels up (src-tauri/target/debug → workspace root)
            let c5 = exe_dir.join("../../..").join(raw);
            if c5.exists() { return Some(c5); }
        }
        None
    }

    /// Load a GGML model from disk.
    ///
    /// # Errors
    /// Returns a descriptive string if the model file is missing or corrupt.
    pub fn load(mut config: SttConfig) -> Result<Self, String> {
        match Self::resolve_model_path(&config.model_path) {
            Some(p) => {
                config.model_path = p.to_string_lossy().into_owned();
            }
            None => {
                return Err(format!(
                    "Whisper model not found: '{}' (searched CWD, exe dir, and parent dirs). \
                     Run `scripts/download_model.ps1` to download the model into the project \
                     `models/` folder.",
                    config.model_path
                ));
            }
        }

        log::info!("WhisperEngine: loading '{}'…", config.model_path);
        let ctx = WhisperContext::new_with_params(
            &config.model_path,
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("Failed to load whisper model: {e}"))?;

        log::info!("WhisperEngine: model ready");
        Ok(Self { ctx, config })
    }

    /// Transcribe **mono 16 kHz f32 PCM** audio.
    ///
    /// Blocks until inference completes (use `tokio::task::spawn_blocking`
    /// to call from async code without blocking the executor).
    ///
    /// # Errors
    /// Returns a descriptive string on whisper.cpp internal errors.
    pub fn transcribe(&self, audio: &[f32]) -> Result<String, String> {
        let mut state = self.ctx
            .create_state()
            .map_err(|e| format!("create_state: {e}"))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(&self.config.language));
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);

        state
            .full(params, audio)
            .map_err(|e| format!("whisper inference failed: {e}"))?;

        let n = state.full_n_segments();

        let mut raw = String::new();
        for i in 0..n {
            if let Some(seg) = state.get_segment(i) {
                // to_str_lossy replaces any invalid UTF-8 with U+FFFD rather
                // than returning an error — more robust for real-world models.
                match seg.to_str_lossy() {
                    Ok(s)  => raw.push_str(&s),
                    Err(e) => log::warn!("Whisper segment {i} null pointer: {e}"),
                }
            }
        }

        let result = if self.config.postprocess {
            postprocess::clean(&raw, true)
        } else {
            raw.trim().to_string()
        };

        log::info!("WhisperEngine: transcript = {:?}", result);
        Ok(result)
    }
}

