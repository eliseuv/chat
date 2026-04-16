use std::net::SocketAddr;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message destination.
///
/// Dictates the target audience for a specific broadcast or message mechanism.
#[derive(Debug, Clone)]
pub enum Destination {
    /// Broadcast the message to all currently connected clients.
    All,
    /// Send the message exclusively to a targeted client, identified by its Socket Address.
    Client(SocketAddr),
}

/// The actual data payload of a message traversing the protocol.
///
/// Can contain various forms of media, primarily plain text or raw binary blobs.
/// This enum requires `Serialize` and `Deserialize` because it explicitly traverses
/// the network boundaries inside the packet structures in `remote.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// A UTF-8 encoded text string. Used for standard chat messages.
    Text(String),
    /// Arbitrary binary data. Used for files, images, or custom binary structures.
    Binary(Vec<u8>),
}

/// A standard multi-purpose message representing data flow between entities.
///
/// This provides the entire context needed for the server to handle routing
/// without requiring the connection logic to peek inside the payload content.
#[derive(Debug, Clone)]
pub struct Message {
    /// The moment this message was created or received.
    pub timestamp: DateTime<Utc>,
    /// Where this message should be delivered.
    pub destination: Destination,
    /// The inner payload data.
    pub content: MessageContent,
}

impl Message {
    /// Create a new message structure utilizing the current UTC timestamp.
    pub fn new(destination: Destination, content: MessageContent) -> Self {
        Self {
            timestamp: Utc::now(),
            destination,
            content,
        }
    }
}
