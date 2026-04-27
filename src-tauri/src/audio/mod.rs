//! Audio capture subsystem — CPAL microphone input.
//!
//! Opens the default input device, records interleaved PCM into a shared
//! buffer, and on stop converts to mono 16 kHz f32 PCM for the STT engine.
//!
//! The VAD module (`audio::vad`) can post-process the raw samples before STT.

pub mod ring_buffer;
pub mod vad;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

// ── Shared buffer ────────────────────────────────────────────────────────────
/// Lock-guarded sample buffer shared between the CPAL callback and the
/// control thread.
type Buf = Arc<Mutex<Vec<f32>>>;

// ── AudioCapture ─────────────────────────────────────────────────────────────

/// An active microphone recording session.
///
/// Dropping this value stops the CPAL stream and frees the device.
pub struct AudioCapture {
    /// Holds the stream alive.  Must not be dropped before `stop_and_drain`.
    _stream:     cpal::Stream,
    buffer:      Buf,
    /// Native sample rate reported by the device.
    pub sample_rate: u32,
}

impl AudioCapture {
    /// Open the default input device and begin recording immediately.
    ///
    /// # Errors
    /// Returns a human-readable string if no device is available, the device
    /// rejects the config, or the stream fails to start.
    pub fn start() -> Result<Self, String> {
        let host   = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No input device available".to_string())?;

        let supported = device
            .default_input_config()
            .map_err(|e| format!("Cannot query input config: {e}"))?;

        let channels    = supported.channels() as usize;
        let sample_rate = supported.sample_rate();
        let format      = supported.sample_format();
        let stream_cfg: cpal::StreamConfig = supported.into();

        log::info!(
            "AudioCapture: device={:?}  {}Hz  {}ch  {:?}",
            device.description(),
            sample_rate, channels, format
        );

        // Pre-allocate 60 s worth of mono samples (worst-case buffer)
        let buffer: Buf = Arc::new(Mutex::new(
            Vec::with_capacity(sample_rate as usize * 60),
        ));

        let buf_cb = Arc::clone(&buffer);
        let err_fn = |e: cpal::StreamError| log::error!("Audio stream error: {e}");

        // Build the stream for whichever sample format the device reports.
        // All formats are normalised to mono f32 before storage.
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_cfg,
                move |data: &[f32], _| push_mono(data, channels, &buf_cb),
                err_fn, None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &stream_cfg,
                move |data: &[i16], _| {
                    let f: Vec<f32> = data.iter()
                        .map(|&s| s as f32 / i16::MAX as f32)
                        .collect();
                    push_mono(&f, channels, &buf_cb);
                },
                err_fn, None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &stream_cfg,
                move |data: &[u16], _| {
                    let f: Vec<f32> = data.iter()
                        .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                        .collect();
                    push_mono(&f, channels, &buf_cb);
                },
                err_fn, None,
            ),
            other => return Err(format!("Unsupported sample format: {other:?}")),
        }
        .map_err(|e| format!("Failed to build input stream: {e}"))?;

        stream.play().map_err(|e| format!("Stream play() failed: {e}"))?;
        log::info!("AudioCapture: recording");

        Ok(Self { _stream: stream, buffer, sample_rate })
    }

    /// Pause recording, drain the internal buffer, and return **mono 16 kHz
    /// f32 PCM** — the exact format whisper.cpp expects.
    pub fn stop_and_drain(self) -> Vec<f32> {
        // Pause before dropping so the last callback has time to finish.
        let _ = self._stream.pause();
        drop(self._stream);

        let raw = self.buffer.lock().unwrap().clone();
        log::info!(
            "AudioCapture: stopped — {}ms of audio captured",
            raw.len() * 1000 / self.sample_rate.max(1) as usize
        );

        if self.sample_rate == 16_000 {
            raw
        } else {
            linear_resample(raw, self.sample_rate, 16_000)
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Downmix interleaved multi-channel PCM to mono and append to `buf`.
fn push_mono(data: &[f32], channels: usize, buf: &Buf) {
    let mut guard = buf.lock().unwrap();
    if channels == 1 {
        guard.extend_from_slice(data);
    } else {
        guard.extend(
            data.chunks_exact(channels)
                .map(|ch| ch.iter().sum::<f32>() / channels as f32),
        );
    }
}

/// Linear (lerp) resampler — quality is adequate for speech-band audio.
///
/// Whisper expects 16 kHz input.  Most desktop devices report 44.1 or 48 kHz;
/// this converts on the fly without pulling in a heavy DSP crate.
pub fn linear_resample(input: Vec<f32>, from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || input.is_empty() {
        return input;
    }
    let ratio      = from_hz as f64 / to_hz as f64;
    let output_len = ((input.len() as f64) / ratio).ceil() as usize;
    let last       = input.len() - 1;
    (0..output_len)
        .map(|i| {
            let src  = i as f64 * ratio;
            let lo   = src.floor() as usize;
            let hi   = (lo + 1).min(last);
            let frac = (src - src.floor()) as f32;
            input[lo] * (1.0 - frac) + input[hi] * frac
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::linear_resample;

    #[test]
    fn resample_identity() {
        let v = vec![0.1, 0.2, 0.3];
        assert_eq!(linear_resample(v.clone(), 16_000, 16_000), v);
    }

    #[test]
    fn resample_down_length() {
        // 48 kHz → 16 kHz should produce ~1/3 the number of samples
        let input: Vec<f32> = (0..480).map(|i| i as f32).collect();
        let output = linear_resample(input, 48_000, 16_000);
        assert!((output.len() as i32 - 160).abs() <= 2);
    }
}

