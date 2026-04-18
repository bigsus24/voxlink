use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;
use chatcall_core::room::host::RoomHost;
use chatcall_core::room::client::RoomClient;
use chatcall_core::room::state::RoomConfig;
use chatcall_net::{encode_ip, decode_ip, DEFAULT_TCP_PORT};

// ── Shared response types ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_name: String,
    pub is_host: bool,
    /// 7-char VoxCode (only present when you are the host)
    pub room_code: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────

/// Fetch the machine's public IPv4 from api.ipify.org.
/// Fast — plain text response, no JSON parsing needed.
async fn get_public_ip() -> Result<String, String> {
    let ip = reqwest::get("https://api.ipify.org")
        .await
        .map_err(|e| format!("Could not reach ipify: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Bad response from ipify: {e}"))?;
    Ok(ip.trim().to_string())
}

// ── Commands ──────────────────────────────────────────────────

/// Create a room. Detects public IP → encodes to VoxCode → starts host.
/// Returns the RoomInfo including the shareable code.
#[tauri::command]
pub async fn create_room(
    state: State<'_, AppState>,
    room_name: String,
) -> Result<RoomInfo, String> {
    let username = state.profile.read().username.clone();

    // 1. Get public IP and encode → VoxCode
    let public_ip = get_public_ip().await?;
    let room_code = encode_ip(&public_ip)
        .map_err(|e| format!("Failed to encode IP: {e}"))?;

    tracing::info!("Public IP: {} → VoxCode: {}", public_ip, room_code);

    // 2. Configure and launch the host
    let config = RoomConfig {
        room_name: room_name.clone(),
        host_name: username,
        max_users: 20,
        tcp_port: DEFAULT_TCP_PORT,
        udp_port: chatcall_net::DEFAULT_UDP_PORT,
    };
    let host = RoomHost::new(config, state.event_tx.clone());
    host.start().await.map_err(|e| e.to_string())?;

    // Host runs on background tasks — leaked intentionally for MVP.
    let _ = Box::leak(Box::new(host));

    // 3. Persist state
    *state.is_in_room.write() = true;
    *state.is_host.write() = true;
    *state.room_name.write() = Some(room_name.clone());
    *state.room_code.write() = Some(room_code.clone());

    tracing::info!("Room '{}' created with code {}", room_name, room_code);

    Ok(RoomInfo {
        room_name,
        is_host: true,
        room_code: Some(room_code),
    })
}

/// Join a room by 7-char VoxCode. Decodes the code → public IP → connects.
#[tauri::command]
pub async fn join_by_code(
    state: State<'_, AppState>,
    code: String,
) -> Result<RoomInfo, String> {
    // Decode the VoxCode → IP
    let host_ip = decode_ip(&code)
        .map_err(|e| format!("Invalid room code: {e}"))?;

    tracing::info!("Code '{}' → IP {}", code, host_ip);

    // Connect using the decoded IP
    join_by_address(state, host_ip).await
}

/// Join a room directly by IP address (fallback / advanced users).
#[tauri::command]
pub async fn join_room(
    state: State<'_, AppState>,
    host_address: String,
) -> Result<RoomInfo, String> {
    join_by_address(state, host_address).await
}

/// Internal: connect to a host by IP string.
async fn join_by_address(
    state: State<'_, AppState>,
    host_ip: String,
) -> Result<RoomInfo, String> {
    let addr = format!("{}:{}", host_ip.trim(), DEFAULT_TCP_PORT)
        .parse()
        .map_err(|e: std::net::AddrParseError| format!("Invalid address: {e}"))?;

    let username = state.profile.read().username.clone();
    let mut client = RoomClient::new(username, state.event_tx.clone());
    client.connect(addr).await.map_err(|e| e.to_string())?;

    let room_name = client.room_name().unwrap_or("Room").to_string();

    *state.is_in_room.write() = true;
    *state.is_host.write() = false;
    *state.room_name.write() = Some(room_name.clone());
    *state.room_code.write() = None;

    // Client runs via spawned tasks — leaked for MVP
    let _ = Box::leak(Box::new(client));

    tracing::info!("Joined room '{}'", room_name);

    Ok(RoomInfo {
        room_name,
        is_host: false,
        room_code: None,
    })
}

/// Close the hosted room and return to lobby.
#[tauri::command]
pub async fn close_room(state: State<'_, AppState>) -> Result<(), String> {
    *state.is_in_room.write() = false;
    *state.is_host.write() = false;
    *state.room_name.write() = None;
    *state.room_code.write() = None;
    tracing::info!("Room closed");
    Ok(())
}

/// Leave a room (as client).
#[tauri::command]
pub async fn leave_room(state: State<'_, AppState>) -> Result<(), String> {
    *state.is_in_room.write() = false;
    *state.is_host.write() = false;
    *state.room_name.write() = None;
    *state.room_code.write() = None;
    tracing::info!("Left room");
    Ok(())
}

/// Get current room state.
#[tauri::command]
pub async fn get_room_state(state: State<'_, AppState>) -> Result<Option<RoomInfo>, String> {
    if !*state.is_in_room.read() {
        return Ok(None);
    }
    Ok(Some(RoomInfo {
        room_name: state.room_name.read().clone().unwrap_or_default(),
        is_host: *state.is_host.read(),
        room_code: state.room_code.read().clone(),
    }))
}
