use chrono::{DateTime, Utc};
use server::remote;

#[derive(Debug)]
pub struct ReceivedMessage {
    // TODO: Use local time
    pub datetime: DateTime<Utc>,
    pub message: remote::packet::OutgoingMessage,
}

/// Chat history
#[derive(Debug)]
pub struct ChatHistory {
    pub messages: Vec<ReceivedMessage>,
}
