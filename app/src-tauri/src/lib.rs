pub mod commands;
pub mod state;

use tauri::Manager;

use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("chatcall=debug,info"))
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            // Initialize app state
            let app_state = state::AppState::new(app.handle().clone());
            app.manage(app_state);

            tracing::info!("ChatCall started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::room::create_room,
            commands::room::join_room,
            commands::room::leave_room,
            commands::room::get_room_state,
            commands::chat::send_message,
            commands::chat::get_messages,
            commands::voice::toggle_mute,
            commands::voice::get_audio_devices,
            commands::settings::get_profile,
            commands::settings::set_username,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ChatCall");
}
