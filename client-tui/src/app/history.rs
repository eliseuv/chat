use chrono::{DateTime, Local};
use server::remote;
use server::protocol::LoginError;

#[derive(Debug)]
pub struct ReceivedMessage {
    pub datetime: DateTime<Local>,
    pub message: remote::packet::ServerMessage,
}

/// Chat history
#[derive(Debug)]
pub struct ChatHistory {
    pub messages: Vec<ReceivedMessage>,
    pub own_id: Option<u64>,
    pub own_username: Option<String>,
    pub login_error: Option<LoginError>,
    pub active_usernames: Vec<String>,
}

