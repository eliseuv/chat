//! Client
//! Worker thread for each client connection

use std::{
    fmt::Display,
    net::SocketAddr,
    sync::atomic::{self, AtomicU64},
};

use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc},
};

use anyhow::bail;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use tokio_util::codec::Framed;

use crate::protocol::{
    message::{Destination, Message},
    request::{ClientRequest, Request},
    response::{Response, ResponseType},
};
use crate::remote::codec::ServerCodec;
use crate::remote::packet::{OutgoingMessage, OutgoingPacket, ServerMessage};

/// Client ID counter
static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Represents the core identity of a client (ID and address).
/// This is isolated into a separate, `Copy`able struct to avoid partial move
/// errors in the main `Client` loop. When `Client::stream` is consumed by `Framed::new`,
/// the `Client` struct becomes partially moved, which bans calling methods on `&self`.
/// By copying this sub-struct instead, we can continuously stamp outgoing messages
/// with identity metadata without needing to borrow `self`.
#[derive(Debug, Clone, Copy)]
pub struct ClientIdentity {
    /// Unique ID
    pub id: u64,
    /// Client address
    pub addr: SocketAddr,
}

impl ClientIdentity {
    /// Wraps a raw `Request` into a `ClientRequest` by attaching the client's
    /// identity (ID and address) and the current timestamp.
    ///
    /// Takes `self` by value (a cheap copy) rather than `&self` to ensure we can
    /// always call this even if the parent `Client` is partially moved.
    pub fn wrap_request(self, request: Request) -> ClientRequest {
        ClientRequest {
            client_id: self.id,
            addr: self.addr,
            timestamp: Utc::now(),
            request,
        }
    }
}

/// Represents a client connection
#[derive(Debug)]
pub struct Client {
    pub identity: ClientIdentity,
    /// Client stream
    pub stream: TcpStream,
    /// Command sender
    pub cmd_tx: mpsc::Sender<ClientRequest>,
    /// Broadcast receiver
    pub bcast_rx: broadcast::Receiver<Response>,
}

impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Client {id} [{addr}]",
            id = self.identity.id,
            addr = self.identity.addr
        )
    }
}

impl Client {
    pub fn new(
        addr: SocketAddr,
        stream: TcpStream,
        cmd_tx: mpsc::Sender<ClientRequest>,
        bcast_tx: &broadcast::Sender<Response>,
    ) -> Self {
        Self {
            identity: ClientIdentity {
                id: CLIENT_ID_COUNTER.fetch_add(1, atomic::Ordering::SeqCst),
                addr,
            },
            stream,
            cmd_tx,
            bcast_rx: bcast_tx.subscribe(),
        }
    }

    pub async fn run(mut self) -> Result<(), anyhow::Error> {
        let client_name = self.to_string();
        log::info!("[{client_name}] Started");

        match self
            .cmd_tx
            .send(self.identity.wrap_request(Request::Connect))
            .await
        {
            Err(e) => bail!("Failed to send connect request: {e}"),
            Ok(_) => log::info!("[{client_name}] Connected"),
        }

        let mut framed = Framed::new(self.stream, ServerCodec::new());

        loop {
            tokio::select! {
                // Read from network socket
                item = framed.next() => {
                    match item {
                        None => {
                            log::info!("[{client_name}] Connection closed by client");
                            break;
                        }
                        Some(result) => match result {
                            Err(e) => {
                                log::error!("[{client_name}] Stream error: {e}");
                                bail!(e);
                            }
                            Ok(packet) => {
                                let rtt = Utc::now().timestamp_millis() - packet.timestamp;
                                log::debug!("[{client_name}] Roundtrip time: {rtt}ms");

                                let request = Request::Message(Message::new(
                                    Destination::All,
                                    packet.message
                                ));
                                if let Err(e) = self.cmd_tx.send(self.identity.wrap_request(request)).await {
                                    log::error!("[{client_name}] Failed to forward request to server core: {e}");
                                    break;
                                }
                            }
                        }
                    }
                }

                // Read from broadcast channel
                result = self.bcast_rx.recv() => {
                    match result {
                        Ok(server_response) => {
                            let out_msg = match server_response.response_type {
                                ResponseType::Welcome(user_id) => {
                                    if user_id == self.identity.id {
                                        OutgoingMessage::ServerMessage(ServerMessage::Welcome(user_id))
                                    } else {
                                        continue;
                                    }
                                }
                                ResponseType::Disconnect(_) => {
                                    OutgoingMessage::ServerMessage(ServerMessage::Disconnect)
                                }
                                ResponseType::Message { sender: _, sender_id, content } => {
                                    OutgoingMessage::PeerMessage {
                                        author_id: sender_id,
                                        content,
                                    }
                                }
                            };

                            let out_packet = OutgoingPacket {
                                timestamp: server_response.timestamp.timestamp_millis(),
                                message: out_msg,
                            };

                            if let Err(e) = framed.send(out_packet).await {
                                log::error!("[{client_name}] Failed to send packet to client: {e}");
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            log::warn!("[{client_name}] Broadcast receiver lagged by {n} messages");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            log::info!("[{client_name}] Broadcast channel closed");
                            break;
                        }
                    }
                }
            }
        }

        // Notify core that we disconnected
        let _ = self
            .cmd_tx
            .send(self.identity.wrap_request(Request::Disconnect))
            .await;

        Ok(())
    }
}
