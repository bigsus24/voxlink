use std::collections::HashSet;

/// Ensures messages are processed in order and duplicates are rejected.
///
/// Maintains a window of recently seen message IDs (UUIDs) to detect
/// and drop duplicate deliveries (which can happen during retransmission).
/// Also tracks a sequence counter for ordering messages in chat.
pub struct MessageOrderer {
    /// Set of recently seen message IDs for deduplication
    seen_ids: HashSet<[u8; 16]>,
    /// Maximum number of IDs to track (prevents unbounded memory growth)
    max_seen: usize,
    /// Next expected sequence number for strict ordering
    next_sequence: u64,
    /// Buffer for out-of-order messages: (sequence, message_id, data)
    reorder_buffer: Vec<(u64, [u8; 16], Vec<u8>)>,
}

impl MessageOrderer {
    /// Create a new orderer with default capacity (10000 recent IDs)
    pub fn new() -> Self {
        Self {
            seen_ids: HashSet::new(),
            max_seen: 10_000,
            next_sequence: 0,
            reorder_buffer: Vec::new(),
        }
    }

    /// Check if a message ID has already been seen (duplicate detection).
    /// Returns `true` if this is a NEW message (not a duplicate).
    pub fn check_and_record(&mut self, message_id: &[u8; 16]) -> bool {
        if self.seen_ids.contains(message_id) {
            return false; // duplicate
        }

        // Evict oldest entries if at capacity
        if self.seen_ids.len() >= self.max_seen {
            // Simple eviction: clear half the set
            // In production, you'd use an LRU or time-based eviction
            self.seen_ids.clear();
        }

        self.seen_ids.insert(*message_id);
        true
    }

    /// Submit a message with a sequence number for ordered delivery.
    /// Returns any messages that are now ready to be delivered in order.
    pub fn submit_ordered(
        &mut self,
        sequence: u64,
        message_id: [u8; 16],
        data: Vec<u8>,
    ) -> Vec<(u64, [u8; 16], Vec<u8>)> {
        // Check for duplicates
        if !self.check_and_record(&message_id) {
            return Vec::new();
        }

        if sequence == self.next_sequence {
            // This is the next expected message — deliver it and check buffer
            let mut deliverable = vec![(sequence, message_id, data)];
            self.next_sequence += 1;

            // Check if buffered messages can now be delivered
            loop {
                if let Some(idx) = self.reorder_buffer.iter()
                    .position(|(seq, _, _)| *seq == self.next_sequence)
                {
                    deliverable.push(self.reorder_buffer.remove(idx));
                    self.next_sequence += 1;
                } else {
                    break;
                }
            }

            deliverable
        } else if sequence > self.next_sequence {
            // Future message — buffer it
            self.reorder_buffer.push((sequence, message_id, data));
            // Sort buffer by sequence for efficient lookup
            self.reorder_buffer.sort_by_key(|(seq, _, _)| *seq);
            Vec::new()
        } else {
            // Old message (already past our sequence window) — drop it
            Vec::new()
        }
    }

    /// Get the next expected sequence number
    pub fn next_expected_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Number of messages in the reorder buffer
    pub fn buffered_count(&self) -> usize {
        self.reorder_buffer.len()
    }
}

impl Default for MessageOrderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(val: u8) -> [u8; 16] {
        [val; 16]
    }

    #[test]
    fn test_duplicate_detection() {
        let mut orderer = MessageOrderer::new();
        let id = make_id(1);
        assert!(orderer.check_and_record(&id));
        assert!(!orderer.check_and_record(&id)); // duplicate
    }

    #[test]
    fn test_in_order_delivery() {
        let mut orderer = MessageOrderer::new();
        let r0 = orderer.submit_ordered(0, make_id(0), vec![0]);
        assert_eq!(r0.len(), 1);
        let r1 = orderer.submit_ordered(1, make_id(1), vec![1]);
        assert_eq!(r1.len(), 1);
        let r2 = orderer.submit_ordered(2, make_id(2), vec![2]);
        assert_eq!(r2.len(), 1);
    }

    #[test]
    fn test_out_of_order_reordering() {
        let mut orderer = MessageOrderer::new();

        // Receive seq 2 first (out of order)
        let r2 = orderer.submit_ordered(2, make_id(2), vec![2]);
        assert!(r2.is_empty()); // buffered, not delivered

        // Receive seq 0
        let r0 = orderer.submit_ordered(0, make_id(0), vec![0]);
        assert_eq!(r0.len(), 1); // only seq 0 delivered

        // Receive seq 1 — should flush both seq 1 and buffered seq 2
        let r1 = orderer.submit_ordered(1, make_id(1), vec![1]);
        assert_eq!(r1.len(), 2); // seq 1 + seq 2 delivered
        assert_eq!(r1[0].0, 1);
        assert_eq!(r1[1].0, 2);
    }

    #[test]
    fn test_duplicate_ordered_message() {
        let mut orderer = MessageOrderer::new();
        let r0 = orderer.submit_ordered(0, make_id(0), vec![0]);
        assert_eq!(r0.len(), 1);

        // Same message again — should be rejected
        let r0_dup = orderer.submit_ordered(0, make_id(0), vec![0]);
        assert!(r0_dup.is_empty());
    }
}
