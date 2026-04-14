use std::sync::Arc;
use parking_lot::RwLock;
use tauri::AppHandle;
use chatcall_core::room::host::RoomHost;
use chatcall_core::room::client::RoomClient;
use chatcall_core::chat::history::ChatHistory;
use chatcall_core::user::profile::UserProfile;
use chatcall_core::events::{self, EventSender, EventReceiver};
use chatcall_audio::pipeline::VoicePipeline;

/// Application state managed by Tauri
pub struct AppState {
    pub app_handle: AppHandle,
    pub profile: Arc<RwLock<UserProfile>>,
    pub host: Arc<RwLock<Option<RoomHost>>>,
    pub client: Arc<RwLock<Option<RoomClient>>>,
    pub voice_pipeline: Arc<RwLock<Option<VoicePipeline>>>,
    pub chat_history: Arc<RwLock<Option<ChatHistory>>>,
    pub event_tx: EventSender,
    pub is_in_room: Arc<RwLock<bool>>,
    pub is_host: Arc<RwLock<bool>>,
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> Self {
        let (event_tx, _event_rx) = events::create_event_channel();

        Self {
            app_handle,
            profile: Arc::new(RwLock::new(UserProfile::default())),
            host: Arc::new(RwLock::new(None)),
            client: Arc::new(RwLock::new(None)),
            voice_pipeline: Arc::new(RwLock::new(None)),
            chat_history: Arc::new(RwLock::new(None)),
            event_tx,
            is_in_room: Arc::new(RwLock::new(false)),
            is_host: Arc::new(RwLock::new(false)),
        }
    }
}
