//! Fixed-size ring buffer for audio samples.
//!
//! Stores 16-bit PCM samples in a circular buffer. Pre-buffers
//! up to `pre_buffer_ms` ms before speech onset to avoid clipping
//! the start of utterances.
//!
//! Not yet wired to the pipeline — reserved for v0.5 VAD-driven mode.
#![allow(dead_code)]

/// A simple ring buffer for `f32` audio samples.
pub struct RingBuffer {
    buf:    Vec<f32>,
    head:   usize,
    len:    usize,
}

impl RingBuffer {
    /// Create a buffer that holds `capacity` samples.
    pub fn new(capacity: usize) -> Self {
        Self {
            buf:  vec![0.0; capacity],
            head: 0,
            len:  0,
        }
    }

    /// Push one sample, overwriting the oldest if the buffer is full.
    pub fn push(&mut self, sample: f32) {
        let cap = self.buf.len();
        self.buf[self.head % cap] = sample;
        self.head = (self.head + 1) % cap;
        if self.len < cap {
            self.len += 1;
        }
    }

    /// Drain the buffer in chronological order.
    pub fn drain(&mut self) -> Vec<f32> {
        let cap = self.buf.len();
        let start = if self.len < cap {
            0
        } else {
            self.head // head wraps to oldest slot
        };

        let mut out = Vec::with_capacity(self.len);
        for i in 0..self.len {
            out.push(self.buf[(start + i) % cap]);
        }
        self.head = 0;
        self.len  = 0;
        out
    }

    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }
}

#[cfg(test)]
mod tests {
    use super::RingBuffer;

    #[test]
    fn push_and_drain() {
        let mut rb = RingBuffer::new(4);
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);
        let out = rb.drain();
        assert_eq!(out, vec![1.0, 2.0, 3.0]);
        assert!(rb.is_empty());
    }

    #[test]
    fn overwrites_oldest_when_full() {
        let mut rb = RingBuffer::new(3);
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);
        rb.push(4.0); // overwrites 1.0
        let out = rb.drain();
        assert_eq!(out, vec![2.0, 3.0, 4.0]);
    }
}
