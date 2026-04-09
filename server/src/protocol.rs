//! Protocol
//! Internal messages between client and server

use std::net::SocketAddr;

use chrono::{DateTime, Utc};

/// Message destination
#[derive(Debug, Clone)]
pub enum Destination {
    /// All clients
    All,
    /// Specific client
    Client(SocketAddr),
}

/// Message content
#[derive(Debug, Clone)]
pub enum MessageContent {
    /// Text message
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
}

/// Message
#[derive(Debug, Clone)]
pub struct Message {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Destination
    pub destination: Destination,
    /// Content
    pub content: MessageContent,
}

impl Message {
    /// Create a new message
    pub fn new(destination: Destination, content: MessageContent) -> Self {
        Self {
            timestamp: Utc::now(),
            destination,
            content,
        }
    }
}

/// Clients -> Server
#[derive(Debug, Clone)]
pub enum Request {
    /// Connection request
    Connect,
    /// Disconnection request
    Disconnect,
    /// Message between clients
    Message(Message),
}

/// Clients -> Server
#[derive(Debug, Clone)]
pub struct ClientRequest {
    /// Client address
    pub addr: SocketAddr,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Message
    pub request: Request,
}

/// Server -> Clients
#[derive(Debug, Clone)]
pub enum Response {
    /// Welcome message
    Welcome(SocketAddr),
    /// Disconnect message
    Disconnect(Destination),
    /// Message between clients
    Message {
        /// Sender address
        sender: SocketAddr,
        /// Message content
        content: MessageContent,
    },
}

/// Server -> Clients
#[derive(Debug, Clone)]
pub struct ServerResponse {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Response
    pub response: Response,
}
