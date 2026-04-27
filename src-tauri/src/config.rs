//! Application configuration backed by `~/.config/voca/config.toml`.
//!
//! On first run VOCA writes sensible defaults so the file exists and the user
//! can inspect / edit it directly.  The Tauri commands `cmd_get_config` and
//! `cmd_save_config` expose read/write to the frontend.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── AppConfig ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Path to the GGML model file, e.g. `"models/ggml-base.bin"`.
    pub model_path: String,

    /// BCP-47 language hint (`"en"`, `"fr"`, …) or `"auto"` for Whisper
    /// auto-detection.
    pub language: String,

    /// Global hotkey string used by the hotkey daemon, e.g. `"ctrl+shift+v"`.
    /// Currently read-only from Rust; live rebinding comes in a later task.
    pub hotkey: String,

    /// Energy-based VAD silence threshold (0.0 – 1.0).
    /// Reserved for the v0.5 VAD wiring task.
    pub vad_threshold: f32,

    /// How to deliver transcribed text: `"inject"` (SendInput/AXUIElement)
    /// or `"clipboard"` (always use clipboard fallback).
    pub input_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model_path:    "models/ggml-base.bin".into(),
            language:      "en".into(),
            hotkey:        "ctrl+shift+v".into(),
            vad_threshold: 0.5,
            input_mode:    "inject".into(),
        }
    }
}

impl AppConfig {
    /// Returns `~/.config/voca/config.toml`.
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("voca")
            .join("config.toml")
    }

    /// Load config from disk.  Writes defaults on first run and recovers
    /// gracefully from parse errors.
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(s) => toml::from_str::<Self>(&s).unwrap_or_else(|e| {
                    log::warn!("Config parse error ({e}); using defaults");
                    Self::default()
                }),
                Err(e) => {
                    log::warn!("Config read error ({e}); using defaults");
                    Self::default()
                }
            }
        } else {
            let cfg = Self::default();
            if let Err(e) = cfg.save() {
                log::warn!("Could not write default config: {e}");
            }
            cfg
        }
    }

    /// Persist config to disk, creating parent directories as needed.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let s = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, s).map_err(|e| e.to_string())?;
        log::info!("Config saved → {}", path.display());
        Ok(())
    }
}
