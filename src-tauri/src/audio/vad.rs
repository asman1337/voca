//! Energy-based Voice Activity Detection (VAD).
//!
//! Splits a continuous audio stream into discrete speech segments by watching
//! the RMS energy of 30 ms frames.  No ONNX model or external process needed;
//! pure Rust and fast enough to run in the CPAL callback thread.
//!
//! # Algorithm
//! 1. Incoming samples are buffered and processed in `frame_size` chunks.
//! 2. Each frame's RMS energy is compared to configurable thresholds.
//! 3. `speech_hold_frames` consecutive energetic frames confirm speech start.
//! 4. `silence_hold_frames` consecutive quiet frames confirm speech end.
//! 5. A pre-roll window ensures the first syllable is never clipped.
//!
//! When speech ends, a [`SpeechSegment`] is returned — ready for the STT
//! engine.  At end-of-stream call [`EnergyVad::flush`] to collect any
//! segment that was still in progress.
//!
//! # Future
//! This module can be swapped for Silero VAD (ONNX) in v0.5 without changing
//! the public API — only the internal `EnergyVad` implementation changes.

// ── Public types ─────────────────────────────────────────────────────────────

/// A completed speech utterance, ready to send to the STT engine.
#[derive(Debug)]
pub struct SpeechSegment {
    /// Mono 16 kHz f32 PCM samples.
    pub audio: Vec<f32>,
}

/// Tuning parameters for the VAD.
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// Sample rate of the audio being fed in (Hz). Usually 16 000.
    pub sample_rate: u32,
    /// RMS energy that triggers speech-start detection (0.0 – 1.0).
    pub speech_threshold: f32,
    /// RMS energy below which a frame counts as silence (0.0 – 1.0).
    pub silence_threshold: f32,
    /// How many consecutive speech frames are required before declaring
    /// speech has started (~90 ms at default settings).
    pub speech_hold_frames: usize,
    /// How many consecutive silent frames are required before declaring
    /// speech has ended (~600 ms at default settings).
    pub silence_hold_frames: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            sample_rate:         16_000,
            speech_threshold:    0.010,
            silence_threshold:   0.005,
            speech_hold_frames:  3,   // 3 × 30 ms = 90 ms onset confirmation
            silence_hold_frames: 20,  // 20 × 30 ms = 600 ms trailing silence
        }
    }
}

// ── EnergyVad ────────────────────────────────────────────────────────────────

/// Stateful VAD that accumulates samples and emits completed speech segments.
pub struct EnergyVad {
    cfg:        VadConfig,
    frame_size: usize,       // samples per 30 ms frame

    // Sample bookkeeping
    leftover:   Vec<f32>,    // partial frame waiting for more samples
    pre_roll:   Vec<f32>,    // recent frames before speech confirmed

    // State machine
    in_speech:     bool,
    speech_hold:   usize,    // consecutive energetic frames seen
    silence_hold:  usize,    // consecutive quiet frames seen (while speaking)
    speech_buf:    Vec<f32>, // accumulating speech samples
}

impl EnergyVad {
    /// Create a new VAD with the given config.
    pub fn new(cfg: VadConfig) -> Self {
        // 30 ms frame at the configured sample rate
        let frame_size = (cfg.sample_rate as usize * 30) / 1_000;
        Self {
            cfg,
            frame_size,
            leftover:     Vec::new(),
            pre_roll:     Vec::new(),
            in_speech:    false,
            speech_hold:  0,
            silence_hold: 0,
            speech_buf:   Vec::new(),
        }
    }

