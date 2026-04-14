use tauri::State;
use serde::{Serialize, Deserialize};
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_input: bool,
}

/// Toggle mute state
#[tauri::command]
pub async fn toggle_mute(state: State<'_, AppState>) -> Result<bool, String> {
    if let Some(pipeline) = state.voice_pipeline.write().as_mut() {
        let new_mute = !pipeline.is_muted();
        pipeline.set_muted(new_mute);
        Ok(new_mute)
    } else {
        Ok(false)
    }
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
