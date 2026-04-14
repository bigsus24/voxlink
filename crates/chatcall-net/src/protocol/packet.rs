use serde::{Serialize, Deserialize};
use crate::protocol::types::PacketType;
use crate::{MAGIC, PROTOCOL_VERSION};

/// Common header for all ChatCall protocol packets (8 bytes).
///
/// ```text
/// ┌─────────────────────────────────────────────────────┐
/// │ Magic (2B) │ Version (1B) │ Type (1B) │ Length (4B) │
/// └─────────────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    pub magic: [u8; 2],
    pub version: u8,
    pub packet_type: PacketType,
    pub payload_length: u32,
}

impl PacketHeader {
    /// Header size in bytes
    pub const SIZE: usize = 8;

    /// Create a new header for the given packet type and payload length
    pub fn new(packet_type: PacketType, payload_length: u32) -> Self {
        Self {
            magic: MAGIC,
            version: PROTOCOL_VERSION,
            packet_type,
            payload_length,
        }
    }

    /// Serialize header to bytes (8 bytes, little-endian)
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0] = self.magic[0];
        buf[1] = self.magic[1];
        buf[2] = self.version;
        buf[3] = self.packet_type as u8;
        buf[4..8].copy_from_slice(&self.payload_length.to_le_bytes());
        buf
    }

    /// Deserialize header from bytes
    pub fn from_bytes(buf: &[u8]) -> Result<Self, PacketError> {
        if buf.len() < Self::SIZE {
            return Err(PacketError::TooShort {
                expected: Self::SIZE,
                got: buf.len(),
            });
        }
        if buf[0] != MAGIC[0] || buf[1] != MAGIC[1] {
            return Err(PacketError::InvalidMagic {
                got: [buf[0], buf[1]],
            });
        }
        let version = buf[2];
        if version != PROTOCOL_VERSION {
            return Err(PacketError::UnsupportedVersion { got: version });
        }
        let packet_type = PacketType::from_u8(buf[3])
            .ok_or(PacketError::UnknownPacketType { got: buf[3] })?;
        let payload_length = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);

        Ok(Self {
            magic: MAGIC,
            version,
            packet_type,
            payload_length,
        })
    }
}

/// Voice frame packet sent over UDP.
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────────────┐
/// │ Header (8B) │ SeqNum (4B) │ Timestamp (4B) │ UserID (2B) │ Payload (n) │
/// └──────────────────────────────────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone)]
pub struct VoicePacket {
    pub header: PacketHeader,
    pub sequence: u32,
    pub timestamp: u32,
    pub user_id: u16,
    pub payload: Vec<u8>, // encrypted Opus data
}

impl VoicePacket {
    /// Metadata size beyond the header: seq(4) + ts(4) + uid(2) = 10
    pub const META_SIZE: usize = 10;

    pub fn new(sequence: u32, timestamp: u32, user_id: u16, payload: Vec<u8>) -> Self {
        let payload_length = (Self::META_SIZE + payload.len()) as u32;
        Self {
            header: PacketHeader::new(PacketType::VoiceFrame, payload_length),
            sequence,
            timestamp,
            user_id,
            payload,
        }
    }

    /// Create a silence packet (no audio payload)
    pub fn silence(sequence: u32, timestamp: u32, user_id: u16) -> Self {
        Self {
            header: PacketHeader::new(PacketType::VoiceSilence, Self::META_SIZE as u32),
            sequence,
            timestamp,
            user_id,
            payload: Vec::new(),
        }
    }

    /// Serialize to bytes for UDP transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let total = PacketHeader::SIZE + Self::META_SIZE + self.payload.len();
        let mut buf = Vec::with_capacity(total);
        buf.extend_from_slice(&self.header.to_bytes());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.user_id.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Deserialize from raw UDP bytes
    pub fn from_bytes(buf: &[u8]) -> Result<Self, PacketError> {
        let header = PacketHeader::from_bytes(buf)?;
        let min_size = PacketHeader::SIZE + Self::META_SIZE;
        if buf.len() < min_size {
            return Err(PacketError::TooShort {
                expected: min_size,
                got: buf.len(),
            });
        }
        let offset = PacketHeader::SIZE;
        let sequence = u32::from_le_bytes([
            buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3],
        ]);
        let timestamp = u32::from_le_bytes([
            buf[offset + 4], buf[offset + 5], buf[offset + 6], buf[offset + 7],
        ]);
        let user_id = u16::from_le_bytes([buf[offset + 8], buf[offset + 9]]);
        let payload = buf[offset + Self::META_SIZE..].to_vec();

