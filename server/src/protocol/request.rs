use std::net::SocketAddr;
use chrono::{DateTime, Utc};
use super::message::Message;

/// A top-level logical request sourced from a client and directed to the server.
///
/// Request acts as the core intent enum, enabling the client to state what 
/// actions it is hoping the server will take on its behalf (like joining, leaving, or sending).
#[derive(Debug, Clone)]
pub enum Request {
    /// Signals the intent to connect and start listening for updates.
    Connect,
    /// Signals the intent to disconnect from the server and terminate the session.
    Disconnect,
    /// Requests the server to route a specific `Message` to other peers.
    Message(Message),
}

/// An envelope wrapping a client's [`Request`] with contextual origin tracking.
#[derive(Debug, Clone)]
pub struct ClientRequest {
    /// The unique sequential ID of the client that emitted this request.
    pub client_id: u64,
    /// The socket address of the client that emitted this request.
    pub addr: SocketAddr,
    /// The exact server-time when this request envelope was assembled internally.
    pub timestamp: DateTime<Utc>,
    /// The underlying action the client wishes to perform.
    pub request: Request,
}
