//! Server
//! Core of the chat server

use tokio::sync::{broadcast, mpsc};

use crate::protocol::{ClientRequest, Request, Response, ServerResponse};

/// Channel size
pub const CHANNEL_CAPACITY: usize = 32;

/// Server Core
#[derive(Debug)]
pub struct Server {
    /// Request receiver
    req_rx: mpsc::Receiver<ClientRequest>,
    /// Broadcast sender
    bcast_tx: broadcast::Sender<ServerResponse>,
}

impl Server {
    pub fn new() -> (
        Self,
        mpsc::Sender<ClientRequest>,
        broadcast::Sender<ServerResponse>,
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
        while let Some(ClientRequest {
            addr,
            timestamp,
            request,
        }) = self.req_rx.recv().await
        {
            match request {
                Request::Connect => {}
                Request::Disconnect => {}
                Request::Message(msg) => {
                    let response = Response::Message {
                        sender: addr,
                        content: msg.content,
                    };
                    let server_response = ServerResponse {
                        timestamp,
                        response,
                    };
                    let _ = self.bcast_tx.send(server_response);
                }
            }
        }
    }
}
