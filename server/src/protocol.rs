//! Protocol
//! Internal messages between client and server
//!
//! This module defines the core message structures that form the logical
//! communication layer between the local server engine and connected clients.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

/// Message destination.
///
/// Dictates the target audience for a specific broadcast or message mechanism.
/// For now, only broadcast messages are supported, but targeted messages
/// will be implemented in the future.
/// TODO: Targeted messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDestination {
    /// Broadcast the message to all currently connected clients.
    AllUsers,
}

/// Message content
///
/// Actual message payload.
/// For now, only text messages are supported, but in the future, support
/// for arbitrary binary data will be added.
/// TODO: Add support for arbitrary binary data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// UTF-8 encoded text
    Text(String),
}

/// A standard multi-purpose message representing data flow between entities.
///
/// This provides the entire context needed for the server to handle routing
/// without requiring the connection logic to peek inside the payload content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub author_id: u64,
    pub author_username: String,
    /// Where this message should be delivered.
    pub destination: MessageDestination,
    /// The inner payload data.
    pub content: MessageContent,
}

/// A top-level logical request sourced from a client and directed to the server.
///
/// Request acts as the core intent enum, enabling the client to state what
/// actions it is hoping the server will take on its behalf (like joining, leaving, or sending).
#[derive(Debug, Clone)]
pub enum Request {
    /// Signals the intent to connect and start listening for updates.
    Connect { username: String },
    /// Requests the list of currently connected active users.
    GetActiveUsers,
    /// Signals the intent to disconnect from the server and terminate the session.
    Disconnect,
    /// Requests the server to route a specific `Message`.
    Message(ChatMessage),
}

/// An envelope wrapping a client's [`Request`] with contextual origin tracking.
#[derive(Debug, Clone)]
pub struct ClientRequest {
    /// The unique id of the client that emitted this request.
    pub client_id: u64,
    /// The address of the client.
    pub client_addr: SocketAddr,
    /// The underlying action the client wishes to perform.
    pub request: Request,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoginError {
    UsernameTaken,
    InvalidUsername,
    EmptyUsername,
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::UsernameTaken => write!(f, "Username is already taken"),
            LoginError::InvalidUsername => write!(f, "Invalid username format"),
            LoginError::EmptyUsername => write!(f, "Username cannot be empty"),
        }
    }
}

/// A top-level logical response from the server directed towards connected clients.
///
/// This enum groups together different types of instructions and information
/// that the server pushes downwards to the clients, such as system events or peer data.
#[derive(Debug, Clone)]
pub enum Response {
    /// Indicates that the connection was successfully registered.
    /// Provides the client with its assigned user id.
    Welcome(u64),
    /// Indicates that the login was rejected.
    LoginReject { client_id: u64, error: LoginError },
    /// Tells the client about the current active usernames.
    ActiveUsers { usernames: Vec<String> },
    /// Indicates that a user joined the chat room.
    Joined { username: String },
    /// Indicates that a user left the chat room.
    Left { username: String },
    /// Instructs the client to close the connection.
    Disconnect(SocketAddr),
    /// A routed message originating from another peer on the network.
    Message(ChatMessage),
}
