use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks pending acknowledgements for reliable message delivery.
///
/// When a chat message is sent, it's registered here with its UUID.
/// If no ACK is received within the timeout, the message is flagged
/// for retransmission. After max retries, it's marked as failed.
pub struct AckTracker {
    /// Pending messages awaiting ACK: message_id -> tracking info
    pending: HashMap<[u8; 16], PendingMessage>,
    /// Timeout before retransmission
    timeout: Duration,
    /// Maximum number of retransmission attempts
    max_retries: u32,
}

/// Tracking info for a pending message
#[derive(Debug, Clone)]
struct PendingMessage {
    /// Raw packet bytes for retransmission
    packet_bytes: Vec<u8>,
    /// When the message was last sent
    last_sent: Instant,
    /// Number of times this message has been retransmitted
    retry_count: u32,
    /// When the message was originally sent
    created_at: Instant,
}

/// Result of checking pending messages
#[derive(Debug)]
pub enum AckStatus {
    /// Message was acknowledged successfully
    Acknowledged,
    /// Message needs to be retransmitted (returns packet bytes)
    NeedsRetransmit(Vec<u8>),
    /// Message exceeded max retries — delivery failed
    Failed { message_id: [u8; 16] },
    /// Message is still within timeout, waiting for ACK
    Waiting,
}

impl AckTracker {
    /// Create a new ACK tracker with default settings (2s timeout, 3 retries)
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            timeout: Duration::from_secs(2),
            max_retries: 3,
        }
    }

    /// Create with custom timeout and retry count
    pub fn with_config(timeout: Duration, max_retries: u32) -> Self {
        Self {
            pending: HashMap::new(),
            timeout,
            max_retries,
        }
    }

    /// Register a message as pending ACK
    pub fn register(&mut self, message_id: [u8; 16], packet_bytes: Vec<u8>) {
        let now = Instant::now();
        self.pending.insert(message_id, PendingMessage {
            packet_bytes,
            last_sent: now,
            retry_count: 0,
            created_at: now,
        });
    }

    /// Mark a message as acknowledged. Returns true if the message was pending.
    pub fn acknowledge(&mut self, message_id: &[u8; 16]) -> bool {
        self.pending.remove(message_id).is_some()
    }

    /// Check all pending messages and return any that need retransmission or have failed.
    pub fn check_timeouts(&mut self) -> Vec<AckStatus> {
        let now = Instant::now();
        let mut results = Vec::new();
        let mut to_remove = Vec::new();

        for (msg_id, pending) in self.pending.iter_mut() {
            if now.duration_since(pending.last_sent) >= self.timeout {
                if pending.retry_count >= self.max_retries {
                    results.push(AckStatus::Failed { message_id: *msg_id });
                    to_remove.push(*msg_id);
                } else {
                    pending.retry_count += 1;
                    pending.last_sent = now;
                    results.push(AckStatus::NeedsRetransmit(pending.packet_bytes.clone()));
                }
            }
        }

        for id in to_remove {
            self.pending.remove(&id);
        }

        results
    }

    /// Number of messages pending ACK
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if a specific message is still pending
    pub fn is_pending(&self, message_id: &[u8; 16]) -> bool {
        self.pending.contains_key(message_id)
    }

    /// Clear all pending messages
    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

impl Default for AckTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_register_and_acknowledge() {
        let mut tracker = AckTracker::new();
        let msg_id = [0xAA; 16];
        tracker.register(msg_id, vec![1, 2, 3]);
        assert_eq!(tracker.pending_count(), 1);
        assert!(tracker.is_pending(&msg_id));

        assert!(tracker.acknowledge(&msg_id));
        assert_eq!(tracker.pending_count(), 0);
        assert!(!tracker.is_pending(&msg_id));
    }

    #[test]
    fn test_acknowledge_unknown_returns_false() {
        let mut tracker = AckTracker::new();
        assert!(!tracker.acknowledge(&[0xFF; 16]));
    }

    #[test]
    fn test_timeout_triggers_retransmit() {
        let mut tracker = AckTracker::with_config(Duration::from_millis(50), 3);
        let msg_id = [0xBB; 16];
        tracker.register(msg_id, vec![4, 5, 6]);

        // Before timeout — no retransmits
        let results = tracker.check_timeouts();
        assert!(results.is_empty());

        // Wait past timeout
        thread::sleep(Duration::from_millis(60));

        let results = tracker.check_timeouts();
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], AckStatus::NeedsRetransmit(data) if data == &[4, 5, 6]));
    }

    #[test]
    fn test_max_retries_triggers_failure() {
        let mut tracker = AckTracker::with_config(Duration::from_millis(10), 2);
        let msg_id = [0xCC; 16];
        tracker.register(msg_id, vec![7, 8, 9]);

        // Exhaust retries
        for _ in 0..2 {
            thread::sleep(Duration::from_millis(15));
            tracker.check_timeouts();
        }

        thread::sleep(Duration::from_millis(15));
        let results = tracker.check_timeouts();
        assert!(results.iter().any(|r| matches!(r, AckStatus::Failed { .. })));
        assert_eq!(tracker.pending_count(), 0);
    }
}
