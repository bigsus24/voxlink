use crate::config;

/// Voice Activity Detector (VAD) based on signal energy.
///
/// Determines whether an audio frame contains speech or silence.
/// Uses a dual-threshold approach with hysteresis to prevent
/// rapid toggling between speech and silence states.
///
/// ## Algorithm
/// 1. Compute RMS (Root Mean Square) energy of the frame
/// 2. Compare against thresholds:
///    - If currently silent and energy > upper threshold → speech
///    - If currently speaking and energy < lower threshold for N frames → silence
/// 3. Hysteresis prevents rapid state changes
pub struct VoiceActivityDetector {
    /// Whether voice is currently detected
    is_speaking: bool,
    /// Upper threshold (transition to speech)
    upper_threshold: f32,
    /// Lower threshold (transition to silence)
    lower_threshold: f32,
    /// Number of consecutive silent frames
    silent_frames: usize,
    /// Number of silent frames before transitioning to silence state
    silence_holdoff: usize,
}

impl VoiceActivityDetector {
    /// Create with default thresholds optimized for voice
    pub fn new() -> Self {
        Self {
            is_speaking: false,
            upper_threshold: config::VAD_THRESHOLD,
            lower_threshold: config::VAD_THRESHOLD * 0.7, // hysteresis
            silent_frames: 0,
            silence_holdoff: config::VAD_SILENCE_FRAMES, // 300ms
        }
    }

    /// Create with custom thresholds
    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            is_speaking: false,
            upper_threshold: threshold,
            lower_threshold: threshold * 0.7,
            silent_frames: 0,
            silence_holdoff: config::VAD_SILENCE_FRAMES,
        }
    }

    /// Process a frame and return whether voice is active.
    pub fn process(&mut self, samples: &[f32]) -> bool {
        let energy = Self::compute_rms(samples);

        if self.is_speaking {
            if energy < self.lower_threshold {
                self.silent_frames += 1;
                if self.silent_frames >= self.silence_holdoff {
                    self.is_speaking = false;
                    self.silent_frames = 0;
                }
            } else {
                self.silent_frames = 0;
            }
        } else if energy > self.upper_threshold {
            self.is_speaking = true;
            self.silent_frames = 0;
        }

        self.is_speaking
    }

    /// Check if currently detecting voice (without processing a new frame)
    pub fn is_speaking(&self) -> bool {
        self.is_speaking
    }

    /// Compute RMS (Root Mean Square) energy of a sample buffer
    fn compute_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    /// Reset the detector state
    pub fn reset(&mut self) {
        self.is_speaking = false;
        self.silent_frames = 0;
    }
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_detection() {
        let mut vad = VoiceActivityDetector::new();
        let silent_frame = vec![0.0f32; 960];
        assert!(!vad.process(&silent_frame));
    }

    #[test]
    fn test_speech_detection() {
        let mut vad = VoiceActivityDetector::new();
        // Generate a loud signal
        let loud_frame: Vec<f32> = (0..960)
            .map(|i| (i as f32 * 0.1).sin() * 0.5)
            .collect();
        assert!(vad.process(&loud_frame));
    }

    #[test]
    fn test_hysteresis_prevents_toggling() {
        let mut vad = VoiceActivityDetector::new();

        // Start speaking
        let loud = vec![0.1f32; 960];
        vad.process(&loud);
        assert!(vad.is_speaking());

        // One quiet frame shouldn't immediately switch to silence
        let quiet = vec![0.001f32; 960];
        vad.process(&quiet);
        assert!(vad.is_speaking()); // still speaking due to holdoff
    }
}
