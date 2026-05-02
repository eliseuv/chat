use super::message::{Destination, MessageContent};
use chrono::{DateTime, Utc};
use std::net::SocketAddr;

/// A top-level logical response from the server directed towards connected clients.
///
/// This enum groups together different types of instructions and information
/// that the server pushes downwards to the clients, such as system events or peer data.
#[derive(Debug, Clone)]
pub enum ResponseType {
    /// Indicates that the connection was successfully registered.
    /// Provides the client with its assigned user id.
    Welcome(u64),
    /// Instructs the client(s) to gracefully close the connection.
    Disconnect(Destination),
    /// A routed message originating from another peer on the network.
    Message {
        /// The origin address of the peer who sent this message.
        sender: SocketAddr,
        /// The unique sequential ID of the peer who sent this message.
        sender_id: u64,
        /// The actual payload data of the message.
        content: MessageContent,
    },
}

/// An envelope wrapping a [`Response`] with server-side chronological data.
#[derive(Debug, Clone)]
pub struct Response {
    /// The exact server-side time when this response was generated.
    pub timestamp: DateTime<Utc>,
    /// The inner response payload.
    pub response_type: ResponseType,
}

impl Response {
    pub fn new(response_type: ResponseType) -> Self {
        Self {
            timestamp: Utc::now(),
            response_type,
        }
    }
}
