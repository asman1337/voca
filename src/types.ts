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
