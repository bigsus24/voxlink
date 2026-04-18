use std::sync::Arc;
use parking_lot::{RwLock, Mutex};
use tauri::AppHandle;
use chatcall_core::user::profile::UserProfile;
use chatcall_core::events::{self, EventSender};
use chatcall_core::room::host::RoomHost;

/// Application state managed by Tauri.
pub struct AppState {
    pub app_handle: AppHandle,
    pub profile: Arc<RwLock<UserProfile>>,
    pub event_tx: EventSender,
    pub is_in_room: Arc<RwLock<bool>>,
    pub is_host: Arc<RwLock<bool>>,
    pub is_muted: Arc<RwLock<bool>>,
    pub room_name: Arc<RwLock<Option<String>>>,
    /// The 7-char VoxCode generated when hosting a room
    pub room_code: Arc<RwLock<Option<String>>>,
    /// The active RoomHost — stored so we can call stop() to release ports
    pub room_host: Arc<Mutex<Option<RoomHost>>>,
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> Self {
        let (event_tx, _event_rx) = events::create_event_channel();

        Self {
            app_handle,
            profile: Arc::new(RwLock::new(UserProfile::default())),
            event_tx,
            is_in_room: Arc::new(RwLock::new(false)),
            is_host: Arc::new(RwLock::new(false)),
            is_muted: Arc::new(RwLock::new(false)),
            room_name: Arc::new(RwLock::new(None)),
            room_code: Arc::new(RwLock::new(None)),
            room_host: Arc::new(Mutex::new(None)),
        }
    }
}
