use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// A chat message in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message identifier (UUID bytes)
    pub id: [u8; 16],
    /// User ID of the sender
    pub user_id: u16,
    /// Display name of the sender
    pub username: String,
    /// Message text content
    pub text: String,
    /// Timestamp when the message was sent
    pub timestamp: DateTime<Utc>,
    /// Delivery status
    pub status: MessageStatus,
}

/// Message delivery status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    /// Message is being sent
    Sending,
    /// Message was sent and ACK'd
    Delivered,
    /// Message delivery failed after retries
    Failed,
}

impl ChatMessage {
    /// Create a new outgoing message
    pub fn new(user_id: u16, username: String, text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            user_id,
            username,
            text,
            timestamp: Utc::now(),
            status: MessageStatus::Sending,
        }
    }

    /// Create from received data
    pub fn received(id: [u8; 16], user_id: u16, username: String, text: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            id,
            user_id,
            username,
            text,
            timestamp,
            status: MessageStatus::Delivered,
        }
    }

    /// Convert message ID to hex string for display
    pub fn id_hex(&self) -> String {
        self.id.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
