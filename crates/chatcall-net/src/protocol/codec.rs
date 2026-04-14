use bytes::BytesMut;
use crate::protocol::packet::{PacketHeader, PacketError};

/// Codec for framing packets on a TCP byte stream.
///
/// TCP is a stream protocol — packets can be split across multiple reads or
/// multiple packets can arrive in a single read. This codec handles
/// length-prefixed framing to correctly extract complete packets.
///
/// Frame format:
/// ```text
/// [PacketHeader (8 bytes)] [Payload (header.payload_length bytes)]
/// ```
pub struct PacketCodec {
    /// Accumulation buffer for incoming TCP bytes
    buffer: BytesMut,
    /// Maximum allowed payload size (prevents memory exhaustion attacks)
    max_payload_size: usize,
}

impl PacketCodec {
    /// Create a new codec with default max payload size (64KB)
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
            max_payload_size: 65536,
        }
    }

    /// Create with a custom max payload size
    pub fn with_max_payload(max_payload_size: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
            max_payload_size,
        }
    }

    /// Feed raw bytes from a TCP read into the codec
    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Try to extract the next complete packet from the buffer.
    /// Returns `None` if not enough data is available yet.
    /// Returns `Some(Ok(bytes))` with the full packet (header + payload).
    /// Returns `Some(Err(...))` if the data is malformed.
    pub fn try_decode(&mut self) -> Option<Result<Vec<u8>, PacketError>> {
        // Need at least a header
        if self.buffer.len() < PacketHeader::SIZE {
            return None;
        }

        // Parse header to find payload length
        let header = match PacketHeader::from_bytes(&self.buffer) {
            Ok(h) => h,
            Err(e) => {
                // Clear corrupted data up to the next potential magic byte
                self.skip_to_next_magic();
                return Some(Err(e));
            }
        };

        let payload_len = header.payload_length as usize;

        // Guard against oversized payloads
        if payload_len > self.max_payload_size {
            self.skip_to_next_magic();
            return Some(Err(PacketError::TooShort {
                expected: self.max_payload_size,
                got: payload_len,
            }));
        }

        let total_len = PacketHeader::SIZE + payload_len;

        // Wait for the full packet
        if self.buffer.len() < total_len {
            return None;
        }

        // Extract the full packet
        let packet_bytes = self.buffer.split_to(total_len).to_vec();
        Some(Ok(packet_bytes))
    }

    /// Skip past corrupted data to the next potential packet boundary
    fn skip_to_next_magic(&mut self) {
        if self.buffer.len() <= 1 {
            self.buffer.clear();
            return;
        }
        // Search for magic bytes starting from position 1
        let magic = crate::MAGIC;
        for i in 1..self.buffer.len() - 1 {
            if self.buffer[i] == magic[0] && self.buffer[i + 1] == magic[1] {
                let _ = self.buffer.split_to(i);
                return;
            }
        }
        // No magic found, clear everything
        self.buffer.clear();
    }

    /// Encode a packet (header + payload) into a framed byte buffer ready for TCP write
    pub fn encode(packet_bytes: &[u8]) -> Vec<u8> {
        // For TCP, we just send the raw packet bytes — the header already
        // contains the length prefix. This method exists for symmetry and
        // future extensibility (e.g., adding a frame checksum).
        packet_bytes.to_vec()
    }

    /// How many buffered bytes are waiting to be decoded
    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }
}

impl Default for PacketCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::PacketType;

    #[test]
    fn test_single_packet_decode() {
        let mut codec = PacketCodec::new();
        let header = PacketHeader::new(PacketType::Ping, 4);
        let mut data = header.to_bytes().to_vec();
        data.extend_from_slice(&[1, 2, 3, 4]); // payload

        codec.feed(&data);
        let result = codec.try_decode().unwrap().unwrap();
        assert_eq!(result.len(), PacketHeader::SIZE + 4);
    }

    #[test]
    fn test_partial_packet() {
        let mut codec = PacketCodec::new();
        let header = PacketHeader::new(PacketType::Ping, 100);
        let data = header.to_bytes().to_vec();

        // Feed only the header, not the payload
        codec.feed(&data);
        assert!(codec.try_decode().is_none()); // should wait for more data
    }

    #[test]
    fn test_multiple_packets() {
        let mut codec = PacketCodec::new();

        // Build two packets
        let h1 = PacketHeader::new(PacketType::Ping, 2);
        let mut p1 = h1.to_bytes().to_vec();
        p1.extend_from_slice(&[0xAA, 0xBB]);

        let h2 = PacketHeader::new(PacketType::Pong, 3);
        let mut p2 = h2.to_bytes().to_vec();
        p2.extend_from_slice(&[0xCC, 0xDD, 0xEE]);

        // Feed both at once
        codec.feed(&p1);
        codec.feed(&p2);

        let r1 = codec.try_decode().unwrap().unwrap();
        assert_eq!(r1.len(), PacketHeader::SIZE + 2);

        let r2 = codec.try_decode().unwrap().unwrap();
        assert_eq!(r2.len(), PacketHeader::SIZE + 3);

        assert!(codec.try_decode().is_none());
    }

    #[test]
    fn test_fragmented_delivery() {
        let mut codec = PacketCodec::new();
        let header = PacketHeader::new(PacketType::ChatMessage, 5);
        let mut full_packet = header.to_bytes().to_vec();
        full_packet.extend_from_slice(&[1, 2, 3, 4, 5]);

        // Feed in chunks
        codec.feed(&full_packet[..3]);
        assert!(codec.try_decode().is_none());

        codec.feed(&full_packet[3..10]);
        assert!(codec.try_decode().is_none());

        codec.feed(&full_packet[10..]);
        let result = codec.try_decode().unwrap().unwrap();
        assert_eq!(result, full_packet);
    }
}
