use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use parking_lot::RwLock;

use chatcall_net::transport::tcp_channel::TcpChannel;
use chatcall_net::transport::udp_channel::UdpChannel;
use chatcall_net::protocol::packet::*;
use chatcall_net::protocol::types::PacketType;
use chatcall_net::crypto::keypair::KeyPair;
use chatcall_net::crypto::session_key::SessionKeys;
use chatcall_net::crypto::cipher::SessionCipher;
use chatcall_net::reliability::ack_tracker::AckTracker;

use crate::room::state::RoomState;
use crate::events::{RoomEvent, EventSender};

/// Error type for client operations
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TCP error: {0}")]
    Tcp(#[from] chatcall_net::transport::tcp_channel::TcpError),

    #[error("Packet error: {0}")]
    Packet(#[from] PacketError),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Handshake failed")]
    HandshakeFailed,

    #[error("Already connected")]
    AlreadyConnected,
}

/// Room client that connects to a host.
///
/// The client is responsible for:
/// - Connecting to the host via TCP
/// - Performing handshake and key exchange
/// - Sending/receiving voice data via UDP
/// - Sending/receiving chat messages via TCP
/// - Maintaining connection state
pub struct RoomClient {
    /// Our assigned user ID
    user_id: Option<u16>,
    /// Our username
    username: String,
    /// TCP channel to host
    tcp: Option<TcpChannel>,
    /// UDP channel for voice
    udp: Option<UdpChannel>,
    /// Session encryption keys
    session_keys: Option<SessionKeys>,
    /// ACK tracker for reliable chat
    ack_tracker: AckTracker,
    /// Event sender
    event_tx: EventSender,
    /// Connection state
    is_connected: Arc<RwLock<bool>>,
    /// Room info received from host
    room_name: Option<String>,
    /// Host address
    host_addr: Option<SocketAddr>,
}

impl RoomClient {
    /// Create a new client
    pub fn new(username: String, event_tx: EventSender) -> Self {
        Self {
            user_id: None,
            username,
            tcp: None,
            udp: None,
            session_keys: None,
            ack_tracker: AckTracker::new(),
            event_tx,
            is_connected: Arc::new(RwLock::new(false)),
            room_name: None,
            host_addr: None,
        }
    }

    /// Connect to a room host at the given address.
    ///
    /// Performs the full handshake:
    /// 1. TCP connect
    /// 2. Send Handshake with username + public key
    /// 3. Receive HandshakeAck with user_id + host public key
    /// 4. Derive session encryption keys
    /// 5. Receive room state (user list)
    /// 6. Bind UDP socket for voice
    pub async fn connect(&mut self, host_addr: SocketAddr) -> Result<(), ClientError> {
        if *self.is_connected.read() {
            return Err(ClientError::AlreadyConnected);
        }

        tracing::info!("Connecting to host at {}", host_addr);

        // ── Step 1: TCP connect ──────────────────────────────
        let stream = TcpStream::connect(host_addr).await?;
        let mut tcp = TcpChannel::new(stream)?;

        // ── Step 2: Generate keypair and send handshake ──────
        let keypair = KeyPair::generate();
        let our_pub = keypair.public_key_bytes();

        let handshake = HandshakePayload {
            username: self.username.clone(),
            public_key: our_pub,
        };

        let packet = ControlPacket::new(PacketType::Handshake, &handshake)?;
        tcp.send_packet(&packet.to_bytes()).await?;

        // ── Step 3: Receive HandshakeAck ─────────────────────
        let ack_bytes = tcp.recv_packet().await?;
        let ack_packet = ControlPacket::from_bytes(&ack_bytes)?;

        if ack_packet.header.packet_type != PacketType::HandshakeAck {
            return Err(ClientError::HandshakeFailed);
        }

        let ack: HandshakeAckPayload = ack_packet.decode_payload()?;

        tracing::info!("Handshake accepted: user_id={}, room={}", ack.user_id, ack.room_name);

        // ── Step 4: Derive session keys ──────────────────────
        let shared_secret = keypair.diffie_hellman(&ack.host_public_key);
        let session_keys = SessionKeys::derive(shared_secret.as_bytes(), false);

        // ── Step 5: Receive room state (JoinRoomAck) ─────────
        let join_bytes = tcp.recv_packet().await?;
        let join_packet = ControlPacket::from_bytes(&join_bytes)?;
        let join_ack: JoinRoomAckPayload = join_packet.decode_payload()?;

        tracing::info!("Joined room '{}' with {} users", join_ack.room_name, join_ack.users.len());

        // ── Step 6: Bind UDP socket ──────────────────────────
        let udp_local = format!("0.0.0.0:{}", ack.udp_port + 1);
        let udp = UdpChannel::bind(&udp_local).await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        // Register host as a UDP peer
        let host_udp_addr = SocketAddr::new(host_addr.ip(), ack.udp_port);
        udp.add_peer(0, host_udp_addr); // host is user_id 0

        // ── Store state ──────────────────────────────────────
        self.user_id = Some(ack.user_id);
        self.tcp = Some(tcp);
        self.udp = Some(udp);
        self.session_keys = Some(session_keys);
        self.room_name = Some(ack.room_name.clone());
        self.host_addr = Some(host_addr);
        *self.is_connected.write() = true;

        // Emit events
        let _ = self.event_tx.send(RoomEvent::Connected {
            user_id: ack.user_id,
            room_name: ack.room_name.clone(),
        });

        for user in &join_ack.users {
            let _ = self.event_tx.send(RoomEvent::UserJoined {
                user_id: user.user_id,
                username: user.username.clone(),
            });
        }

        Ok(())
    }

    /// Send a chat message
    pub async fn send_chat(&mut self, text: &str) -> Result<[u8; 16], ClientError> {
        let tcp = self.tcp.as_mut()
            .ok_or(ClientError::ConnectionFailed("Not connected".into()))?;
        let user_id = self.user_id.unwrap_or(0);

        let message_id = uuid::Uuid::new_v4().into_bytes();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Encrypt the message text
        let payload = if let Some(keys) = &self.session_keys {
            keys.send_cipher.encrypt(text.as_bytes())
                .unwrap_or_else(|_| text.as_bytes().to_vec())
        } else {
            text.as_bytes().to_vec()
        };

        let chat_packet = ChatPacket::new(message_id, user_id, timestamp, payload);
        let bytes = chat_packet.to_bytes();

        // Register for ACK tracking
        self.ack_tracker.register(message_id, bytes.clone());

        tcp.send_packet(&bytes).await?;

        Ok(message_id)
    }

    /// Send voice data to the host for relay
    pub async fn send_voice(&self, voice_data: &[u8]) -> Result<(), ClientError> {
        let udp = self.udp.as_ref()
            .ok_or(ClientError::ConnectionFailed("Not connected".into()))?;

        // Send to host (user_id 0) for relay
        udp.send_to_peer(0, voice_data).await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        Ok(())
    }

    /// Disconnect from the room
    pub async fn disconnect(&mut self) -> Result<(), ClientError> {
        if let Some(tcp) = &mut self.tcp {
            let leave = ControlPacket::new(PacketType::LeaveRoom, &());
            if let Ok(packet) = leave {
                let _ = tcp.send_packet(&packet.to_bytes()).await;
            }
            let _ = tcp.shutdown().await;
        }

        self.tcp = None;
        self.udp = None;
        self.session_keys = None;
        self.user_id = None;
        *self.is_connected.write() = false;

        let _ = self.event_tx.send(RoomEvent::Disconnected);

        Ok(())
    }

    /// Get our user ID
    pub fn user_id(&self) -> Option<u16> {
        self.user_id
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        *self.is_connected.read()
    }

    /// Get the room name
    pub fn room_name(&self) -> Option<&str> {
        self.room_name.as_deref()
    }

    /// Get a reference to the UDP channel (for the voice pipeline)
    pub fn udp_channel(&self) -> Option<&UdpChannel> {
        self.udp.as_ref()
    }

    /// Get the send cipher for encrypting voice packets
    pub fn send_cipher(&self) -> Option<&SessionCipher> {
        self.session_keys.as_ref().map(|k| &k.send_cipher)
    }

    /// Get the recv cipher for decrypting voice packets
    pub fn recv_cipher(&self) -> Option<&SessionCipher> {
        self.session_keys.as_ref().map(|k| &k.recv_cipher)
    }
}
