//! Remote
//! Messages between local client thread and remote client

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RemoteMessage {
    /// Timestamp
    pub timestamp: i64,
    /// Message content
    pub content: String,
}

impl RemoteMessage {
    pub fn new(content: String) -> Self {
        Self {
            timestamp: Utc::now().timestamp(),
            content,
        }
    }
}