        Ok(Self {
            header,
            sequence,
            timestamp,
            user_id,
            payload,
        })
    }
}

/// Chat message packet sent over TCP (reliable delivery).
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────────────────────┐
/// │ Header (8B) │ MsgID (16B) │ UserID (2B) │ Timestamp (8B) │ Payload (n)      │
/// └───────────────────────────────────────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone)]
pub struct ChatPacket {
    pub header: PacketHeader,
    pub message_id: [u8; 16], // UUID bytes
    pub user_id: u16,
    pub timestamp: u64,       // Unix timestamp millis
    pub payload: Vec<u8>,     // encrypted text
}

impl ChatPacket {
    /// Metadata size: msg_id(16) + uid(2) + ts(8) = 26
    pub const META_SIZE: usize = 26;

    pub fn new(message_id: [u8; 16], user_id: u16, timestamp: u64, payload: Vec<u8>) -> Self {
        let payload_length = (Self::META_SIZE + payload.len()) as u32;
        Self {
            header: PacketHeader::new(PacketType::ChatMessage, payload_length),
            message_id,
            user_id,
            timestamp,
            payload,
        }
    }

    /// Serialize to bytes for TCP transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let total = PacketHeader::SIZE + Self::META_SIZE + self.payload.len();
        let mut buf = Vec::with_capacity(total);
        buf.extend_from_slice(&self.header.to_bytes());
        buf.extend_from_slice(&self.message_id);
        buf.extend_from_slice(&self.user_id.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Deserialize from raw TCP bytes (after header has been parsed)
    pub fn from_bytes(buf: &[u8]) -> Result<Self, PacketError> {
        let header = PacketHeader::from_bytes(buf)?;
        let min_size = PacketHeader::SIZE + Self::META_SIZE;
        if buf.len() < min_size {
            return Err(PacketError::TooShort {
                expected: min_size,
                got: buf.len(),
            });
        }
        let offset = PacketHeader::SIZE;
        let mut message_id = [0u8; 16];
        message_id.copy_from_slice(&buf[offset..offset + 16]);
        let user_id = u16::from_le_bytes([buf[offset + 16], buf[offset + 17]]);
        let timestamp = u64::from_le_bytes([
            buf[offset + 18], buf[offset + 19], buf[offset + 20], buf[offset + 21],
            buf[offset + 22], buf[offset + 23], buf[offset + 24], buf[offset + 25],
        ]);
        let payload = buf[offset + Self::META_SIZE..].to_vec();

        Ok(Self {
            header,
            message_id,
            user_id,
            timestamp,
            payload,
        })
    }
}

/// Generic control packet for handshake, room management, keepalive, etc.
/// Uses bincode-serialized payload for flexible structured data.
#[derive(Debug, Clone)]
pub struct ControlPacket {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl ControlPacket {
    /// Create a control packet with a serializable payload
    pub fn new<T: Serialize>(packet_type: PacketType, data: &T) -> Result<Self, PacketError> {
        let payload = bincode::serialize(data)
            .map_err(|e| PacketError::SerializationError(e.to_string()))?;
        let header = PacketHeader::new(packet_type, payload.len() as u32);
        Ok(Self { header, payload })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let total = PacketHeader::SIZE + self.payload.len();
        let mut buf = Vec::with_capacity(total);
        buf.extend_from_slice(&self.header.to_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Deserialize from bytes
    pub fn from_bytes(buf: &[u8]) -> Result<Self, PacketError> {
        let header = PacketHeader::from_bytes(buf)?;
        let payload = buf[PacketHeader::SIZE..].to_vec();
        Ok(Self { header, payload })
    }

    /// Deserialize the payload into a specific type
    pub fn decode_payload<T: for<'de> Deserialize<'de>>(&self) -> Result<T, PacketError> {
        bincode::deserialize(&self.payload)
            .map_err(|e| PacketError::SerializationError(e.to_string()))
    }
}

// ── Control message payloads ────────────────────────────────

/// Handshake request from client to host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakePayload {
    pub username: String,
    pub public_key: [u8; 32], // X25519 public key
}

/// Handshake response from host to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeAckPayload {
    pub user_id: u16,
    pub room_name: String,
    pub host_public_key: [u8; 32],
    pub tcp_port: u16,
    pub udp_port: u16,
}

/// Room join acknowledgement with full room state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRoomAckPayload {
    pub users: Vec<UserInfo>,
    pub room_name: String,
}

/// User information broadcast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: u16,
    pub username: String,
    pub is_muted: bool,
    pub is_host: bool,
}

