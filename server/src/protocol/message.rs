//! Message structures.
//!
//! This module defines the core message structures that form the logical
//! communication layer between the local server engine and connected clients.
//! These structures are high-level event representations, often containing detailed
//! metadata (like timestamps and addresses) for processing the message routing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message destination.
///
/// Dictates the target audience for a specific broadcast or message mechanism.
/// For now, only broadcast messages are supported, but targeted messages
/// will be implemented in the future.
/// TODO: Targeted messages
#[derive(Debug, Clone)]
pub enum MessageDestination {
    /// Broadcast the message to all currently connected clients.
    All,
}

/// The actual data payload of a message traversing the protocol.
///
/// Can contain various forms of media, primarily plain text or raw binary blobs.
/// TODO: Add support for Arbitrary binary data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// UTF-8 encoded text
    Text(String),
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
    pub destination: MessageDestination,
    /// The inner payload data.
    pub content: MessageContent,
}

impl Message {
    /// Create a new message structure utilizing the current UTC timestamp.
    pub fn new(destination: MessageDestination, content: MessageContent) -> Self {
        Self {
            timestamp: Utc::now(),
            destination,
            content,
        }
    }
}
