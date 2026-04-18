//! # ChatCall Networking Library
//!
//! Custom networking library for peer-to-peer voice and text communication.
//! Built from scratch with no dependency on WebRTC, WebSockets, or
//! existing real-time communication frameworks.
//!
//! ## Modules
//!
//! - `protocol` - Custom binary packet protocol (header + payload)
//! - `transport` - TCP/UDP channel management
//! - `crypto` - ChaCha20-Poly1305 encryption + X25519 key exchange
//! - `reliability` - ACK tracking, retransmission, message ordering
//! - `discovery` - LAN peer discovery via UDP broadcast
//! - `serialization` - Binary serialization helpers

pub mod protocol;
pub mod transport;
pub mod crypto;
pub mod reliability;
pub mod discovery;
pub mod serialization;
pub mod room_code;

pub use protocol::types::PacketType;
pub use protocol::packet::{PacketHeader, VoicePacket, ChatPacket, ControlPacket};
pub use transport::tcp_channel::TcpChannel;
pub use transport::udp_channel::UdpChannel;
pub use transport::connection::PeerConnection;
pub use crypto::cipher::SessionCipher;
pub use crypto::keypair::KeyPair;
pub use discovery::lan::LanDiscovery;
pub use reliability::ack_tracker::AckTracker;
pub use room_code::{encode_ip, decode_ip};

/// Protocol magic bytes identifying ChatCall packets
pub const MAGIC: [u8; 2] = [0xCC, 0xAA];
/// Current protocol version
pub const PROTOCOL_VERSION: u8 = 0x01;
/// Default TCP control port
pub const DEFAULT_TCP_PORT: u16 = 7770;
/// Default UDP voice port
pub const DEFAULT_UDP_PORT: u16 = 7771;
/// Discovery broadcast port
pub const DISCOVERY_PORT: u16 = 7772;
/// Maximum UDP packet size (below typical MTU)
pub const MAX_UDP_PACKET_SIZE: usize = 1400;
/// Maximum users in a room
pub const MAX_ROOM_USERS: usize = 20;
