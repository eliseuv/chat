//! Server
//! Core of the chat server

use tokio::sync::{broadcast, mpsc};

use crate::protocol::{ChatMessage, ClientRequest, LoginError, Request, Response};
use crate::db::Database;

/// Server Core
#[derive(Debug)]
pub struct Server {
    /// Request receiver
    req_rx: mpsc::Receiver<ClientRequest>,
    /// Broadcast sender
    bcast_tx: broadcast::Sender<Response>,
    /// Map of active client_id -> username
    active_users: std::collections::HashMap<u64, String>,
    /// Database connection
    db: Database,
}

impl Server {
    pub fn new(channel_capacity: usize, db: Database) -> (
        Self,
        mpsc::Sender<ClientRequest>,
        broadcast::Sender<Response>,
    ) {
        // MPSC Channel: Clients -> Server
        let (cmd_tx, cmd_rx) = mpsc::channel(channel_capacity);

        // Broadcast Channel: Server -> Clients
        let (bcast_tx, _) = broadcast::channel(channel_capacity);

        (
            Self {
                req_rx: cmd_rx,
                bcast_tx: bcast_tx.clone(),
                active_users: std::collections::HashMap::new(),
                db,
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
                    client_addr,
                    request,
                }) = self.req_rx.recv() => {
                    match request {

                        Request::Connect { username } => {
                            let taken = self.active_users.values().any(|u| u.eq_ignore_ascii_case(&username));
                            if username.trim().is_empty() {
                                let response = Response::LoginReject {
                                    client_id,
                                    error: LoginError::EmptyUsername,
                                };
                                let _ = self.bcast_tx.send(response);
                            } else if taken {
                                let response = Response::LoginReject {
                                    client_id,
                                    error: LoginError::UsernameTaken,
                                };
                                let _ = self.bcast_tx.send(response);
                            } else {
                                self.active_users.insert(client_id, username.clone());
                                
                                if let Err(e) = self.db.update_login(&username, &client_addr.to_string()) {
                                    log::error!("Database error updating login for {}: {}", username, e);
                                }

                                let response = Response::Welcome(client_id);
                                let _ = self.bcast_tx.send(response);

                                // Broadcast that the user joined!
                                let joined_response = Response::Joined {
                                    username: username.clone(),
                                };
                                let _ = self.bcast_tx.send(joined_response);

                                // Broadcast the updated active users list!
                                let active_usernames: Vec<String> = self.active_users.values().cloned().collect();
                                let active_response = Response::ActiveUsers {
                                    usernames: active_usernames,
                                };
                                let _ = self.bcast_tx.send(active_response);
                            }
                        }

                        Request::GetActiveUsers => {
                            let active_usernames: Vec<String> = self.active_users.values().cloned().collect();
                            let response = Response::ActiveUsers {
                                usernames: active_usernames,
                            };
                            let _ = self.bcast_tx.send(response);
                        }

                        Request::Disconnect => {
                            if let Some(username) = self.active_users.remove(&client_id) {
                                if let Err(e) = self.db.update_logout(&username) {
                                    log::error!("Database error updating logout for {}: {}", username, e);
                                }

                                let left_response = Response::Left {
                                    username: username.clone(),
                                };
                                let _ = self.bcast_tx.send(left_response);
                            }

                            // Broadcast the updated active users list!
                            let active_usernames: Vec<String> = self.active_users.values().cloned().collect();
                            let active_response = Response::ActiveUsers {
                                usernames: active_usernames,
                            };
                            let _ = self.bcast_tx.send(active_response);
                        }

                        Request::Message(ChatMessage { author_id, author_username, destination, content }) => {
                            let response = Response::Message(ChatMessage{
                                author_id,
                                author_username,
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
