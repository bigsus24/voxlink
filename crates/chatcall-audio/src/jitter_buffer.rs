use std::collections::BTreeMap;
use crate::config;
use crate::EncodedFrame;

/// Adaptive jitter buffer for smoothing out network timing variations.
///
/// Voice packets arrive over the network with variable delays (jitter).
/// The jitter buffer holds packets for a short time to re-order them
/// and maintain a steady playback cadence.
///
/// ## Design
/// - Uses a BTreeMap sorted by sequence number for O(log n) insertion
/// - Adaptive depth: adjusts based on observed jitter
/// - Drops late packets that arrive after their playback slot
/// - Returns `None` when a packet is missing (triggers PLC)
pub struct JitterBuffer {
    /// Buffered frames sorted by sequence number
    buffer: BTreeMap<u32, EncodedFrame>,
    /// Maximum number of frames to buffer
    capacity: usize,
    /// Next sequence number expected for playback
    next_playback_seq: u32,
    /// Target buffer depth in frames (adaptive)
    target_depth: usize,
    /// Statistics for adaptive adjustment
    stats: JitterStats,
    /// Whether the buffer has been initialized (received first packet)
    initialized: bool,
}

/// Jitter statistics for adaptive depth adjustment
#[derive(Debug, Default)]
struct JitterStats {
    /// Running average of inter-arrival jitter (in ms)
    jitter_estimate: f64,
    /// Timestamp of last received packet
    last_arrival_time: Option<std::time::Instant>,
    /// Expected interval between packets (in ms)
    expected_interval: f64,
    /// Number of packets received
    packet_count: u64,
    /// Number of packets received out of order
    out_of_order_count: u64,
    /// Number of late packets dropped
    late_drop_count: u64,
}

impl JitterBuffer {
    /// Create a new jitter buffer
    pub fn new() -> Self {
        Self {
            buffer: BTreeMap::new(),
            capacity: config::JITTER_BUFFER_MAX_DEPTH,
            next_playback_seq: 0,
            target_depth: config::JITTER_BUFFER_INITIAL_DEPTH,
            stats: JitterStats {
                expected_interval: config::FRAME_DURATION_MS as f64,
                ..Default::default()
            },
            initialized: false,
        }
    }

    /// Push a received frame into the buffer.
    /// Returns `true` if the frame was accepted, `false` if it was dropped (late/duplicate).
    pub fn push(&mut self, frame: EncodedFrame) -> bool {
        let seq = frame.sequence;

        if !self.initialized {
            self.next_playback_seq = seq;
            self.initialized = true;
        }

        // Drop late packets (sequence number already played)
        if self.initialized && self.seq_before(seq, self.next_playback_seq) {
            self.stats.late_drop_count += 1;
            return false;
        }

        // Track out-of-order arrivals
        if seq != self.next_expected_arrival_seq() {
            self.stats.out_of_order_count += 1;
        }

        // Update jitter estimate
        self.update_jitter_stats();

        // Insert into buffer (BTreeMap handles ordering)
        self.buffer.insert(seq, frame);
        self.stats.packet_count += 1;

        // If buffer exceeds capacity, drop oldest
        while self.buffer.len() > self.capacity {
            self.buffer.pop_first();
            self.next_playback_seq = self.next_playback_seq.wrapping_add(1);
        }

        true
    }

    /// Pop the next frame for playback.
    ///
    /// Returns `Some(frame)` if the next expected frame is available.
    /// Returns `None` if the frame is missing (caller should use PLC).
    ///
    /// Always advances the playback sequence, so calling this at regular
    /// intervals (every 20ms) maintains the correct playback cadence.
    pub fn pop(&mut self) -> Option<EncodedFrame> {
        if !self.initialized {
            return None;
        }

        // Wait until buffer has enough depth before starting playback
        if self.buffer.len() < self.target_depth {
            // Still filling — don't pop yet (but only before first pop)
            if self.stats.packet_count <= self.target_depth as u64 {
                return None;
            }
        }

        let seq = self.next_playback_seq;
        self.next_playback_seq = self.next_playback_seq.wrapping_add(1);

        self.buffer.remove(&seq)
    }

    /// Periodically adjust buffer depth based on observed jitter.
    /// Should be called every ~1 second.
    pub fn adjust_depth(&mut self) {
        let jitter_frames = (self.stats.jitter_estimate / self.stats.expected_interval).ceil() as usize;
        let new_depth = jitter_frames
            .max(2) // minimum 2 frames (40ms)
            .min(config::JITTER_BUFFER_MAX_DEPTH);

        if new_depth != self.target_depth {
            tracing::debug!(
                "Jitter buffer depth adjusted: {} → {} frames (jitter: {:.1}ms)",
                self.target_depth, new_depth, self.stats.jitter_estimate
            );
            self.target_depth = new_depth;
        }
    }

