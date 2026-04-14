use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;
use chatcall_core::user::profile::UserProfile;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub username: String,
    pub avatar_color: String,
}

/// Get user profile
#[tauri::command]
pub async fn get_profile(state: State<'_, AppState>) -> Result<ProfileInfo, String> {
    let profile = state.profile.read();
    Ok(ProfileInfo {
        username: profile.username.clone(),
        avatar_color: profile.avatar_color.clone(),
    })
}

/// Set username
#[tauri::command]
pub async fn set_username(
    state: State<'_, AppState>,
    username: String,
) -> Result<ProfileInfo, String> {
    let new_profile = UserProfile::new(username);
    let info = ProfileInfo {
        username: new_profile.username.clone(),
        avatar_color: new_profile.avatar_color.clone(),
    };
    *state.profile.write() = new_profile;
    Ok(info)
}
