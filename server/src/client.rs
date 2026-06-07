//! Client thread
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

use crate::protocol::{ChatMessage, ClientRequest, MessageDestination, Request, Response};
use crate::remote::codec::ServerCodec;
use crate::remote::packet::{ClientMessage, ServerCommand, ServerMessage, ServerRemotePacket};

/// Client ID counter
static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Represents the core identity of a client (ID and address).
#[derive(Debug, Clone, Copy)]
pub struct ClientIdentity {
    /// Unique ID
    pub id: u64,
    /// Client address
    pub addr: SocketAddr,
}

impl ClientIdentity {
    pub fn wrap_request(self, request: Request) -> ClientRequest {
        ClientRequest {
            client_id: self.id,
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
    /// Client username chosen on login
    pub username: Option<String>,
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
            username: None,
        }
    }

    pub async fn run(mut self) -> Result<(), anyhow::Error> {
        let client_name = self.to_string();
        log::info!("[{client_name}] Started");

        if let Err(e) = self.cmd_tx.send(self.identity.wrap_request(Request::GetActiveUsers)).await {
            log::error!("[{client_name}] Failed to request initial active users list: {e}");
        }

        let mut framed = Framed::new(self.stream, ServerCodec::new());

        loop {
            tokio::select! {
                // Read from network socket
                item = framed.next() => {
                    match item {
                        // Connection closed by client
                        None => {
                            log::info!("[{client_name}] Connection closed by client");
                            break;
                        }

                        // Received a message from client
                        Some(result) => match result {
                            // Stream error
                            Err(e) => {
                                log::error!("[{client_name}] Stream error: {e}");
                                bail!(e);
                            }

                            // Valid message received
                            Ok(packet) => {
                                let rtt = Utc::now().timestamp_millis() - packet.timestamp;
                                log::debug!("[{client_name}] Roundtrip time: {rtt}ms");

                                match packet.message {
                                    ClientMessage::Login(username) => {
                                        if self.username.is_some() {
                                            log::warn!("[{client_name}] Already logged in; ignoring login packet.");
                                            continue;
                                        }
                                        self.username = Some(username.clone());
                                        match self
                                            .cmd_tx
                                            .send(self.identity.wrap_request(Request::Connect { username }))
                                            .await
                                        {
                                            Err(e) => {
                                                log::error!("[{client_name}] Failed to send connect request: {e}");
                                                break;
                                            }
                                            Ok(_) => log::info!("[{client_name}] Sent login request to server core"),
                                        }
                                    }
                                    ClientMessage::Chat(message_content) => {
                                        if let Some(ref username) = self.username {
                                            let request = Request::Message(
                                                ChatMessage {
                                                    author_id: self.identity.id,
                                                    author_username: username.clone(),
                                                    destination: MessageDestination::AllUsers,
                                                    content: message_content,
                                                }
                                            );
                                            if let Err(e) = self.cmd_tx.send(self.identity.wrap_request(request)).await {
                                                log::error!("[{client_name}] Failed to forward request to server core: {e}");
                                                break;
                                            }
                                        } else {
                                            log::warn!("[{client_name}] Received Chat message before login; ignoring.");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Read from broadcast channel
                result = self.bcast_rx.recv() => {
                    match result {
                        // Broadcast channel closed
                        Err(broadcast::error::RecvError::Closed) => {
                            return Err(broadcast::error::RecvError::Closed.into())
                        }

                        // Broadcast channel lagged
                        Err(broadcast::error::RecvError::Lagged(n)) => log::warn!("[{client_name}] Broadcast receiver lagged by {n} messages"),

                        // Valid response
                        Ok(response) => {

                            match response {
                                Response::Welcome(client_id) => {
                                     if client_id == self.identity.id {
                                         let remote_msg = ServerMessage::Command(ServerCommand::Welcome(client_id));
                                         let packet = ServerRemotePacket::new(remote_msg);
                                         if let Err(e) = framed.send(packet).await {
                                             log::error!("[{client_name}] Failed to send welcome message to client: {e}");
                                             break;
                                         }
                                     }
                                     else {
                                         continue;
                                     }
                                 }

                                Response::LoginReject { client_id, error } => {
                                    if client_id == self.identity.id {
                                        self.username = None;
                                        let remote_msg = ServerMessage::Command(ServerCommand::LoginError(error));
                                        let packet = ServerRemotePacket::new(remote_msg);
                                        if let Err(e) = framed.send(packet).await {
                                            log::error!("[{client_name}] Failed to send login reject to client: {e}");
                                            break;
                                        }
                                    } else {
                                        continue;
                                    }
                                }

                                Response::ActiveUsers { usernames } => {
                                    let remote_msg = ServerMessage::Command(ServerCommand::ActiveUsers { usernames });
                                    let packet = ServerRemotePacket::new(remote_msg);
                                    if let Err(e) = framed.send(packet).await {
                                        log::error!("[{client_name}] Failed to send active users to client: {e}");
                                        break;
                                    }
                                }

                                Response::Disconnect(client_addr) => {
                                    if client_addr == self.identity.addr {
                                        let remote_msg = ServerMessage::Command(ServerCommand::Disconnect);
                                        let packet = ServerRemotePacket::new(remote_msg);
                                        if let Err(e) = framed.send(packet).await {
                                            log::error!("[{client_name}] Failed to send disconnect message to client: {e}");
                                            break;
                                        }
                                    } else {
                                        continue;
                                    }
                                }

                                Response::Message(chat_message) => {
                                    match chat_message.destination {
                                        MessageDestination::AllUsers => {
                                            if self.username.is_some() {
                                                let remote_msg = ServerMessage::Chat(chat_message);
                                                let packet = ServerRemotePacket::new(remote_msg);
                                                if let Err(e) = framed.send(packet).await {
                                                    log::error!("[{client_name}] Failed to send message to client: {e}");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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
