//! Server
//! Core of the chat server

use tokio::sync::{broadcast, mpsc};

use crate::protocol::{ChatMessage, ClientRequest, Request, Response};

/// Channel size
pub const CHANNEL_CAPACITY: usize = 32;

/// Server Core
#[derive(Debug)]
pub struct Server {
    /// Request receiver
    req_rx: mpsc::Receiver<ClientRequest>,
    /// Broadcast sender
    bcast_tx: broadcast::Sender<Response>,
}

impl Server {
    pub fn new() -> (
        Self,
        mpsc::Sender<ClientRequest>,
        broadcast::Sender<Response>,
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
                Some(ClientRequest {
                    client_id,
                    request,
                }) = self.req_rx.recv() => {
                    match request {

                        Request::Connect => {
                            // For now we accept all connection requests
                            let response = Response::Welcome(client_id);
                            let _ = self.bcast_tx.send(response);
                        }

                        Request::Disconnect => {
                            // TODO: Gracefully disconnect the client
                        }

                        Request::Message(ChatMessage { author_id, destination, content }) => {
                            let response = Response::Message(ChatMessage{
                                author_id,
                                destination,
                                content,
                            });
                            let _ = self.bcast_tx.send(response);
                        }
                    }
                }
                else => break,
            }
        }
    }
}
