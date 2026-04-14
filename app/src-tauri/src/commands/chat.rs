use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatEntry {
    pub id: String,
    pub user_id: u16,
    pub username: String,
    pub text: String,
    pub timestamp: String,
    pub status: String,
}

/// Send a chat message
#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    text: String,
) -> Result<String, String> {
    if let Some(client) = state.client.write().as_mut() {
        let msg_id = client.send_chat(&text).await.map_err(|e| e.to_string())?;
        let hex: String = msg_id.iter().map(|b| format!("{:02x}", b)).collect();
        Ok(hex)
    } else {
        Err("Not connected to a room".to_string())
    }
}

/// Get chat history
#[tauri::command]
pub async fn get_messages(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<ChatEntry>, String> {
    // Return empty for now — messages will come from the event system
    Ok(vec![])
}
