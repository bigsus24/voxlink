use std::collections::HashMap;
use crossbeam_channel::{Sender, Receiver, bounded};
use crate::capture::AudioCapture;
use crate::playback::AudioPlayback;
use crate::encoder::{AudioEncoder, EncoderError};
use crate::decoder::{AudioDecoder, DecoderError};
use crate::jitter_buffer::JitterBuffer;
use crate::mixer::AudioMixer;
use crate::vad::VoiceActivityDetector;
use crate::{AudioFrame, EncodedFrame, config};

/// Error type for pipeline operations
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Capture error: {0}")]
    Capture(#[from] crate::capture::CaptureError),

    #[error("Playback error: {0}")]
    Playback(#[from] crate::playback::PlaybackError),

    #[error("Encoder error: {0}")]
    Encoder(#[from] EncoderError),

    #[error("Decoder error: {0}")]
    Decoder(#[from] DecoderError),

    #[error("Pipeline not started")]
    NotStarted,
}

/// Complete voice pipeline orchestrating capture → encode → send and recv → decode → play.
///
/// ## Architecture
/// ```text
/// [Mic] → [VAD] → [Encoder] → [outbound_tx] → (to network)
/// (from network) → [inbound_rx] → [JitterBuffer] → [Decoder] → [Mixer] → [Speaker]
/// ```
pub struct VoicePipeline {
    /// Encoder for outbound audio
    encoder: AudioEncoder,
    /// Per-user decoders for inbound audio
    decoders: HashMap<u16, AudioDecoder>,
    /// Per-user jitter buffers
    jitter_buffers: HashMap<u16, JitterBuffer>,
    /// Audio mixer for combining multiple users
    mixer: AudioMixer,
    /// Voice activity detector
    vad: VoiceActivityDetector,
    /// Whether the local user is muted
    is_muted: bool,
    /// Capture handle (kept alive to maintain the stream)
    capture: Option<AudioCapture>,
    /// Playback handle
    playback: Option<AudioPlayback>,
    /// Channel for captured audio frames (from mic callback)
    capture_rx: Option<Receiver<AudioFrame>>,
    /// Channel for sending mixed audio to the speaker
    playback_tx: Option<Sender<Vec<f32>>>,
}

impl VoicePipeline {
    /// Create a new voice pipeline (not yet started)
    pub fn new() -> Result<Self, PipelineError> {
        Ok(Self {
            encoder: AudioEncoder::new()?,
            decoders: HashMap::new(),
            jitter_buffers: HashMap::new(),
            mixer: AudioMixer::new(),
            vad: VoiceActivityDetector::new(),
            is_muted: false,
            capture: None,
            playback: None,
            capture_rx: None,
            playback_tx: None,
        })
    }

    /// Start the audio capture and playback streams.
    /// Returns a receiver for encoded frames ready to send over the network.
    pub fn start(&mut self) -> Result<Receiver<AudioFrame>, PipelineError> {
        // Create capture channel (bounded to prevent memory growth)
        let (capture_tx, capture_rx) = bounded::<AudioFrame>(16);
        let capture = AudioCapture::start(capture_tx)?;

        // Create playback channel
        let (playback_tx, playback_rx) = bounded::<Vec<f32>>(16);
        let playback = AudioPlayback::start(playback_rx)?;

        self.capture = Some(capture);
        self.playback = Some(playback);
        self.capture_rx = Some(capture_rx.clone());
        self.playback_tx = Some(playback_tx);

        Ok(capture_rx)
    }

    /// Process a captured audio frame: VAD check → encode.
    /// Returns `None` if the frame is silence (VAD says no voice activity).
    pub fn process_capture(&mut self, frame: &AudioFrame) -> Result<Option<EncodedFrame>, PipelineError> {
        if self.is_muted {
            return Ok(None);
        }

        let is_speech = self.vad.process(&frame.samples);

        if is_speech {
            let encoded = self.encoder.encode(&frame.samples, frame.timestamp)?;
            Ok(Some(encoded))
        } else {
            Ok(None) // Silence — don't send anything
        }
    }

    /// Receive an encoded frame from a remote user.
    /// Pushes it into the appropriate user's jitter buffer.
    pub fn receive_remote_frame(&mut self, user_id: u16, frame: EncodedFrame) {
        // Create decoder and jitter buffer for new users on first receive
        if !self.jitter_buffers.contains_key(&user_id) {
            self.jitter_buffers.insert(user_id, JitterBuffer::new());
            if let Ok(decoder) = AudioDecoder::new() {
                self.decoders.insert(user_id, decoder);
            }
        }

        if let Some(jb) = self.jitter_buffers.get_mut(&user_id) {
            jb.push(frame);
        }
    }

    /// Process one playback cycle: pop from jitter buffers → decode → mix → play.
    /// Should be called every 20ms by the pipeline timer.
    pub fn process_playback(&mut self) -> Result<(), PipelineError> {
        let mut decoded_frames: Vec<Vec<f32>> = Vec::new();

        let user_ids: Vec<u16> = self.jitter_buffers.keys().copied().collect();

        for user_id in user_ids {
            let jb = self.jitter_buffers.get_mut(&user_id).unwrap();
            let decoder = self.decoders.get_mut(&user_id);

            if let Some(decoder) = decoder {
                let timestamp = jb.depth() as u32 * config::FRAME_DURATION_MS;

                match jb.pop() {
                    Some(encoded) => {
                        if encoded.is_silence {
                            // Don't decode silence markers — just skip
                            continue;
                        }
                        match decoder.decode(&encoded.data, timestamp) {
                            Ok(audio_frame) => {
                                decoded_frames.push(audio_frame.samples);
                            }
                            Err(e) => {
                                tracing::warn!("Decode error for user {}: {}", user_id, e);
                            }
                        }
                    }
                    None => {
                        // Packet loss — use PLC
                        match decoder.decode_plc(timestamp) {
                            Ok(plc_frame) => {
                                decoded_frames.push(plc_frame.samples);
                            }
                            Err(_) => {} // ignore PLC errors
                        }
                    }
                }
            }
        }

        // Mix all decoded frames
        if !decoded_frames.is_empty() {
            let mixed = self.mixer.mix(&decoded_frames);
            if let Some(tx) = &self.playback_tx {
                let _ = tx.try_send(mixed);
            }
        }

        Ok(())
    }

    /// Set mute state
    pub fn set_muted(&mut self, muted: bool) {
        self.is_muted = muted;
        if let Some(capture) = &self.capture {
            if muted {
                capture.pause();
            } else {
                capture.resume();
            }
        }
    }

    /// Check if muted
    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    /// Remove a user's decoder and jitter buffer (when they leave)
    pub fn remove_user(&mut self, user_id: u16) {
        self.jitter_buffers.remove(&user_id);
        self.decoders.remove(&user_id);
    }

    /// Get jitter stats for a user
    pub fn user_jitter_stats(&self, user_id: u16) -> Option<(u64, u64, u64)> {
        self.jitter_buffers.get(&user_id).map(|jb| jb.stats())
    }

    /// Stop the pipeline
    pub fn stop(&mut self) {
        self.capture.take();
        self.playback.take();
        self.capture_rx.take();
        self.playback_tx.take();
    }
}

impl Default for VoicePipeline {
    fn default() -> Self {
        Self::new().expect("Failed to create default VoicePipeline")
    }
}