    /// Feed a chunk of audio samples.
    ///
    /// Returns any speech segments that were completed during this call.
    /// In normal use this returns `[]` most of the time and one element
    /// at the end of each utterance.
    pub fn push(&mut self, samples: &[f32]) -> Vec<SpeechSegment> {
        self.leftover.extend_from_slice(samples);
        let mut completed = Vec::new();

        while self.leftover.len() >= self.frame_size {
            let frame: Vec<f32> = self.leftover.drain(..self.frame_size).collect();
            let energy = rms(&frame);

            if self.in_speech {
                self.speech_buf.extend_from_slice(&frame);

                if energy < self.cfg.silence_threshold {
                    self.silence_hold += 1;
                    if self.silence_hold >= self.cfg.silence_hold_frames {
                        // ── Speech ended ───────────────────────────────────
                        self.in_speech   = false;
                        self.silence_hold = 0;
                        self.speech_hold  = 0;
                        let audio = std::mem::take(&mut self.speech_buf);
                        self.pre_roll.clear();
                        completed.push(SpeechSegment { audio });
                    }
                } else {
                    self.silence_hold = 0; // reset on energetic frame
                }
            } else {
                // ── Not yet in speech — maintain pre-roll window ──────────
                self.pre_roll.extend_from_slice(&frame);
                // Keep only the last `speech_hold_frames * 2` frames in pre-roll
                let keep = self.cfg.speech_hold_frames * self.frame_size * 2;
                if self.pre_roll.len() > keep {
                    self.pre_roll.drain(..self.pre_roll.len() - keep);
                }

                if energy >= self.cfg.speech_threshold {
                    self.speech_hold += 1;
                    if self.speech_hold >= self.cfg.speech_hold_frames {
                        // ── Speech started — flush pre-roll into buffer ────
                        self.in_speech   = true;
                        self.silence_hold = 0;
                        let pre = std::mem::take(&mut self.pre_roll);
                        self.speech_buf  = pre;
                        self.speech_buf.extend_from_slice(&frame);
                    }
                } else {
                    self.speech_hold = 0;
                }
            }
        }

        completed
    }

    /// Flush any in-progress speech segment (call this at end-of-stream).
    ///
    /// Returns the segment if one was being accumulated, `None` otherwise.
    pub fn flush(&mut self) -> Option<SpeechSegment> {
        if self.in_speech && !self.speech_buf.is_empty() {
            self.in_speech = false;
            Some(SpeechSegment {
                audio: std::mem::take(&mut self.speech_buf),
            })
        } else {
            None
        }
    }

    /// Reset all internal state (useful when the recording session changes).
    pub fn reset(&mut self) {
        self.leftover.clear();
        self.pre_roll.clear();
        self.speech_buf.clear();
        self.in_speech   = false;
        self.speech_hold  = 0;
        self.silence_hold = 0;
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Root-mean-square energy of a sample slice.
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mean_sq = samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32;
    mean_sq.sqrt()
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn silence(n: usize) -> Vec<f32> { vec![0.0f32; n] }
    fn tone(n: usize) -> Vec<f32>    { vec![0.5f32; n] } // RMS 0.5 >> threshold

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(rms(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn rms_of_constant_is_same() {
        let v = vec![0.5f32; 16];
        let r = rms(&v);
        assert!((r - 0.5).abs() < 1e-6, "expected 0.5, got {r}");
    }

    #[test]
    fn silence_produces_no_segments() {
        let cfg = VadConfig { sample_rate: 16_000, ..Default::default() };
        let mut vad = EnergyVad::new(cfg.clone());
        // 2 seconds of silence
        let segs = vad.push(&silence(cfg.sample_rate as usize * 2));
        assert!(segs.is_empty());
        assert!(vad.flush().is_none());
    }

    #[test]
    fn speech_followed_by_silence_yields_one_segment() {
        let cfg = VadConfig {
            sample_rate:         16_000,
            speech_hold_frames:  1,
            silence_hold_frames: 2,
            ..Default::default()
        };
        let mut vad = EnergyVad::new(cfg.clone());

        // 300 ms of speech (10 frames × 30 ms) then 200 ms of silence (7 frames)
        let speech_samples  = tone(cfg.sample_rate as usize / 10 * 3);  // 300 ms
        let silence_samples = silence(cfg.sample_rate as usize / 10 * 2); // 200 ms

        let mut segs = vad.push(&speech_samples);
        segs.extend(vad.push(&silence_samples));

        assert_eq!(segs.len(), 1, "expected exactly one speech segment");
        assert!(!segs[0].audio.is_empty());
    }

    #[test]
    fn flush_returns_in_progress_segment() {
        let cfg = VadConfig {
            sample_rate:        16_000,
            speech_hold_frames: 1,
            ..Default::default()
        };
        let mut vad = EnergyVad::new(cfg.clone());
        // Feed only speech, never silence
        let _segs = vad.push(&tone(cfg.sample_rate as usize / 2)); // 500 ms
        let seg = vad.flush();
        assert!(seg.is_some(), "flush should return the in-progress segment");
    }
}
