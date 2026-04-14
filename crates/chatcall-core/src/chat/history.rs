use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::chat::message::{ChatMessage, MessageStatus};
use chrono::{DateTime, Utc};

/// Error type for chat history operations
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Local SQLite-backed chat history storage.
///
/// All messages are stored locally on the user's machine.
/// No cloud services or remote APIs are used.
pub struct ChatHistory {
    conn: Connection,
}

impl ChatHistory {
    /// Open or create the chat history database at the given path
    pub fn open(db_path: PathBuf) -> Result<Self, HistoryError> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // Create table if it doesn't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id          BLOB PRIMARY KEY,
                user_id     INTEGER NOT NULL,
                username    TEXT NOT NULL,
                text        TEXT NOT NULL,
                timestamp   TEXT NOT NULL,
                status      INTEGER NOT NULL DEFAULT 1,
                room_name   TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_messages_timestamp
                ON messages(timestamp);

            CREATE INDEX IF NOT EXISTS idx_messages_room
                ON messages(room_name);
            "
        )?;

        Ok(Self { conn })
    }

    /// Open an in-memory database (for testing)
    pub fn in_memory() -> Result<Self, HistoryError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE messages (
                id          BLOB PRIMARY KEY,
                user_id     INTEGER NOT NULL,
                username    TEXT NOT NULL,
                text        TEXT NOT NULL,
                timestamp   TEXT NOT NULL,
                status      INTEGER NOT NULL DEFAULT 1,
                room_name   TEXT
            );"
        )?;
        Ok(Self { conn })
    }

    /// Save a message to the database
    pub fn save_message(&self, msg: &ChatMessage, room_name: &str) -> Result<(), HistoryError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO messages (id, user_id, username, text, timestamp, status, room_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                msg.id.as_slice(),
                msg.user_id as i64,
                msg.username,
                msg.text,
                msg.timestamp.to_rfc3339(),
                msg.status as i32,
                room_name,
            ],
        )?;
        Ok(())
    }

    /// Update message status
    pub fn update_status(&self, message_id: &[u8; 16], status: MessageStatus) -> Result<(), HistoryError> {
        self.conn.execute(
            "UPDATE messages SET status = ?1 WHERE id = ?2",
            params![status as i32, message_id.as_slice()],
        )?;
        Ok(())
    }

    /// Get recent messages for a room (most recent first)
    pub fn get_messages(&self, room_name: &str, limit: usize) -> Result<Vec<ChatMessage>, HistoryError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, username, text, timestamp, status
             FROM messages
             WHERE room_name = ?1
             ORDER BY timestamp DESC
             LIMIT ?2"
        )?;

        let messages = stmt.query_map(params![room_name, limit as i64], |row| {
            let id_blob: Vec<u8> = row.get(0)?;
            let mut id = [0u8; 16];
            id.copy_from_slice(&id_blob);

            let user_id: i64 = row.get(1)?;
            let username: String = row.get(2)?;
            let text: String = row.get(3)?;
            let timestamp_str: String = row.get(4)?;
            let status_int: i32 = row.get(5)?;

            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let status = match status_int {
                0 => MessageStatus::Sending,
                1 => MessageStatus::Delivered,
                _ => MessageStatus::Failed,
            };

            Ok(ChatMessage {
                id,
                user_id: user_id as u16,
                username,
                text,
                timestamp,
                status,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        // Reverse to get chronological order
        let mut messages = messages;
        messages.reverse();
        Ok(messages)
    }

    /// Get total message count for a room
    pub fn message_count(&self, room_name: &str) -> Result<usize, HistoryError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE room_name = ?1",
            params![room_name],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Delete all messages for a room
    pub fn clear_room(&self, room_name: &str) -> Result<(), HistoryError> {
        self.conn.execute(
            "DELETE FROM messages WHERE room_name = ?1",
            params![room_name],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_retrieve() {
        let history = ChatHistory::in_memory().unwrap();
        let msg = ChatMessage::new(1, "alice".to_string(), "Hello!".to_string());

        history.save_message(&msg, "test-room").unwrap();

        let messages = history.get_messages("test-room", 10).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hello!");
        assert_eq!(messages[0].username, "alice");
    }

    #[test]
    fn test_message_ordering() {
        let history = ChatHistory::in_memory().unwrap();

        for i in 0..5 {
            let msg = ChatMessage::new(1, "bob".to_string(), format!("Message {}", i));
            history.save_message(&msg, "room1").unwrap();
        }

        let messages = history.get_messages("room1", 10).unwrap();
        assert_eq!(messages.len(), 5);
        // Should be in chronological order
        assert!(messages[0].text.contains("0"));
    }

    #[test]
    fn test_room_isolation() {
        let history = ChatHistory::in_memory().unwrap();

        let msg1 = ChatMessage::new(1, "alice".to_string(), "Room A".to_string());
        let msg2 = ChatMessage::new(2, "bob".to_string(), "Room B".to_string());

        history.save_message(&msg1, "room-a").unwrap();
        history.save_message(&msg2, "room-b").unwrap();

        let a = history.get_messages("room-a", 10).unwrap();
        let b = history.get_messages("room-b", 10).unwrap();

        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a[0].text, "Room A");
        assert_eq!(b[0].text, "Room B");
    }
}