/// User list update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListPayload {
    pub users: Vec<UserInfo>,
    pub event: UserEvent,
}

/// Events that trigger user list updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserEvent {
    Joined(u16),
    Left(u16),
    StateChanged(u16),
}

/// Ping/Pong for keepalive and latency measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingPayload {
    pub timestamp: u64,
    pub sequence: u32,
}

/// Chat ACK
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAckPayload {
    pub message_id: [u8; 16],
}

/// Discovery announcement broadcast over LAN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryAnnouncePayload {
    pub room_name: String,
    pub host_name: String,
    pub tcp_port: u16,
    pub user_count: u8,
    pub max_users: u8,
}

// ── Errors ──────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("Packet too short: expected at least {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },

    #[error("Invalid magic bytes: expected [0xCC, 0xAA], got {got:?}")]
    InvalidMagic { got: [u8; 2] },

    #[error("Unsupported protocol version: {got}")]
    UnsupportedVersion { got: u8 },

    #[error("Unknown packet type: 0x{got:02x}")]
    UnknownPacketType { got: u8 },

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = PacketHeader::new(PacketType::VoiceFrame, 1234);
        let bytes = header.to_bytes();
        let parsed = PacketHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, parsed);
    }

    #[test]
    fn test_voice_packet_roundtrip() {
        let payload = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let packet = VoicePacket::new(42, 1000, 7, payload.clone());
        let bytes = packet.to_bytes();
        let parsed = VoicePacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.sequence, 42);
        assert_eq!(parsed.timestamp, 1000);
        assert_eq!(parsed.user_id, 7);
        assert_eq!(parsed.payload, payload);
    }

    #[test]
    fn test_chat_packet_roundtrip() {
        let msg_id = [0xAB; 16];
        let payload = b"hello world".to_vec();
        let packet = ChatPacket::new(msg_id, 3, 1713052800000, payload.clone());
        let bytes = packet.to_bytes();
        let parsed = ChatPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.message_id, msg_id);
        assert_eq!(parsed.user_id, 3);
        assert_eq!(parsed.timestamp, 1713052800000);
        assert_eq!(parsed.payload, payload);
    }

    #[test]
    fn test_control_packet_roundtrip() {
        let handshake = HandshakePayload {
            username: "alice".to_string(),
            public_key: [0x42; 32],
        };
        let packet = ControlPacket::new(PacketType::Handshake, &handshake).unwrap();
        let bytes = packet.to_bytes();
        let parsed = ControlPacket::from_bytes(&bytes).unwrap();
        let decoded: HandshakePayload = parsed.decode_payload().unwrap();
        assert_eq!(decoded.username, "alice");
        assert_eq!(decoded.public_key, [0x42; 32]);
    }

    #[test]
    fn test_silence_packet() {
        let packet = VoicePacket::silence(10, 500, 1);
        assert_eq!(packet.header.packet_type, PacketType::VoiceSilence);
        assert!(packet.payload.is_empty());
        let bytes = packet.to_bytes();
        let parsed = VoicePacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.sequence, 10);
        assert!(parsed.payload.is_empty());
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = PacketHeader::new(PacketType::Ping, 0).to_bytes();
        bytes[0] = 0xFF; // corrupt magic
        assert!(matches!(
            PacketHeader::from_bytes(&bytes),
            Err(PacketError::InvalidMagic { .. })
        ));
    }
}
