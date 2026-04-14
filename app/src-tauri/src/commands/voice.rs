use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;

/// Toggle mute state
#[tauri::command]
pub async fn toggle_mute(state: State<'_, AppState>) -> Result<bool, String> {
    let mut muted = state.is_muted.write();
    *muted = !*muted;
    let new_state = *muted;
    tracing::debug!("Mute toggled: {}", new_state);
    Ok(new_state)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_input: bool,
}

/// Get available audio devices
#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();

    match chatcall_audio::capture::AudioCapture::list_devices() {
        Ok(input_devices) => {
            for name in input_devices {
                devices.push(AudioDevice { name, is_input: true });
            }
        }
        Err(e) => tracing::warn!("Failed to list input devices: {}", e),
    }

    match chatcall_audio::playback::AudioPlayback::list_devices() {
        Ok(output_devices) => {
            for name in output_devices {
                devices.push(AudioDevice { name, is_input: false });
            }
        }
        Err(e) => tracing::warn!("Failed to list output devices: {}", e),
    }

    Ok(devices)
}
