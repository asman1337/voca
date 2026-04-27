//! Model downloader — fetches, verifies, and stores GGML Whisper models.
//!
//! Downloads are streamed via `reqwest`, written atomically (`.tmp` → final),
//! and hashed with SHA-256.  Progress is broadcast as Tauri events so the
//! frontend can render a live progress bar without polling.
//!
//! # Events emitted on the `AppHandle`
//! | Event                       | Payload              |
//! |-----------------------------|----------------------|
//! `model_download_progress`     | [`ProgressPayload`]  |
//! `model_download_done`         | [`DonePayload`]      |
//! `model_download_error`        | [`ErrorPayload`]     |

use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

// ─── Internal catalogue entry (URL kept server-side, never serialised) ────────

struct Entry {
    id:          &'static str,
    name:        &'static str,
    description: &'static str,
    size_mb:     u32,
    url:         &'static str,
}

static CATALOGUE: &[Entry] = &[
    Entry {
        id:          "tiny.en",
        name:        "Tiny · English",
        description: "~10× real-time. English only. Best for low-power hardware.",
        size_mb:     75,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
    },
    Entry {
        id:          "tiny",
        name:        "Tiny · Multilingual",
        description: "Fastest multilingual model.",
        size_mb:     75,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    },
    Entry {
        id:          "base.en",
        name:        "Base · English",
        description: "Good balance of speed and accuracy. English only.",
        size_mb:     142,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
    },
    Entry {
        id:          "base",
        name:        "Base · Multilingual",
        description: "Multilingual base model.",
        size_mb:     142,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    },
    Entry {
        id:          "small.en",
        name:        "Small · English",
        description: "Better accuracy, ~4× real-time. English only.",
        size_mb:     466,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
    },
    Entry {
        id:          "small",
        name:        "Small · Multilingual",
        description: "Multilingual small model, ~4× real-time.",
        size_mb:     466,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    },
    Entry {
        id:          "medium.en",
        name:        "Medium · English",
        description: "High accuracy, English only. Requires ~4 GB RAM.",
        size_mb:     1_500,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
    },
    Entry {
        id:          "medium",
        name:        "Medium · Multilingual",
        description: "Best multilingual accuracy. Requires ~4 GB RAM.",
        size_mb:     1_500,
        url:         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    },
];

// ─── Event payloads (serialised to the frontend) ──────────────────────────────

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub model_id:    String,
    pub bytes_done:  u64,
    pub bytes_total: u64,
    pub percent:     f32,
}

#[derive(Clone, Serialize)]
pub struct DonePayload {
    pub model_id: String,
    pub path:     String,
    pub sha256:   String,
}

#[derive(Clone, Serialize)]
pub struct ErrorPayload {
    pub model_id: String,
    pub error:    String,
}

// ─── Frontend-visible model entry ────────────────────────────────────────────

/// Returned by [`list_models`].  Includes real-time download status.
#[derive(Clone, Serialize)]
pub struct ModelEntry {
    pub id:          &'static str,
    pub name:        &'static str,
    pub description: &'static str,
    /// Approximate size shown to the user before downloading.
    pub size_mb:     u32,
    /// `true` when the `.bin` file is present on disk.
    pub downloaded:  bool,
    /// Absolute path to the model file, if downloaded.
    pub path:        Option<String>,
    /// SHA-256 hex stored in the sidecar `.sha256` file, if present.
    pub sha256:      Option<String>,
}

// ─── Paths ───────────────────────────────────────────────────────────────────

/// Platform data dir: `%APPDATA%\voca\models` · `~/Library/Application Support/voca/models` · `~/.local/share/voca/models`
pub fn models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("voca")
        .join("models")
}

/// Full path to `ggml-{id}.bin`.
pub fn model_path(id: &str) -> PathBuf {
    models_dir().join(format!("ggml-{id}.bin"))
}

fn hash_path(id: &str) -> PathBuf {
    models_dir().join(format!("ggml-{id}.sha256"))
}

fn stored_hash(id: &str) -> Option<String> {
    std::fs::read_to_string(hash_path(id))
        .ok()
        .map(|s| s.trim().to_owned())
}

// ─── Public queries ───────────────────────────────────────────────────────────

