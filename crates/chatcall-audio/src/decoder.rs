use opus::{Decoder as OpusDecoder, Channels};
use crate::config;
use crate::AudioFrame;

/// Error type for decoder operations
#[derive(Debug, thiserror::Error)]
pub enum DecoderError {
    #[error("Opus decoder creation failed: {0}")]
    CreateFailed(String),

    #[error("Opus decoding failed: {0}")]
    DecodeFailed(String),
}

/// Opus audio decoder that converts compressed Opus frames back to PCM.
///
/// Also supports Packet Loss Concealment (PLC) — when a frame is lost,
/// calling `decode_plc()` generates interpolated audio to fill the gap,
/// which sounds better than silence.
pub struct AudioDecoder {
    decoder: OpusDecoder,
    decode_buffer: Vec<f32>,
}

impl AudioDecoder {
    /// Create a new Opus decoder
    pub fn new() -> Result<Self, DecoderError> {
        let decoder = OpusDecoder::new(
            config::SAMPLE_RATE,
            Channels::Mono,
        ).map_err(|e| DecoderError::CreateFailed(e.to_string()))?;

        Ok(Self {
            decoder,
            decode_buffer: vec![0.0f32; config::SAMPLES_PER_FRAME],
        })
    }

    /// Decode an Opus-encoded frame to PCM samples
    pub fn decode(&mut self, opus_data: &[u8], timestamp: u32) -> Result<AudioFrame, DecoderError> {
        let decoded_len = self.decoder
            .decode_float(opus_data, &mut self.decode_buffer, false)
            .map_err(|e| DecoderError::DecodeFailed(e.to_string()))?;

        Ok(AudioFrame::new(
            self.decode_buffer[..decoded_len].to_vec(),
            timestamp,
        ))
    }

    /// Generate Packet Loss Concealment audio.
    /// Call this when a frame was lost to maintain audio continuity.
    /// Opus internally generates interpolated audio that smoothly
    /// transitions from the last decoded frame.
    pub fn decode_plc(&mut self, timestamp: u32) -> Result<AudioFrame, DecoderError> {
        let decoded_len = self.decoder
            .decode_float(&[], &mut self.decode_buffer, true)
            .map_err(|e| DecoderError::DecodeFailed(e.to_string()))?;

        Ok(AudioFrame::new(
            self.decode_buffer[..decoded_len].to_vec(),
            timestamp,
        ))
    }

    /// Reset the decoder state
    pub fn reset(&mut self) -> Result<(), DecoderError> {
        self.decoder.reset_state()
            .map_err(|e| DecoderError::DecodeFailed(e.to_string()))
    }
}
