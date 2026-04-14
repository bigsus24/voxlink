use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;
use chatcall_core::room::host::RoomHost;
use chatcall_core::room::client::RoomClient;
use chatcall_core::room::state::RoomConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_name: String,
    pub is_host: bool,
}

/// Create and host a new room
#[tauri::command]
pub async fn create_room(
    state: State<'_, AppState>,
    room_name: String,
) -> Result<RoomInfo, String> {
    let username = state.profile.read().username.clone();

    let config = RoomConfig {
        room_name: room_name.clone(),
        host_name: username,
        max_users: 20,
        tcp_port: chatcall_net::DEFAULT_TCP_PORT,
        udp_port: chatcall_net::DEFAULT_UDP_PORT,
    };

    let host = RoomHost::new(config, state.event_tx.clone());
    host.start().await.map_err(|e| e.to_string())?;

    // Host runs in background via spawned tokio tasks.
    // We don't store it in state (not Send+Sync), but it stays alive
    // via the spawned tasks. We leak it intentionally — it will be
    // cleaned up when the tasks end (on room close).
    let host = Box::leak(Box::new(host));
    let _ = host; // kept alive

    *state.is_in_room.write() = true;
    *state.is_host.write() = true;
    *state.room_name.write() = Some(room_name.clone());

    tracing::info!("Room '{}' created", room_name);

    Ok(RoomInfo {
        room_name,
        is_host: true,
    })
}

/// Join an existing room by address
#[tauri::command]
pub async fn join_room(
    state: State<'_, AppState>,
    host_address: String,
) -> Result<RoomInfo, String> {
    let port = chatcall_net::DEFAULT_TCP_PORT;
    let addr = format!("{}:{}", host_address, port)
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;

    let username = state.profile.read().username.clone();
    let mut client = RoomClient::new(username, state.event_tx.clone());
    client.connect(addr).await.map_err(|e| e.to_string())?;

    let room_name = client.room_name().unwrap_or("Room").to_string();

    *state.is_in_room.write() = true;
    *state.is_host.write() = false;
    *state.room_name.write() = Some(room_name.clone());

    // Client runs via spawned tasks too — leak to keep alive
    let _ = Box::leak(Box::new(client));

    tracing::info!("Joined room '{}'", room_name);

    Ok(RoomInfo {
        room_name,
        is_host: false,
    })
}

/// Leave the current room
#[tauri::command]
pub async fn leave_room(state: State<'_, AppState>) -> Result<(), String> {
    *state.is_in_room.write() = false;
    *state.is_host.write() = false;
    *state.room_name.write() = None;

    tracing::info!("Left room");
    Ok(())
}

/// Get current room state
#[tauri::command]
pub async fn get_room_state(state: State<'_, AppState>) -> Result<Option<RoomInfo>, String> {
    let is_in_room = *state.is_in_room.read();
    if !is_in_room {
        return Ok(None);
    }

    let room_name = state.room_name.read().clone().unwrap_or_default();
    let is_host = *state.is_host.read();

    Ok(Some(RoomInfo {
        room_name,
        is_host,
    }))
}