/// Returns the full catalogue annotated with current download / hash status.
pub fn list_models() -> Vec<ModelEntry> {
    CATALOGUE
        .iter()
        .map(|e| {
            let downloaded = model_path(e.id).exists();
            ModelEntry {
                id:          e.id,
                name:        e.name,
                description: e.description,
                size_mb:     e.size_mb,
                downloaded,
                path:   downloaded.then(|| model_path(e.id).to_string_lossy().into_owned()),
                sha256: stored_hash(e.id),
            }
        })
        .collect()
}

/// Returns `true` when the model binary is present on disk.
#[allow(dead_code)]
pub fn is_downloaded(id: &str) -> bool {
    model_path(id).exists()
}

/// Remove the model binary and its SHA-256 sidecar to free disk space.
pub fn delete_model(id: &str) -> Result<(), String> {
    let bin = model_path(id);
    if bin.exists() {
        std::fs::remove_file(&bin).map_err(|e| e.to_string())?;
    }
    let hash = hash_path(id);
    if hash.exists() {
        let _ = std::fs::remove_file(hash);
    }
    log::info!("Model '{id}' deleted from disk");
    Ok(())
}

// ─── Cancellation ─────────────────────────────────────────────────────────────

/// Global flag. Set via [`cancel_download`]; checked inside the chunk loop.
static CANCEL: AtomicBool = AtomicBool::new(false);

/// Signal the in-flight download to stop at the next chunk boundary.
pub fn cancel_download() {
    CANCEL.store(true, Ordering::Relaxed);
    log::info!("Model download cancellation requested");
}

// ─── Download ─────────────────────────────────────────────────────────────────

/// Stream-download a model, compute SHA-256 on the fly, write atomically.
///
/// The call returns as soon as the download completes *or* fails; progress is
/// delivered as Tauri events.  The caller is responsible for spawning this on
/// the async runtime (see `cmd_download_model` in `lib.rs`).
pub async fn download_model(app: AppHandle, model_id: String) -> Result<(), String> {
    // Reset cancellation flag at the start of each fresh download.
    CANCEL.store(false, Ordering::Relaxed);

    let entry = CATALOGUE
        .iter()
        .find(|e| e.id == model_id)
        .ok_or_else(|| format!("Unknown model id: '{model_id}'"))?;

    let dest = model_path(&model_id);
    let tmp  = dest.with_extension("tmp");

    // Ensure models dir exists before opening a file inside it.
    std::fs::create_dir_all(models_dir()).map_err(|e| e.to_string())?;

    log::info!("Downloading model '{model_id}' from {}", entry.url);

    let client = reqwest::Client::builder()
        .user_agent(concat!("VOCA/", env!("CARGO_PKG_VERSION"), " model-downloader"))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(entry.url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} — {}", resp.status(), entry.url));
    }

    let total        = resp.content_length().unwrap_or(0);
    let mut file     = tokio::fs::File::create(&tmp).await.map_err(|e| e.to_string())?;
    let mut hasher   = Sha256::new();
    let mut done     = 0u64;
    let mut stream   = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if CANCEL.load(Ordering::Relaxed) {
            drop(file);
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err("Download cancelled".to_string());
        }

        let bytes = chunk.map_err(|e| format!("Stream error: {e}"))?;
        hasher.update(&bytes);
        file.write_all(&bytes).await.map_err(|e| e.to_string())?;
        done += bytes.len() as u64;

        let percent = if total > 0 { done as f32 / total as f32 * 100.0 } else { 0.0 };
        let _ = app.emit("model_download_progress", ProgressPayload {
            model_id:    model_id.clone(),
            bytes_done:  done,
            bytes_total: total,
            percent,
        });
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

    // Atomic rename — no partial file is ever visible as the real path.
    tokio::fs::rename(&tmp, &dest)
        .await
        .map_err(|e| format!("Rename failed: {e}"))?;

    // Write SHA-256 sidecar so future integrity checks don't need the network.
    let sha256 = hex::encode(hasher.finalize());
    let _ = std::fs::write(hash_path(&model_id), &sha256);

    log::info!("Model '{model_id}' ready at {dest:?}  SHA256={sha256}");

    let _ = app.emit("model_download_done", DonePayload {
        model_id: model_id.clone(),
        path:     dest.to_string_lossy().into_owned(),
        sha256,
    });

    Ok(())
}
