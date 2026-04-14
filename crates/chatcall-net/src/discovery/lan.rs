use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::time::Duration;
use crate::protocol::packet::{
    ControlPacket, DiscoveryAnnouncePayload, PacketHeader, PacketError,
};
use crate::protocol::types::PacketType;
use crate::DISCOVERY_PORT;

/// Error type for discovery operations
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Packet error: {0}")]
    Packet(#[from] PacketError),
}

/// Discovered room on the LAN
#[derive(Debug, Clone)]
pub struct DiscoveredRoom {
    pub room_name: String,
    pub host_name: String,
    pub host_addr: SocketAddr,
    pub tcp_port: u16,
    pub user_count: u8,
    pub max_users: u8,
}

/// LAN peer discovery using UDP broadcast.
///
/// The host periodically broadcasts room announcements on the discovery port.
/// Clients listen for these broadcasts to find rooms on the local network.
pub struct LanDiscovery {
    socket: UdpSocket,
}

impl LanDiscovery {
    /// Create a new discovery broadcaster/listener
    pub async fn new() -> Result<Self, DiscoveryError> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)).await?;
        socket.set_broadcast(true)?;
        Ok(Self { socket })
    }

    /// Create a discovery instance bound to a specific port (for hosts that
    /// may already have the discovery port in use)
    pub async fn bind(port: u16) -> Result<Self, DiscoveryError> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
        socket.set_broadcast(true)?;
        Ok(Self { socket })
    }

    /// Broadcast a room announcement on the LAN.
    /// Should be called periodically by the host (e.g., every 3 seconds).
    pub async fn announce(&self, payload: &DiscoveryAnnouncePayload) -> Result<(), DiscoveryError> {
        let packet = ControlPacket::new(PacketType::DiscoveryAnnounce, payload)?;
        let bytes = packet.to_bytes();
        let broadcast_addr: SocketAddr = format!("255.255.255.255:{}", DISCOVERY_PORT).parse()
            .expect("valid broadcast address");
        self.socket.send_to(&bytes, broadcast_addr).await?;
        Ok(())
    }

    /// Listen for room announcements. Returns the next discovered room.
    /// Blocks until an announcement is received or the timeout expires.
    pub async fn listen(&self, timeout: Duration) -> Result<Option<DiscoveredRoom>, DiscoveryError> {
        let mut buf = [0u8; 1024];

        match tokio::time::timeout(timeout, self.socket.recv_from(&mut buf)).await {
            Ok(Ok((n, addr))) => {
                // Try to parse as a discovery announcement
                match ControlPacket::from_bytes(&buf[..n]) {
                    Ok(packet) => {
                        if packet.header.packet_type == PacketType::DiscoveryAnnounce {
                            if let Ok(payload) = packet.decode_payload::<DiscoveryAnnouncePayload>() {
                                return Ok(Some(DiscoveredRoom {
                                    room_name: payload.room_name,
                                    host_name: payload.host_name,
                                    host_addr: addr,
                                    tcp_port: payload.tcp_port,
                                    user_count: payload.user_count,
                                    max_users: payload.max_users,
                                }));
                            }
                        }
                        Ok(None)
                    }
                    Err(_) => Ok(None), // not a valid packet, ignore
                }
            }
            Ok(Err(e)) => Err(DiscoveryError::Io(e)),
            Err(_) => Ok(None), // timeout
        }
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<SocketAddr, DiscoveryError> {
        Ok(self.socket.local_addr()?)
    }
}
