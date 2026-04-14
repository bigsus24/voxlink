use rusqlite::Connection;
use std::path::PathBuf;
use crate::user::profile::UserProfile;

/// Error type for database operations
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(String),
}

/// Local SQLite database for application settings and user data.
/// All data is stored on the user's machine — no cloud services.
pub struct AppDatabase {
    conn: Connection,
}

impl AppDatabase {
    /// Open or create the app database
    pub fn open(db_path: PathBuf) -> Result<Self, DatabaseError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;

        Ok(Self { conn })
    }

    /// Save a setting
    pub fn set(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// Get a setting
    pub fn get(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let result = self.conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        );

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    /// Save user profile
    pub fn save_profile(&self, profile: &UserProfile) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(profile)
            .map_err(|e| DatabaseError::Serde(e.to_string()))?;
        self.set("user_profile", &json)
    }

    /// Load user profile
    pub fn load_profile(&self) -> Result<Option<UserProfile>, DatabaseError> {
        match self.get("user_profile")? {
            Some(json) => {
                let profile = serde_json::from_str(&json)
                    .map_err(|e| DatabaseError::Serde(e.to_string()))?;
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }
}
