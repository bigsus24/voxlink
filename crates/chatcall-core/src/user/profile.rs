use serde::{Serialize, Deserialize};

/// Local user profile stored on the user's machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub username: String,
    pub avatar_color: String, // hex color for avatar placeholder
}

impl UserProfile {
    pub fn new(username: String) -> Self {
        // Generate a deterministic color from the username
        let hash = username.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        let hue = hash % 360;
        let color = format!("hsl({}, 70%, 60%)", hue);

        Self {
            username,
            avatar_color: color,
        }
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self::new("User".to_string())
    }
}
