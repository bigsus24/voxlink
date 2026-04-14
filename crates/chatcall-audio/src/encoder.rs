use opus::{Encoder as OpusEncoder, Application, Channels};
use crate::config;
use crate::EncodedFrame;

/// Error type for encoder operations
#[derive(Debug, thiserror::Error)]
pub enum EncoderError {
    #[error("Opus encoder creation failed: {0}")]
    CreateFailed(String),

    #[error("Opus encoding failed: {0}")]
    EncodeFailed(String),
}

/// Opus audio encoder for compressing voice data.
///
/// Takes raw PCM frames (960 samples, 48kHz, mono) and produces
/// compact Opus-encoded byte arrays (~80 bytes per frame at 32kbps).
pub struct AudioEncoder {
    encoder: OpusEncoder,
    sequence: u32,
    encode_buffer: Vec<u8>,
}

impl AudioEncoder {
    /// Create a new Opus encoder configured for voice
    pub fn new() -> Result<Self, EncoderError> {
        let mut encoder = OpusEncoder::new(
            config::SAMPLE_RATE,
            Channels::Mono,
            Application::Voip,
        ).map_err(|e| EncoderError::CreateFailed(e.to_string()))?;

        // Set bitrate for good voice quality at low bandwidth
        encoder.set_bitrate(opus::Bitrate::Bits(config::OPUS_BITRATE))
            .map_err(|e| EncoderError::CreateFailed(e.to_string()))?;

        // Enable in-band Forward Error Correction for packet loss resilience
        encoder.set_inband_fec(true)
            .map_err(|e| EncoderError::CreateFailed(e.to_string()))?;

        // Set expected packet loss percentage (helps Opus optimize for lossy networks)
        encoder.set_packet_loss_perc(5)
            .map_err(|e| EncoderError::CreateFailed(e.to_string()))?;

        Ok(Self {
            encoder,
            sequence: 0,
            encode_buffer: vec![0u8; config::MAX_ENCODED_SIZE],
        })
    }

    /// Encode a PCM frame (f32 samples) into an Opus-compressed frame.
    ///
    /// The input must be exactly `SAMPLES_PER_FRAME` (960) samples.
    /// Returns an `EncodedFrame` with the compressed data and metadata.
    pub fn encode(&mut self, pcm: &[f32], timestamp: u32) -> Result<EncodedFrame, EncoderError> {
        let encoded_len = self.encoder.encode_float(pcm, &mut self.encode_buffer)
            .map_err(|e| EncoderError::EncodeFailed(e.to_string()))?;

        let frame = EncodedFrame {
            data: self.encode_buffer[..encoded_len].to_vec(),
            sequence: self.sequence,
            timestamp,
            is_silence: false,
        };

        self.sequence = self.sequence.wrapping_add(1);
        Ok(frame)
    }

    /// Get the current sequence number
    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    /// Reset the encoder state (e.g., after a long pause)
    pub fn reset(&mut self) -> Result<(), EncoderError> {
        self.encoder.reset_state()
            .map_err(|e| EncoderError::EncodeFailed(e.to_string()))
    }
}
