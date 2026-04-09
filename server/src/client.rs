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

use crate::protocol::{
    ClientRequest, Destination, Message, MessageContent, Request, ServerResponse,
};

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
    pub cmd_tx: mpsc::Sender<ClientRequest>,
    /// Broadcast receiver
    pub bcast_rx: broadcast::Receiver<ServerResponse>,
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
        cmd_tx: mpsc::Sender<ClientRequest>,
        bcast_tx: &broadcast::Sender<ServerResponse>,
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
        request: Request,
    ) -> Result<(), mpsc::error::SendError<ClientRequest>> {
        let request = ClientRequest {
            addr: self.addr,
            timestamp: Utc::now(),
            request,
        };

        self.cmd_tx.send(request).await
    }

    pub async fn run(mut self) -> Result<(), anyhow::Error> {
        log::info!("[{self}] Started");

        // let (mut reader, mut writer) = self.stream.split();

        match self.send_request(Request::Connect).await {
            Err(e) => bail!("Failed to send connect request: {e}"),
            Ok(_) => log::info!("[{self}] Connected"),
        }

        // Mock sending a message from this client to the server core
        let request = Request::Message(Message::new(
            Destination::All,
            MessageContent::Text(format!("Hello from {self}!")),
        ));
        if let Err(e) = self.send_request(request).await {
            bail!("[{self}] unable to send request: {e}");
        }

        // Loop listening to server
        loop {
            match self.bcast_rx.recv().await {
                Err(e) => bail!("Failed to receive broadcast: {e}"),
                Ok(response) => log::info!("Received broadcast: '{response:?}'"),
            }
        }

        // A complete implementation would loop indefinitely here utilizing `tokio::select!`
        // to concurrently read from `self.socket` (sending that data to `self.cmd_tx`),
        // while also reading from `self.bcast_rx` (and writing that data to `self.socket`).
    }
}
