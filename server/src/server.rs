//! Server
//! Core of the chat server

use tokio::sync::{broadcast, mpsc};

use crate::protocol;

/// Channel size
pub const CHANNEL_CAPACITY: usize = 32;

/// Server Core
#[derive(Debug)]
pub struct Server {
    /// Request receiver
    req_rx: mpsc::Receiver<protocol::request::ClientRequest>,
    /// Broadcast sender
    bcast_tx: broadcast::Sender<protocol::response::ServerResponse>,
}

impl Server {
    pub fn new() -> (
        Self,
        mpsc::Sender<protocol::request::ClientRequest>,
        broadcast::Sender<protocol::response::ServerResponse>,
    ) {
        // MPSC Channel: Clients -> Server
        let (cmd_tx, cmd_rx) = mpsc::channel(CHANNEL_CAPACITY);

        // Broadcast Channel: Server -> Clients
        let (bcast_tx, _) = broadcast::channel(CHANNEL_CAPACITY);

        (
            Self {
                req_rx: cmd_rx,
                bcast_tx: bcast_tx.clone(),
            },
            cmd_tx,
            bcast_tx,
        )
    }

    pub async fn run(mut self) {
        log::info!("[Server Core] Task started");

        // Listen for incoming commands from all workers indefinitely
        loop {
            tokio::select! {
                Some(protocol::request::ClientRequest {
                    client_id,
                    addr,
                    timestamp,
                    request,
                }) = self.req_rx.recv() => {
                    match request {
                        protocol::request::Request::Connect => {
                            let response = protocol::response::Response::Welcome(client_id);
                            let server_response = protocol::response::ServerResponse {
                                timestamp,
                                response,
                            };
                            let _ = self.bcast_tx.send(server_response);
                        }
                        protocol::request::Request::Disconnect => {}
                        protocol::request::Request::Message(msg) => {
                            let response = protocol::response::Response::Message {
                                sender: addr,
                                sender_id: client_id,
                                content: msg.content,
                            };
                            let server_response = protocol::response::ServerResponse {
                                timestamp,
                                response,
                            };
                            let _ = self.bcast_tx.send(server_response);
                        }
                    }
                }
                else => break,
            }
        }
    }
}
