use tokio::sync::broadcast;

/// Events emitted by the room system for UI updates
#[derive(Debug, Clone)]
pub enum RoomEvent {
    /// Room was created (host only)
    RoomCreated { room_name: String },
    /// Room was closed (host only)
    RoomClosed,
    /// Successfully connected to a room (client only)
    Connected { user_id: u16, room_name: String },
    /// Disconnected from room
    Disconnected,
    /// A user joined the room
    UserJoined { user_id: u16, username: String },
    /// A user left the room
    UserLeft { user_id: u16, username: String },
    /// User mute state changed
    UserMuteChanged { user_id: u16, is_muted: bool },
    /// Chat message received
    ChatMessageReceived { user_id: u16, data: Vec<u8> },
    /// Chat message sent successfully (ACK received)
    ChatMessageAcked { message_id: [u8; 16] },
    /// Chat message delivery failed
    ChatMessageFailed { message_id: [u8; 16] },
    /// Voice activity changed for a user
    VoiceActivity { user_id: u16, is_speaking: bool },
    /// Latency updated for a user
    LatencyUpdated { user_id: u16, latency_ms: u32 },
    /// Error occurred
    Error { message: String },
}

/// Type alias for the event sender
pub type EventSender = broadcast::Sender<RoomEvent>;

/// Type alias for the event receiver
pub type EventReceiver = broadcast::Receiver<RoomEvent>;

/// Create a new event channel with default capacity
pub fn create_event_channel() -> (EventSender, EventReceiver) {
    broadcast::channel(256)
}
