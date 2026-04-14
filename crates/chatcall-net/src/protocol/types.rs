use serde::{Serialize, Deserialize};

/// All packet types supported by the ChatCall protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PacketType {
    // ── Control plane (TCP) ─────────────────────────────────
    /// Initial handshake from client to host
    Handshake       = 0x01,
    /// Host acknowledges handshake, assigns user_id
    HandshakeAck    = 0x02,
    /// Client requests to join a room
    JoinRoom        = 0x03,
    /// Host confirms join, sends room state
    JoinRoomAck     = 0x04,
    /// User leaves the room
    LeaveRoom       = 0x05,
    /// Broadcast updated user list
    UserList        = 0x06,
    /// User state change (mute/unmute/etc.)
    UserStateChange = 0x07,

    // ── Chat (TCP, reliable) ────────────────────────────────
    /// Text chat message
    ChatMessage     = 0x10,
    /// Acknowledgement of chat message receipt
    ChatAck         = 0x11,

    // ── Keepalive (TCP) ─────────────────────────────────────
    /// Ping for latency measurement and keepalive
    Ping            = 0x20,
    /// Pong response
    Pong            = 0x21,

    // ── Security (TCP) ──────────────────────────────────────
    /// Key exchange packet (X25519 public key)
    KeyExchange     = 0x30,
    /// Key exchange acknowledgement
    KeyExchangeAck  = 0x31,

    // ── Voice (UDP) ─────────────────────────────────────────
    /// Encoded voice audio frame
    VoiceFrame      = 0x40,
    /// Silence indicator (no audio data, saves bandwidth)
    VoiceSilence    = 0x41,

    // ── Discovery (UDP broadcast) ───────────────────────────
    /// Room advertisement broadcast
    DiscoveryAnnounce = 0x50,
    /// Response to discovery probe
    DiscoveryResponse = 0x51,
}

impl PacketType {
    /// Convert from raw byte value
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(Self::Handshake),
            0x02 => Some(Self::HandshakeAck),
            0x03 => Some(Self::JoinRoom),
            0x04 => Some(Self::JoinRoomAck),
            0x05 => Some(Self::LeaveRoom),
            0x06 => Some(Self::UserList),
            0x07 => Some(Self::UserStateChange),
            0x10 => Some(Self::ChatMessage),
            0x11 => Some(Self::ChatAck),
            0x20 => Some(Self::Ping),
            0x21 => Some(Self::Pong),
            0x30 => Some(Self::KeyExchange),
            0x31 => Some(Self::KeyExchangeAck),
            0x40 => Some(Self::VoiceFrame),
            0x41 => Some(Self::VoiceSilence),
            0x50 => Some(Self::DiscoveryAnnounce),
            0x51 => Some(Self::DiscoveryResponse),
            _ => None,
        }
    }

    /// Whether this packet type uses TCP (control/chat) or UDP (voice/discovery)
    pub fn is_tcp(&self) -> bool {
        (*self as u8) < 0x40
    }

    /// Whether this packet type uses UDP
    pub fn is_udp(&self) -> bool {
        !self.is_tcp()
    }
}
