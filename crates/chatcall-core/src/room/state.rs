use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chatcall_net::protocol::packet::UserInfo;

/// Room configuration provided when creating a room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub room_name: String,
    pub host_name: String,
    pub max_users: u8,
    pub tcp_port: u16,
    pub udp_port: u16,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            room_name: "ChatCall Room".to_string(),
            host_name: "Host".to_string(),
            max_users: 20,
            tcp_port: chatcall_net::DEFAULT_TCP_PORT,
            udp_port: chatcall_net::DEFAULT_UDP_PORT,
        }
    }
}

/// Current state of a room
#[derive(Debug, Clone)]
pub struct RoomState {
    pub config: RoomConfig,
    pub users: HashMap<u16, UserInfo>,
    pub next_user_id: u16,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl RoomState {
    pub fn new(config: RoomConfig) -> Self {
        Self {
            config,
            users: HashMap::new(),
            next_user_id: 1, // 0 is reserved for host
            is_active: true,
            created_at: chrono::Utc::now(),
        }
    }

    /// Add a user and return their assigned user_id
    pub fn add_user(&mut self, username: String) -> Option<u16> {
        if self.users.len() >= self.config.max_users as usize {
            return None; // room full
        }

        let user_id = self.next_user_id;
        self.next_user_id += 1;

        self.users.insert(user_id, UserInfo {
            user_id,
            username,
            is_muted: false,
            is_host: false,
        });

        Some(user_id)
    }

    /// Remove a user
    pub fn remove_user(&mut self, user_id: u16) -> Option<UserInfo> {
        self.users.remove(&user_id)
    }

    /// Get user info
    pub fn get_user(&self, user_id: u16) -> Option<&UserInfo> {
        self.users.get(&user_id)
    }

    /// Update user mute state
    pub fn set_user_muted(&mut self, user_id: u16, muted: bool) {
        if let Some(user) = self.users.get_mut(&user_id) {
            user.is_muted = muted;
        }
    }

    /// Get all users as a list
    pub fn user_list(&self) -> Vec<UserInfo> {
        self.users.values().cloned().collect()
    }

    /// Get user count
    pub fn user_count(&self) -> usize {
        self.users.len()
    }

    /// Check if room is full
    pub fn is_full(&self) -> bool {
        self.users.len() >= self.config.max_users as usize
    }
}
