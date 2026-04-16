use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::protocol;

/// Fundamental connection control signals from the server to the client.
///
/// These differ from protocol-level messages as they are used to manage the
/// remote socket lifecycle, notifying the far-end about state changes instead
/// of relaying peer data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Indicates the handshake was valid and the connection is active.
    Welcome(u64),
    /// Indicates the server is actively dropping the client's connection.
    Disconnect,
}

/// The unified message type that the server blasts outwards to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutgoingMessage {
    /// A system-level administrative message (e.g. Welcome/Disconnect).
    ServerMessage(ServerMessage),
    /// Relayed application-level data from another user acting as a peer.
    PeerMessage {
        /// The unique integer identifier of the peer who generated the content.
        author_id: u64,
        /// The inner data payload (text, binary, etc).
        content: protocol::message::MessageContent,
    },
}

/// The wire-level envelope dispatched from the Server strictly to the Client.
///
/// Contains the payload alongside synchronization data like timestamps.
/// The entire struct derives `Serialize` to be directly converted to CBOR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingPacket {
    /// UTC timestamp indicating when the server created this packet.
    /// Uses primitive `i64` (typical UNIX seconds/millis) for compact serialization.
    pub timestamp: i64,
    /// The encapsulated data destined for the client.
    pub message: OutgoingMessage,
}

impl OutgoingPacket {
    /// Instantiates a new packet attached to the current UTC Unix timestamp.
    pub fn new(message: OutgoingMessage) -> Self {
        Self {
            timestamp: Utc::now().timestamp(),
            message,
        }
    }
}

/// The wire-level envelope received by the Server explicitly from the Client.
///
/// Clients transmit this structure upwards to relay application data
/// into the server's routing core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingPacket {
    /// UTC timestamp indicating when the client originally pushed this data.
    pub timestamp: i64,
    /// The intended payload (text message, etc.) the client wishes to send.
    pub message: protocol::message::MessageContent,
}
