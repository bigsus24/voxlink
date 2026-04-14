use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;
use chatcall_core::room::host::RoomHost;
use chatcall_core::room::client::RoomClient;
use chatcall_core::room::state::RoomConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_name: String,
    pub user_count: usize,
    pub is_host: bool,
    pub users: Vec<UserEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserEntry {
    pub user_id: u16,
    pub username: String,
    pub is_muted: bool,
    pub is_host: bool,
}

/// Create and host a new room
#[tauri::command]
pub async fn create_room(
    state: State<'_, AppState>,
    room_name: String,
    tcp_port: Option<u16>,
    udp_port: Option<u16>,
) -> Result<RoomInfo, String> {
    let config = RoomConfig {
        room_name: room_name.clone(),
        host_name: state.profile.read().username.clone(),
        max_users: 20,
        tcp_port: tcp_port.unwrap_or(chatcall_net::DEFAULT_TCP_PORT),
        udp_port: udp_port.unwrap_or(chatcall_net::DEFAULT_UDP_PORT),
    };

    let host = RoomHost::new(config, state.event_tx.clone());
    host.start().await.map_err(|e| e.to_string())?;

    let room_state = host.state();

    *state.host.write() = Some(host);
    *state.is_in_room.write() = true;
    *state.is_host.write() = true;

    tracing::info!("Room '{}' created", room_name);

    Ok(RoomInfo {
        room_name,
        user_count: room_state.user_count(),
        is_host: true,
        users: vec![],
    })
}

/// Join an existing room by address
#[tauri::command]
pub async fn join_room(
    state: State<'_, AppState>,
    host_address: String,
    tcp_port: Option<u16>,
) -> Result<RoomInfo, String> {
    let port = tcp_port.unwrap_or(chatcall_net::DEFAULT_TCP_PORT);
    let addr = format!("{}:{}", host_address, port)
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;

    let username = state.profile.read().username.clone();
    let mut client = RoomClient::new(username, state.event_tx.clone());
    client.connect(addr).await.map_err(|e| e.to_string())?;

    let room_name = client.room_name().unwrap_or("Unknown").to_string();

    *state.client.write() = Some(client);
    *state.is_in_room.write() = true;
    *state.is_host.write() = false;

    tracing::info!("Joined room '{}'", room_name);

    Ok(RoomInfo {
        room_name,
        user_count: 0,
        is_host: false,
        users: vec![],
    })
}

/// Leave the current room
#[tauri::command]
pub async fn leave_room(state: State<'_, AppState>) -> Result<(), String> {
    // Disconnect client
    if let Some(client) = state.client.write().as_mut() {
        client.disconnect().await.map_err(|e| e.to_string())?;
    }
    *state.client.write() = None;

    // Stop host
    if let Some(host) = state.host.read().as_ref() {
        host.stop();
    }
    *state.host.write() = None;

    *state.is_in_room.write() = false;
    *state.is_host.write() = false;

    tracing::info!("Left room");
    Ok(())
}

/// Get current room state
#[tauri::command]
pub async fn get_room_state(state: State<'_, AppState>) -> Result<Option<RoomInfo>, String> {
    if !*state.is_in_room.read() {
        return Ok(None);
    }

    if let Some(host) = state.host.read().as_ref() {
        let room_state = host.state();
        let users: Vec<UserEntry> = room_state.user_list().into_iter().map(|u| UserEntry {
            user_id: u.user_id,
            username: u.username,
            is_muted: u.is_muted,
            is_host: u.is_host,
        }).collect();

        return Ok(Some(RoomInfo {
            room_name: room_state.config.room_name,
            user_count: users.len(),
            is_host: true,
            users,
        }));
    }

    Ok(None)
}
