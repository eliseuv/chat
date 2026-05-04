use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::protocol::{ChatMessage, MessageContent};

/// Fundamental connection control signals from the server to the client.
///
/// These differ from protocol-level messages as they are used to manage the
/// remote socket lifecycle, notifying the far-end about state changes instead
/// of relaying peer data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerCommand {
    /// Indicates the handshake was valid and the connection is active.
    Welcome(u64),
    /// Indicates the server is actively dropping the client's connection.
    Disconnect,
}

/// The unified message type that the server blasts outwards to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// A system-level administrative message (e.g. Welcome/Disconnect).
    Command(ServerCommand),
    /// Relayed application-level data from another user acting as a peer.
    Chat(ChatMessage),
}

/// The wire-level envelope dispatched from the Server strictly to the Client.
///
/// Contains the payload alongside synchronization data like timestamps.
/// The entire struct derives `Serialize` to be directly converted to CBOR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRemotePacket {
    /// UTC timestamp indicating when the server created this packet.
    pub timestamp: i64,
    /// The encapsulated data destined for the client.
    pub message: ServerMessage,
}

impl ServerRemotePacket {
    /// Instantiates a new packet attached to the current UTC Unix timestamp.
    pub fn new(message: ServerMessage) -> Self {
        Self {
            timestamp: Utc::now().timestamp_millis(),
            message,
        }
    }
}

/// The wire-level envelope received by the Server explicitly from the Client.
///
/// Clients transmit this structure upwards to relay application data
/// into the server's routing core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRemotePacket {
    /// Untrusted. Just used for roundtrip time measurement.
    pub timestamp: i64,
    /// The intended payload (text message, etc.) the client wishes to send.
    pub message_content: MessageContent,
}
