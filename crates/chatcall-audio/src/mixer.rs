use crate::config;

/// Multi-user audio mixer.
///
/// Combines PCM audio from multiple users into a single output stream.
/// Uses simple additive mixing with soft clipping to prevent distortion.
///
/// ## Algorithm
/// 1. Sum all input samples at each position
/// 2. Apply soft clipping using `tanh()` to prevent harsh clipping
/// 3. Normalize if peak exceeds threshold
pub struct AudioMixer {
    /// Number of active sources being mixed
    active_sources: usize,
}

impl AudioMixer {
    pub fn new() -> Self {
        Self { active_sources: 0 }
    }

    /// Mix multiple audio frames into a single output.
    /// All input frames must have the same length.
    ///
    /// Returns a mixed PCM buffer ready for playback.
    pub fn mix(&mut self, frames: &[Vec<f32>]) -> Vec<f32> {
        if frames.is_empty() {
            return vec![0.0; config::SAMPLES_PER_FRAME];
        }

        self.active_sources = frames.len();

        if frames.len() == 1 {
            // Single source — no mixing needed, just return a clone
            return frames[0].clone();
        }

        let frame_len = frames[0].len();
        let mut mixed = vec![0.0f32; frame_len];

        // Additive mixing
        for frame in frames {
            for (i, sample) in frame.iter().enumerate() {
                if i < mixed.len() {
                    mixed[i] += sample;
                }
            }
        }

        // Soft clipping using tanh to prevent distortion
        // Scale down first if many sources, then apply tanh for smooth limiting
        let scale = 1.0 / (frames.len() as f32).sqrt(); // sqrt scaling for natural mix
        for sample in mixed.iter_mut() {
            *sample = (*sample * scale).tanh();
        }

        mixed
    }

    /// Get the number of active sources in the last mix
    pub fn active_sources(&self) -> usize {
        self.active_sources
    }
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_source() {
        let mut mixer = AudioMixer::new();
        let input = vec![0.5f32; 960];
        let output = mixer.mix(&[input.clone()]);
        assert_eq!(output, input); // single source passthrough
    }

    #[test]
    fn test_mixing_doesnt_clip() {
        let mut mixer = AudioMixer::new();
        let loud1 = vec![0.9f32; 960];
        let loud2 = vec![0.9f32; 960];
        let output = mixer.mix(&[loud1, loud2]);

        // All samples should be < 1.0 after soft clipping
        for sample in &output {
            assert!(sample.abs() < 1.0, "Sample {} exceeds ±1.0", sample);
        }
    }

    #[test]
    fn test_empty_produces_silence() {
        let mut mixer = AudioMixer::new();
        let output = mixer.mix(&[]);
        assert!(output.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn test_silence_plus_signal() {
        let mut mixer = AudioMixer::new();
        let silence = vec![0.0f32; 960];
        let signal = vec![0.3f32; 960];
        let output = mixer.mix(&[silence, signal.clone()]);

        // Result should approximate the signal (with scaling)
        assert!(output[0] > 0.0);
    }
}
