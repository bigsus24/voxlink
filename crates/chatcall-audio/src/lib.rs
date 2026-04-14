//! # ChatCall Audio Library
//!
//! Custom audio pipeline for voice communication including:
//! - Microphone capture (cpal)
//! - Opus encoding/decoding
//! - Adaptive jitter buffer
//! - Voice activity detection
//! - Multi-user audio mixing
//! - Full send/receive pipeline orchestration

pub mod capture;
pub mod playback;
pub mod encoder;
pub mod decoder;
pub mod jitter_buffer;
pub mod mixer;
pub mod vad;
pub mod pipeline;

/// Audio configuration constants
pub mod config {
    /// Sample rate in Hz (Opus native rate)
    pub const SAMPLE_RATE: u32 = 48_000;
    /// Number of audio channels (mono for voice)
    pub const CHANNELS: u16 = 1;
    /// Frame duration in milliseconds
    pub const FRAME_DURATION_MS: u32 = 20;
    /// Number of samples per frame (48000 * 0.020)
    pub const SAMPLES_PER_FRAME: usize = 960;
    /// Opus bitrate for voice in bits/second
    pub const OPUS_BITRATE: i32 = 32_000;
    /// Maximum encoded frame size in bytes
    pub const MAX_ENCODED_SIZE: usize = 256;
    /// Jitter buffer initial depth in frames
    pub const JITTER_BUFFER_INITIAL_DEPTH: usize = 3;
    /// Jitter buffer maximum depth in frames
    pub const JITTER_BUFFER_MAX_DEPTH: usize = 8;
    /// VAD energy threshold (RMS below this = silence)
    pub const VAD_THRESHOLD: f32 = 0.01;
    /// VAD silence duration before triggering (in frames)
    pub const VAD_SILENCE_FRAMES: usize = 15; // 300ms at 20ms/frame
}

/// A single audio frame (20ms of PCM samples)
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// PCM samples (f32, mono, 48kHz)
    pub samples: Vec<f32>,
    /// Timestamp in milliseconds
    pub timestamp: u32,
}

impl AudioFrame {
    pub fn new(samples: Vec<f32>, timestamp: u32) -> Self {
        Self { samples, timestamp }
    }

    /// Create a silent frame
    pub fn silence(timestamp: u32) -> Self {
        Self {
            samples: vec![0.0; config::SAMPLES_PER_FRAME],
            timestamp,
        }
    }

    /// Get frame duration in milliseconds
    pub fn duration_ms(&self) -> u32 {
        (self.samples.len() as u32 * 1000) / config::SAMPLE_RATE
    }
}

/// Encoded audio frame ready for network transmission
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    /// Opus-encoded bytes
    pub data: Vec<u8>,
    /// Sequence number
    pub sequence: u32,
    /// Timestamp in milliseconds
    pub timestamp: u32,
    /// Whether this frame is silence (no opus data)
    pub is_silence: bool,
}
