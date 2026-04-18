use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use parking_lot::RwLock;

use chatcall_net::transport::tcp_channel::TcpChannel;
use chatcall_net::transport::udp_channel::UdpChannel;
use chatcall_net::protocol::packet::*;
use chatcall_net::protocol::types::PacketType;
use chatcall_net::crypto::keypair::KeyPair;
use chatcall_net::crypto::session_key::SessionKeys;
use chatcall_net::discovery::lan::LanDiscovery;

use crate::room::state::{RoomConfig, RoomState};
use crate::events::{RoomEvent, EventSender};

/// Error type for host operations
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TCP error: {0}")]
    Tcp(#[from] chatcall_net::transport::tcp_channel::TcpError),

    #[error("Packet error: {0}")]
    Packet(#[from] PacketError),

    #[error("Room is full")]
    RoomFull,

    #[error("Host already running")]
    AlreadyRunning,
}

/// Client connection state tracked by the host
#[allow(dead_code)]
struct ClientConnection {
    tcp: TcpChannel,
    session_keys: SessionKeys,
    udp_addr: Option<SocketAddr>,
    user_id: u16,
    username: String,
}

/// Room host that manages connections and relays data.
///
/// The host is responsible for:
/// - Accepting TCP connections from clients
/// - Performing handshake and key exchange
/// - Relaying voice UDP packets between clients
/// - Broadcasting chat messages to all clients
/// - Managing room state (user list, join/leave events)
/// - Running LAN discovery announcements
pub struct RoomHost {
    config: RoomConfig,
    state: Arc<RwLock<RoomState>>,
    event_tx: EventSender,
    is_running: Arc<RwLock<bool>>,
    /// Abort handles for spawned tasks — aborted on stop() to release ports immediately
    task_handles: Arc<parking_lot::Mutex<Vec<tokio::task::AbortHandle>>>,
}

impl RoomHost {
    /// Create a new room host with the given configuration
    pub fn new(config: RoomConfig, event_tx: EventSender) -> Self {
        let state = RoomState::new(config.clone());
        Self {
            config,
            state: Arc::new(RwLock::new(state)),
            event_tx,
            is_running: Arc::new(RwLock::new(false)),
            task_handles: Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }

    /// Start the host — begins listening for connections.
    /// This spawns background tasks and returns immediately.
    pub async fn start(&self) -> Result<(), HostError> {
        if *self.is_running.read() {
            return Err(HostError::AlreadyRunning);
        }
        *self.is_running.write() = true;

        let tcp_addr = format!("0.0.0.0:{}", self.config.tcp_port);
        let udp_addr = format!("0.0.0.0:{}", self.config.udp_port);

        let tcp_listener = TcpListener::bind(&tcp_addr).await?;
        let udp_channel = UdpChannel::bind(&udp_addr).await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        tracing::info!("Room host started: TCP={}, UDP={}", tcp_addr, udp_addr);

        let state = self.state.clone();
        let event_tx = self.event_tx.clone();
        let is_running = self.is_running.clone();
        let config = self.config.clone();
        let udp_clone = udp_channel.clone();

        // Spawn TCP accept loop
        let state_tcp = state.clone();
        let event_tx_tcp = event_tx.clone();
        let is_running_tcp = is_running.clone();
        let udp_for_tcp = udp_channel.clone();

        let tcp_task = tokio::spawn(async move {
            Self::tcp_accept_loop(
                tcp_listener,
                state_tcp,
                event_tx_tcp,
                is_running_tcp,
                udp_for_tcp,
            ).await;
        });

        // Spawn UDP voice relay loop
        let state_udp = state.clone();
        let is_running_udp = is_running.clone();

        let udp_task = tokio::spawn(async move {
            Self::udp_relay_loop(udp_clone, state_udp, is_running_udp).await;
        });

        // Spawn LAN discovery announcer
        let config_disc = config.clone();
        let state_disc = state.clone();
        let is_running_disc = is_running.clone();

        let disc_task = tokio::spawn(async move {
            Self::discovery_loop(config_disc, state_disc, is_running_disc).await;
        });

        // Spawn UPnP auto-port-forwarder
        let config_upnp = config.clone();
        let upnp_task = tokio::task::spawn_blocking(move || {
            // Automatically add Windows Firewall rules if running as Administrator
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("netsh")
                    .args(&["advfirewall", "firewall", "add", "rule", "name=Lake App", "dir=in", "action=allow", "protocol=TCP", &format!("localport={}", config_upnp.tcp_port)])
                    .output();
                let _ = std::process::Command::new("netsh")
                    .args(&["advfirewall", "firewall", "add", "rule", "name=Lake App UDP", "dir=in", "action=allow", "protocol=UDP", &format!("localport={}", config_upnp.udp_port)])
                    .output();
            }

            tracing::info!("Searching for UPnP Internet Gateway Device...");
            match igd::search_gateway(Default::default()) {
                Ok(gw) => {
                    tracing::info!("Found UPnP Gateway: {}", gw);
                    if let Ok(std::net::IpAddr::V4(local_ip)) = local_ip_address::local_ip() {
                        let tcp_addr = std::net::SocketAddrV4::new(local_ip, config_upnp.tcp_port);
                        let udp_addr = std::net::SocketAddrV4::new(local_ip, config_upnp.udp_port);
                        
                        // 3600 seconds = 1 hour lease duration. Many modern routers reject 0 (infinite).
                        let tcp_res = gw.add_port(igd::PortMappingProtocol::TCP, config_upnp.tcp_port, tcp_addr, 3600, "Lake App");
                        let udp_res = gw.add_port(igd::PortMappingProtocol::UDP, config_upnp.udp_port, udp_addr, 3600, "Lake App UDP");
                        
                        if tcp_res.is_err() || udp_res.is_err() {
                            tracing::warn!("UPnP Mapping failed (Router denied request). TCP: {:?}, UDP: {:?}", tcp_res.err(), udp_res.err());
                        } else {
                            tracing::info!("UPnP Port Mapping successful! {} / {}", config_upnp.tcp_port, config_upnp.udp_port);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("UPnP Gateway not found or error: {}", e);
                }
            }
        });

        // Store abort handles so stop() can kill them immediately
        {
            let mut handles = self.task_handles.lock();
            handles.push(tcp_task.abort_handle());
            handles.push(udp_task.abort_handle());
            handles.push(disc_task.abort_handle());
            handles.push(upnp_task.abort_handle());
        }

        // Emit room created event
        let _ = event_tx.send(RoomEvent::RoomCreated {
            room_name: config.room_name.clone(),
        });

        Ok(())
    }

    /// Stop the host — aborts all spawned tasks, releasing ports immediately.
    pub fn stop(&self) {
        *self.is_running.write() = false;
        // Abort all background tasks so TcpListener/UdpSocket drop right away
        for handle in self.task_handles.lock().drain(..) {
            handle.abort();
        }

        // Remove UPnP mapping synchronously by firing off a new search
        let tcp_port = self.config.tcp_port;
        let udp_port = self.config.udp_port;
        
        // We spawn a short-lived sync task to remove mappings
        tokio::task::spawn_blocking(move || {
            // Remove firewall rules
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("netsh")
                    .args(&["advfirewall", "firewall", "delete", "rule", "name=Lake App"])
                    .output();
                let _ = std::process::Command::new("netsh")
                    .args(&["advfirewall", "firewall", "delete", "rule", "name=Lake App UDP"])
                    .output();
            }

            if let Ok(gw) = igd::search_gateway(Default::default()) {
                let _ = gw.remove_port(igd::PortMappingProtocol::TCP, tcp_port);
                let _ = gw.remove_port(igd::PortMappingProtocol::UDP, udp_port);
                tracing::info!("UPnP mappings removed");
            }
        });

        let _ = self.event_tx.send(RoomEvent::RoomClosed);
        tracing::info!("Room host stopped — ports released");
    }

    /// Get the current room state
    pub fn state(&self) -> RoomState {
        self.state.read().clone()
    }

    /// Check if host is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }

    // ── Background task loops ────────────────────────────────

    async fn tcp_accept_loop(
        listener: TcpListener,
        state: Arc<RwLock<RoomState>>,
        event_tx: EventSender,
        is_running: Arc<RwLock<bool>>,
        udp_channel: UdpChannel,
    ) {
        while *is_running.read() {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("New connection from {}", addr);

                    let state = state.clone();
                    let event_tx = event_tx.clone();
                    let udp = udp_channel.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, addr, state, event_tx, udp).await {
                            tracing::warn!("Client {} error: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("TCP accept error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn handle_client(
        stream: tokio::net::TcpStream,
        addr: SocketAddr,
        state: Arc<RwLock<RoomState>>,
        event_tx: EventSender,
        udp_channel: UdpChannel,
    ) -> Result<(), HostError> {
        let mut tcp = TcpChannel::new(stream)?;

        // ── Step 1: Receive handshake ────────────────────────
        let handshake_bytes = tcp.recv_packet().await?;
        let handshake_packet = ControlPacket::from_bytes(&handshake_bytes)?;
        let handshake: HandshakePayload = handshake_packet.decode_payload()?;

        tracing::info!("Handshake from {} ({})", handshake.username, addr);

        // ── Step 2: Generate host keypair and derive session keys ─
        let host_keypair = KeyPair::generate();
        let host_pub = host_keypair.public_key_bytes();
        let shared_secret = host_keypair.diffie_hellman(&handshake.public_key);
        let _session_keys = SessionKeys::derive(shared_secret.as_bytes(), true);

        // ── Step 3: Assign user ID and send handshake ACK ────
        let user_id = {
            let mut st = state.write();
            match st.add_user(handshake.username.clone()) {
                Some(id) => id,
                None => return Err(HostError::RoomFull),
            }
        };

        let config = state.read().config.clone();
        let ack_payload = HandshakeAckPayload {
            user_id,
            room_name: config.room_name.clone(),
            host_public_key: host_pub,
            tcp_port: config.tcp_port,
            udp_port: config.udp_port,
        };

        let ack_packet = ControlPacket::new(PacketType::HandshakeAck, &ack_payload)?;
        tcp.send_packet(&ack_packet.to_bytes()).await?;

        // ── Step 4: Send current user list ───────────────────
        let user_list = state.read().user_list();
        let join_ack = JoinRoomAckPayload {
            users: user_list,
            room_name: config.room_name.clone(),
        };
        let join_packet = ControlPacket::new(PacketType::JoinRoomAck, &join_ack)?;
        tcp.send_packet(&join_packet.to_bytes()).await?;

        // Register UDP peer address (same IP, client's UDP port)
        let udp_addr = SocketAddr::new(addr.ip(), config.udp_port + 1);
        udp_channel.add_peer(user_id, udp_addr);

        // ── Emit join event ──────────────────────────────────
        let _ = event_tx.send(RoomEvent::UserJoined {
            user_id,
            username: handshake.username.clone(),
        });

        // ── Step 5: Main client message loop ─────────────────
        loop {
            match tcp.recv_packet().await {
                Ok(packet_bytes) => {
                    let header = match PacketHeader::from_bytes(&packet_bytes) {
                        Ok(h) => h,
                        Err(_) => continue,
                    };

                    match header.packet_type {
                        PacketType::ChatMessage => {
                            // Decrypt, store, and relay to all other clients
                            let _ = event_tx.send(RoomEvent::ChatMessageReceived {
                                user_id,
                                data: packet_bytes.clone(),
                            });
                            // TODO: relay to other connected clients
                        }
                        PacketType::LeaveRoom => {
                            tracing::info!("User {} leaving room", user_id);
                            break;
                        }
                        PacketType::Ping => {
                            // Respond with pong
                            let pong = ControlPacket::new(PacketType::Pong, &PingPayload {
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64,
                                sequence: 0,
                            });
                            if let Ok(pong_packet) = pong {
                                let _ = tcp.send_packet(&pong_packet.to_bytes()).await;
                            }
                        }
                        _ => {
                            tracing::debug!("Unhandled packet type: {:?}", header.packet_type);
                        }
                    }
                }
                Err(chatcall_net::transport::tcp_channel::TcpError::ConnectionClosed) => {
                    tracing::info!("Client {} disconnected", user_id);
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error reading from client {}: {}", user_id, e);
                    break;
                }
            }
        }

        // ── Cleanup ──────────────────────────────────────────
        state.write().remove_user(user_id);
        udp_channel.remove_peer(user_id);

        let _ = event_tx.send(RoomEvent::UserLeft {
            user_id,
            username: handshake.username,
        });

        Ok(())
    }

    /// UDP relay loop: receives voice packets from any client and
    /// broadcasts them to all other clients.
    async fn udp_relay_loop(
        udp: UdpChannel,
        _state: Arc<RwLock<RoomState>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        let mut buf = [0u8; chatcall_net::MAX_UDP_PACKET_SIZE];

        while *is_running.read() {
            match udp.recv_from(&mut buf).await {
                Ok((n, _addr)) => {
                    // Parse the voice packet to get the sender's user_id
                    if let Ok(voice) = VoicePacket::from_bytes(&buf[..n]) {
                        // Relay to all other peers
                        let _ = udp.broadcast(&buf[..n], voice.user_id).await;
                    }
                }
                Err(e) => {
                    tracing::debug!("UDP recv error: {}", e);
                }
            }
        }
    }

    /// LAN discovery broadcast loop
    async fn discovery_loop(
        config: RoomConfig,
        state: Arc<RwLock<RoomState>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        // Try to bind discovery, ignore if port is in use
        let discovery = match LanDiscovery::bind(0).await {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to start discovery: {}", e);
                return;
            }
        };

        while *is_running.read() {
            let user_count = state.read().user_count() as u8;

            let payload = DiscoveryAnnouncePayload {
                room_name: config.room_name.clone(),
                host_name: config.host_name.clone(),
                tcp_port: config.tcp_port,
                user_count,
                max_users: config.max_users,
            };

            if let Err(e) = discovery.announce(&payload).await {
                tracing::debug!("Discovery announce error: {}", e);
            }

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }
}
