use std::net::SocketAddr;
use crate::transport::tcp_channel::{TcpChannel, TcpError};

/// Combined TCP + UDP connection to a peer.
///
/// Each peer connection has:
/// - A TCP channel for reliable control messages and chat
/// - Access to the shared UDP channel for voice data
/// - Connection metadata (user_id, username, etc.)
pub struct PeerConnection {
    pub user_id: u16,
    pub username: String,
    pub tcp: TcpChannel,
    pub udp_addr: Option<SocketAddr>,
    pub is_muted: bool,
    pub latency_ms: u32,
}

impl PeerConnection {
    /// Create a new peer connection
    pub fn new(user_id: u16, username: String, tcp: TcpChannel) -> Self {
        Self {
            user_id,
            username,
            tcp,
            udp_addr: None,
            is_muted: false,
            latency_ms: 0,
        }
    }

    /// Set the UDP address for this peer (discovered after handshake)
    pub fn set_udp_addr(&mut self, addr: SocketAddr) {
        self.udp_addr = Some(addr);
    }

    /// Send a control/chat packet over TCP
    pub async fn send_tcp(&mut self, packet_bytes: &[u8]) -> Result<(), TcpError> {
        self.tcp.send_packet(packet_bytes).await
    }

    /// Receive a packet from TCP
    pub async fn recv_tcp(&mut self) -> Result<Vec<u8>, TcpError> {
        self.tcp.recv_packet().await
    }

    /// Get the TCP peer address
    pub fn tcp_addr(&self) -> SocketAddr {
        self.tcp.peer_addr()
    }

    /// Update measured latency
    pub fn update_latency(&mut self, latency_ms: u32) {
        self.latency_ms = latency_ms;
    }

    /// Graceful shutdown
    pub async fn disconnect(&mut self) -> Result<(), TcpError> {
        self.tcp.shutdown().await
    }
}
