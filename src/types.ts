// All orb states that the Rust backend can emit
export type OrbState =
  | "idle"
  | "listening"
  | "transcribing"
  | "injected"
  | "muted";

// Payload sent when transcription lands with no focused input
export interface ClipboardPayload {
  text: string;
}
