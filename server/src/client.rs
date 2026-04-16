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

use crate::protocol;
use crate::remote::codec::ServerCodec;
use crate::remote::packet::{OutgoingMessage, OutgoingPacket, ServerMessage};

/// Client ID counter
static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Represents a client connection
#[derive(Debug)]
pub struct Client {
    /// Unique ID
    pub id: u64,
    /// Client address
    pub addr: SocketAddr,
    /// Client stream
    pub stream: TcpStream,
    /// Command sender
    pub cmd_tx: mpsc::Sender<protocol::request::ClientRequest>,
    /// Broadcast receiver
    pub bcast_rx: broadcast::Receiver<protocol::response::ServerResponse>,
}

impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client {id} [{addr}]", id = self.id, addr = self.addr)
    }
}

impl Client {
    pub fn new(
        addr: SocketAddr,
        stream: TcpStream,
        cmd_tx: mpsc::Sender<protocol::request::ClientRequest>,
        bcast_tx: &broadcast::Sender<protocol::response::ServerResponse>,
    ) -> Self {
        Self {
            id: CLIENT_ID_COUNTER.fetch_add(1, atomic::Ordering::SeqCst),
            addr,
            stream,
            cmd_tx,
            bcast_rx: bcast_tx.subscribe(),
        }
    }

    pub async fn send_request(
        &self,
        request: protocol::request::Request,
    ) -> Result<(), mpsc::error::SendError<protocol::request::ClientRequest>> {
        let request = protocol::request::ClientRequest {
            client_id: self.id,
            addr: self.addr,
            timestamp: Utc::now(),
            request,
        };

        self.cmd_tx.send(request).await
    }

    pub async fn run(mut self) -> Result<(), anyhow::Error> {
        let client_name = self.to_string();
        log::info!("[{client_name}] Started");

        match self.send_request(protocol::request::Request::Connect).await {
            Err(e) => bail!("Failed to send connect request: {e}"),
            Ok(_) => log::info!("[{client_name}] Connected"),
        }

        let mut framed = Framed::new(self.stream, ServerCodec::new());

        loop {
            tokio::select! {
                // Read from network socket
                result = framed.next() => {
                    match result {
                        Some(Ok(packet)) => {
                            let request = protocol::request::ClientRequest {
                                client_id: self.id,
                                addr: self.addr,
                                timestamp: Utc::now(),
                                request: protocol::request::Request::Message(protocol::message::Message::new(
                                    protocol::message::Destination::All,
                                    packet.message,
                                )),
                            };
                            if let Err(e) = self.cmd_tx.send(request).await {
                                log::error!("[{client_name}] Failed to forward request to server core: {e}");
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("[{client_name}] Stream error: {e}");
                            break;
                        }
                        None => {
                            log::info!("[{client_name}] Connection closed by client");
                            break;
                        }
                    }
                }

                // Read from broadcast channel
                result = self.bcast_rx.recv() => {
                    match result {
                        Ok(server_response) => {
                            let out_msg = match server_response.response {
                                protocol::response::Response::Welcome(user_id) => {
                                    if user_id == self.id {
                                        OutgoingMessage::ServerMessage(ServerMessage::Welcome(user_id))
                                    } else {
                                        continue;
                                    }
                                }
                                protocol::response::Response::Disconnect(_) => {
                                    OutgoingMessage::ServerMessage(ServerMessage::Disconnect)
                                }
                                protocol::response::Response::Message { sender: _, sender_id, content } => {
                                    OutgoingMessage::PeerMessage {
                                        author_id: sender_id,
                                        content,
                                    }
                                }
                            };

                            let out_packet = OutgoingPacket {
                                timestamp: server_response.timestamp.timestamp(),
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
        let disconnect_req = protocol::request::ClientRequest {
            client_id: self.id,
            addr: self.addr,
            timestamp: Utc::now(),
            request: protocol::request::Request::Disconnect,
        };
        let _ = self.cmd_tx.send(disconnect_req).await;

        Ok(())
    }
}
