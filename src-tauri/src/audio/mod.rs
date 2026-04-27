//! Audio subsystem — microphone capture and VAD.
//!
//! # Planned implementation (v0.1 MVP)
//!
//! 1. Open default input device via CPAL at 16 kHz mono PCM.
//! 2. Push samples into a ring buffer (`RingBuffer`).
//! 3. Run Silero VAD (ONNX) on 30 ms frames.
//! 4. On speech_start → notify orb engine → start buffering.
//! 5. On speech_end → flush buffer to STT module.
//!
//! See tasks t-p1-06, t-p1-07, t-p1-08 in the dev spec.

// ── TODO (t-p1-06): Microphone capture via CPAL ───────────────────────────
// use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// ── TODO (t-p1-07): Silero VAD via ONNX Runtime ──────────────────────────
// use ort::{Environment, Session};

// ── TODO (t-p1-08): Ring buffer ───────────────────────────────────────────
pub mod ring_buffer;

/// Stub: starts the audio capture thread.
/// Will be replaced with real CPAL capture in t-p1-06.
pub fn start() {
    log::info!("Audio subsystem: stub — real CPAL capture not yet implemented");
}

/// Stub: stops the audio capture thread.
pub fn stop() {
    log::info!("Audio subsystem: stop (stub)");
}
