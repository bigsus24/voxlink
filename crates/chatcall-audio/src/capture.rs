use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use crossbeam_channel::Sender;
use std::sync::Arc;
use parking_lot::Mutex;
use crate::config;
use crate::AudioFrame;

/// Error type for audio capture operations
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("No input device available")]
    NoInputDevice,

    #[error("No supported input config")]
    NoSupportedConfig,

    #[error("Failed to build input stream: {0}")]
    BuildStreamFailed(String),

    #[error("Failed to start stream: {0}")]
    PlayFailed(String),

    #[error("Device error: {0}")]
    DeviceError(String),
}

/// Captures audio from the microphone using cpal.
///
/// Accumulates raw PCM samples into 20ms frames (960 samples at 48kHz)
/// and sends them through a crossbeam channel for processing.
pub struct AudioCapture {
    stream: Option<cpal::Stream>,
    is_active: Arc<Mutex<bool>>,
}

impl AudioCapture {
    /// Create and start an audio capture stream.
    /// Returns the capture handle and a receiver for audio frames.
    pub fn start(tx: Sender<AudioFrame>) -> Result<Self, CaptureError> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or(CaptureError::NoInputDevice)?;

        tracing::info!("Using input device: {:?}", device.name().unwrap_or_default());

        let stream_config = StreamConfig {
            channels: config::CHANNELS,
            sample_rate: cpal::SampleRate(config::SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        // Shared state for frame accumulation
        let frame_buffer = Arc::new(Mutex::new(Vec::with_capacity(config::SAMPLES_PER_FRAME)));
        let timestamp_counter = Arc::new(Mutex::new(0u32));
        let is_active = Arc::new(Mutex::new(true));
        let is_active_clone = is_active.clone();

        let frame_buffer_clone = frame_buffer.clone();
        let timestamp_counter_clone = timestamp_counter.clone();

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !*is_active_clone.lock() {
                    return;
                }

                let mut buffer = frame_buffer_clone.lock();
                buffer.extend_from_slice(data);

                // When we have a full frame (960 samples), send it
                while buffer.len() >= config::SAMPLES_PER_FRAME {
                    let frame_samples: Vec<f32> =
                        buffer.drain(..config::SAMPLES_PER_FRAME).collect();

                    let mut ts = timestamp_counter_clone.lock();
                    let frame = AudioFrame::new(frame_samples, *ts);
                    *ts = ts.wrapping_add(config::FRAME_DURATION_MS);

                    // Non-blocking send — drop frame if channel is full
                    let _ = tx.try_send(frame);
                }
            },
            move |err| {
                tracing::error!("Audio capture error: {}", err);
            },
            None, // no timeout
        ).map_err(|e| CaptureError::BuildStreamFailed(e.to_string()))?;

        stream.play()
            .map_err(|e| CaptureError::PlayFailed(e.to_string()))?;

        Ok(Self {
            stream: Some(stream),
            is_active,
        })
    }

    /// Pause the capture (stop sending frames but keep the stream alive)
    pub fn pause(&self) {
        *self.is_active.lock() = false;
    }

    /// Resume the capture
    pub fn resume(&self) {
        *self.is_active.lock() = true;
    }

    /// Check if capture is active
    pub fn is_active(&self) -> bool {
        *self.is_active.lock()
    }

    /// List available input devices
    pub fn list_devices() -> Result<Vec<String>, CaptureError> {
        let host = cpal::default_host();
        let devices = host.input_devices()
            .map_err(|e| CaptureError::DeviceError(e.to_string()))?;

        Ok(devices
            .filter_map(|d| d.name().ok())
            .collect())
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        *self.is_active.lock() = false;
        self.stream.take(); // drop the stream to release the device
    }
}
