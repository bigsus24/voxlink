use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use crossbeam_channel::Receiver;
use std::sync::Arc;
use parking_lot::Mutex;
use crate::config;

/// Error type for audio playback operations
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("No output device available")]
    NoOutputDevice,

    #[error("Failed to build output stream: {0}")]
    BuildStreamFailed(String),

    #[error("Failed to start stream: {0}")]
    PlayFailed(String),

    #[error("Device error: {0}")]
    DeviceError(String),
}

/// Plays audio to the speaker using cpal.
///
/// Consumes PCM sample buffers from a crossbeam channel and writes
/// them to the audio output device. Uses a ring buffer to handle
/// timing differences between the audio callback and data arrival.
pub struct AudioPlayback {
    stream: Option<cpal::Stream>,
    is_active: Arc<Mutex<bool>>,
}

impl AudioPlayback {
    /// Create and start an audio playback stream.
    /// The receiver should provide mixed PCM samples (f32, mono, 48kHz).
    pub fn start(rx: Receiver<Vec<f32>>) -> Result<Self, PlaybackError> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or(PlaybackError::NoOutputDevice)?;

        tracing::info!("Using output device: {:?}", device.name().unwrap_or_default());

        let stream_config = StreamConfig {
            channels: config::CHANNELS,
            sample_rate: cpal::SampleRate(config::SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        // Shared playback buffer (ring buffer approach)
        let playback_buffer: Arc<Mutex<Vec<f32>>> =
            Arc::new(Mutex::new(Vec::with_capacity(config::SAMPLES_PER_FRAME * 4)));
        let playback_buffer_clone = playback_buffer.clone();
        let is_active = Arc::new(Mutex::new(true));
        let is_active_clone = is_active.clone();

        // Separate thread to drain the channel into the playback buffer
        std::thread::spawn(move || {
            while let Ok(samples) = rx.recv() {
                let mut buf = playback_buffer_clone.lock();
                buf.extend_from_slice(&samples);
                // Keep buffer from growing unbounded (max ~200ms of audio)
                let max_samples = config::SAMPLES_PER_FRAME * 10;
                if buf.len() > max_samples {
                    let drain_count = buf.len() - max_samples;
                    buf.drain(..drain_count);
                }
            }
        });

        let stream = device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if !*is_active_clone.lock() {
                    // Output silence when paused
                    data.fill(0.0);
                    return;
                }

                let mut buf = playback_buffer.lock();
                let available = buf.len().min(data.len());

                if available > 0 {
                    data[..available].copy_from_slice(&buf[..available]);
                    buf.drain(..available);
                }
                // Fill remaining with silence
                if available < data.len() {
                    data[available..].fill(0.0);
                }
            },
            move |err| {
                tracing::error!("Audio playback error: {}", err);
            },
            None,
        ).map_err(|e| PlaybackError::BuildStreamFailed(e.to_string()))?;

        stream.play()
            .map_err(|e| PlaybackError::PlayFailed(e.to_string()))?;

        Ok(Self {
            stream: Some(stream),
            is_active,
        })
    }

    /// Pause playback (output silence)
    pub fn pause(&self) {
        *self.is_active.lock() = false;
    }

    /// Resume playback
    pub fn resume(&self) {
        *self.is_active.lock() = true;
    }

    /// Check if playback is active
    pub fn is_active(&self) -> bool {
        *self.is_active.lock()
    }

    /// List available output devices
    pub fn list_devices() -> Result<Vec<String>, PlaybackError> {
        let host = cpal::default_host();
        let devices = host.output_devices()
            .map_err(|e| PlaybackError::DeviceError(e.to_string()))?;

        Ok(devices
            .filter_map(|d| d.name().ok())
            .collect())
    }
}

impl Drop for AudioPlayback {
    fn drop(&mut self) {
        *self.is_active.lock() = false;
        self.stream.take();
    }
}
