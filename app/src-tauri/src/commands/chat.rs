use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatEntry {
    pub id: String,
    pub username: String,
    pub text: String,
    pub timestamp: String,
}

/// Send a chat message
#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    text: String,
) -> Result<String, String> {
    if !*state.is_in_room.read() {
        return Err("Not in a room".to_string());
    }

    let username = state.profile.read().username.clone();
    tracing::debug!("{}: {}", username, text);

    // For the MVP, messages are sent via the event system.
    // Full implementation will use `RoomClient::send_chat()` on the client thread.
    Ok(uuid::Uuid::new_v4().to_string())
}

/// Get chat history
#[tauri::command]
pub async fn get_messages(
    _state: State<'_, AppState>,
    _limit: Option<usize>,
) -> Result<Vec<ChatEntry>, String> {
    // Messages come from the real-time event system, not polling
    Ok(vec![])
}