    /// Get current buffer depth (number of frames buffered)
    pub fn depth(&self) -> usize {
        self.buffer.len()
    }

    /// Get target depth
    pub fn target_depth(&self) -> usize {
        self.target_depth
    }

    /// Get jitter estimate in milliseconds
    pub fn jitter_estimate_ms(&self) -> f64 {
        self.stats.jitter_estimate
    }

    /// Get statistics
    pub fn stats(&self) -> (u64, u64, u64) {
        (self.stats.packet_count, self.stats.out_of_order_count, self.stats.late_drop_count)
    }

    /// Reset the buffer (e.g., when a user reconnects)
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.initialized = false;
        self.stats = JitterStats {
            expected_interval: config::FRAME_DURATION_MS as f64,
            ..Default::default()
        };
        self.target_depth = config::JITTER_BUFFER_INITIAL_DEPTH;
    }

    // ── Internal helpers ──────────────────────────────────────

    /// Update jitter statistics using exponential moving average
    fn update_jitter_stats(&mut self) {
        let now = std::time::Instant::now();
        if let Some(last) = self.stats.last_arrival_time {
            let interval = now.duration_since(last).as_secs_f64() * 1000.0;
            let jitter = (interval - self.stats.expected_interval).abs();

            // Exponential moving average: α = 1/8 (same as RTP jitter calculation)
            self.stats.jitter_estimate =
                0.875 * self.stats.jitter_estimate + 0.125 * jitter;
        }
        self.stats.last_arrival_time = Some(now);
    }

    /// Check if sequence a is before sequence b (handling wrapping)
    fn seq_before(&self, a: u32, b: u32) -> bool {
        let diff = b.wrapping_sub(a);
        diff > 0 && diff < u32::MAX / 2
    }

    /// Estimate next expected arrival sequence
    fn next_expected_arrival_seq(&self) -> u32 {
        if let Some((&last_seq, _)) = self.buffer.last_key_value() {
            last_seq.wrapping_add(1)
        } else {
            self.next_playback_seq
        }
    }
}

impl Default for JitterBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(seq: u32) -> EncodedFrame {
        EncodedFrame {
            data: vec![seq as u8; 10],
            sequence: seq,
            timestamp: seq * 20,
            is_silence: false,
        }
    }

    #[test]
    fn test_in_order_push_pop() {
        let mut jb = JitterBuffer::new();
        // Push enough frames to fill target depth
        for i in 0..5 {
            assert!(jb.push(make_frame(i)));
        }

        // Pop should return in order
        for i in 0..5 {
            let frame = jb.pop().unwrap();
            assert_eq!(frame.sequence, i);
        }
    }

    #[test]
    fn test_out_of_order_reordering() {
        let mut jb = JitterBuffer::new();
        // Push out of order: 0, 2, 1, 3
        jb.push(make_frame(0));
        jb.push(make_frame(2));
        jb.push(make_frame(1));
        jb.push(make_frame(3));

        // Pop should still be in order
        assert_eq!(jb.pop().unwrap().sequence, 0);
        assert_eq!(jb.pop().unwrap().sequence, 1);
        assert_eq!(jb.pop().unwrap().sequence, 2);
        assert_eq!(jb.pop().unwrap().sequence, 3);
    }

    #[test]
    fn test_missing_packet_returns_none() {
        let mut jb = JitterBuffer::new();
        jb.push(make_frame(0));
        jb.push(make_frame(1));
        // Skip seq 2
        jb.push(make_frame(3));
        jb.push(make_frame(4));

        assert_eq!(jb.pop().unwrap().sequence, 0);
        assert_eq!(jb.pop().unwrap().sequence, 1);
        assert!(jb.pop().is_none()); // seq 2 missing → PLC
        assert_eq!(jb.pop().unwrap().sequence, 3);
    }

    #[test]
    fn test_late_packet_dropped() {
        let mut jb = JitterBuffer::new();
        for i in 0..5 {
            jb.push(make_frame(i));
        }
        // Pop first 3
        jb.pop(); jb.pop(); jb.pop();

        // Try to push seq 0 (already played) — should be dropped
        assert!(!jb.push(make_frame(0)));
    }
}
