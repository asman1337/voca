// All orb states that the Rust backend can emit
export type OrbState =
  | "idle"
  | "listening"
  | "transcribing"
  | "injected"
  | "muted";

// Payload sent when transcription lands with no focused input.
// The backend emits a plain string — use it directly.
export type ClipboardPayload = string;

// ── Config (mirrors src-tauri/src/config.rs AppConfig) ───────────────────────
export interface AppConfig {
  model_path:    string;
  language:      string;
  hotkey:        string;
  vad_threshold: number;
  input_mode:    string;
  auto_launch:   boolean;
}

// ── Model catalogue entry (mirrors downloader::ModelEntry) ───────────────────
export interface ModelEntry {
  id:          string;
  name:        string;
  description: string;
  size_mb:     number;
  downloaded:  boolean;
  path:        string | null;
  sha256:      string | null;
}

// ── Download progress payload (model_download_progress event) ────────────────
export interface DownloadProgress {
  model_id:    string;
  bytes_done:  number;
  bytes_total: number;
  percent:     number;
}
