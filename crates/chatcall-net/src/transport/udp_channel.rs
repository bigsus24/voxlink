use tokio::net::UdpSocket;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use parking_lot::RwLock;

/// Error type for UDP channel operations
#[derive(Debug, thiserror::Error)]
pub enum UdpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown peer: user_id={0}")]
    UnknownPeer(u16),

    #[error("Packet too large: {size} bytes (max {max})")]
    PacketTooLarge { size: usize, max: usize },
}

/// UDP channel for low-latency voice data transmission.
///
/// Manages a single UDP socket and a mapping of user IDs to their
/// socket addresses. Supports sending to specific peers, broadcasting
/// to all peers, and receiving from any source.
pub struct UdpChannel {
    socket: Arc<UdpSocket>,
    /// Map of user_id -> their UDP address
    peers: Arc<RwLock<HashMap<u16, SocketAddr>>>,
    /// Maximum packet size
    max_packet_size: usize,
}

impl UdpChannel {
    /// Default maximum UDP packet size (below typical MTU to avoid fragmentation)
    pub const DEFAULT_MAX_PACKET: usize = 1400;

    /// Bind a UDP socket to the specified address
    pub async fn bind(addr: &str) -> Result<Self, UdpError> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
            peers: Arc::new(RwLock::new(HashMap::new())),
            max_packet_size: Self::DEFAULT_MAX_PACKET,
        })
    }

    /// Create from an existing bound socket
    pub fn from_socket(socket: UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
            peers: Arc::new(RwLock::new(HashMap::new())),
            max_packet_size: Self::DEFAULT_MAX_PACKET,
        }
    }

    /// Register a peer's address
    pub fn add_peer(&self, user_id: u16, addr: SocketAddr) {
        self.peers.write().insert(user_id, addr);
    }

    /// Remove a peer
    pub fn remove_peer(&self, user_id: u16) {
        self.peers.write().remove(&user_id);
    }

    /// Get a peer's address
    pub fn get_peer_addr(&self, user_id: u16) -> Option<SocketAddr> {
        self.peers.read().get(&user_id).copied()
    }

    /// Get all registered peer IDs
    pub fn peer_ids(&self) -> Vec<u16> {
        self.peers.read().keys().copied().collect()
    }

    /// Send raw bytes to a specific peer by user_id
    pub async fn send_to_peer(&self, user_id: u16, data: &[u8]) -> Result<usize, UdpError> {
        if data.len() > self.max_packet_size {
            return Err(UdpError::PacketTooLarge {
                size: data.len(),
                max: self.max_packet_size,
            });
        }
        let addr = self.peers.read().get(&user_id).copied()
            .ok_or(UdpError::UnknownPeer(user_id))?;
        let sent = self.socket.send_to(data, addr).await?;
        Ok(sent)
    }

    /// Send raw bytes to a specific socket address
    pub async fn send_to_addr(&self, addr: SocketAddr, data: &[u8]) -> Result<usize, UdpError> {
        if data.len() > self.max_packet_size {
            return Err(UdpError::PacketTooLarge {
                size: data.len(),
                max: self.max_packet_size,
            });
        }
        let sent = self.socket.send_to(data, addr).await?;
        Ok(sent)
    }

    /// Broadcast data to all registered peers except the specified user
    pub async fn broadcast(&self, data: &[u8], exclude_user: u16) -> Result<(), UdpError> {
        if data.len() > self.max_packet_size {
            return Err(UdpError::PacketTooLarge {
                size: data.len(),
                max: self.max_packet_size,
            });
        }
        let peers: Vec<(u16, SocketAddr)> = self.peers.read()
            .iter()
            .filter(|(&id, _)| id != exclude_user)
            .map(|(&id, &addr)| (id, addr))
            .collect();

        for (_, addr) in peers {
            // Fire-and-forget for voice — don't let one failed send block others
            let _ = self.socket.send_to(data, addr).await;
        }
        Ok(())
    }

    /// Broadcast to ALL peers (no exclusion)
    pub async fn broadcast_all(&self, data: &[u8]) -> Result<(), UdpError> {
        if data.len() > self.max_packet_size {
            return Err(UdpError::PacketTooLarge {
                size: data.len(),
                max: self.max_packet_size,
            });
        }
        let addrs: Vec<SocketAddr> = self.peers.read().values().copied().collect();
        for addr in addrs {
            let _ = self.socket.send_to(data, addr).await;
        }
        Ok(())
    }

    /// Receive a UDP datagram. Returns the data and the sender's address.
    /// This is the main recv loop call — blocks until data arrives.
    pub async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), UdpError> {
        let (n, addr) = self.socket.recv_from(buf).await?;
        Ok((n, addr))
    }

    /// Get the local address this socket is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, UdpError> {
        Ok(self.socket.local_addr()?)
    }

    /// Get a clone of the socket Arc (for sharing across tasks)
    pub fn socket_handle(&self) -> Arc<UdpSocket> {
        Arc::clone(&self.socket)
    }
}

impl Clone for UdpChannel {
    fn clone(&self) -> Self {
        Self {
            socket: Arc::clone(&self.socket),
            peers: Arc::clone(&self.peers),
            max_packet_size: self.max_packet_size,
        }
    }
}
