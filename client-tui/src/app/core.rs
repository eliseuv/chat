use crate::ui::UiEventStream;
use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use server::protocol::MessageContent;
use server::remote::codec::ClientCodec;
use server::remote::packet::{ClientMessage, ClientRemotePacket, ServerCommand, ServerMessage};
use std::io;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::app::history::{ChatHistory, ReceivedMessage};
use crate::app::state::State;
use crate::ui::ChatInterface;

/// Chat Application
pub struct ChatApp {
    pub(crate) state: State,
    pub(crate) framed_connection: Framed<TcpStream, ClientCodec>,
    pub(crate) input_buffer: String,
    pub(crate) history: ChatHistory,
    pub(crate) interface: ChatInterface<io::Stdout>,
    pub(crate) is_confirming_quit: bool,
    pub(crate) autocomplete_state: Option<(String, usize, usize)>,
}

impl ChatApp {
    pub fn new(stream: TcpStream) -> anyhow::Result<Self> {
        Ok(Self {
            state: State::Login,
            framed_connection: Framed::new(stream, ClientCodec::default()),
            input_buffer: String::new(),
            history: ChatHistory {
                messages: Vec::new(),
                own_id: None,
                own_username: None,
                login_error: None,
                active_usernames: Vec::new(),
            },
            interface: ChatInterface::new(io::stdout())?,
            is_confirming_quit: false,
            autocomplete_state: None,
        })
    }

    pub async fn send_message(&mut self, message: MessageContent) -> anyhow::Result<()> {
        let packet = ClientRemotePacket {
            timestamp: Utc::now().timestamp_millis(),
            message: ClientMessage::Chat(message),
        };
        self.framed_connection
            .send(packet)
            .await
            .context("Unable to send message")?;
        Ok(())
    }

    pub async fn send_login(&mut self, username: String) -> anyhow::Result<()> {
        let packet = ClientRemotePacket {
            timestamp: Utc::now().timestamp_millis(),
            message: ClientMessage::Login(username),
        };
        self.framed_connection
            .send(packet)
            .await
            .context("Unable to send login request")?;
        Ok(())
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.interface.draw(self.state, &self.history, &self.input_buffer, self.is_confirming_quit)
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Initialize terminal
        let mut reader = UiEventStream::new();
        self.draw()?;

        // Enter main loop
        loop {
            match self.state {
                // Close the application
                State::Quit => {
                    return Ok(());
                }

                // Keep the application running
                State::Login | State::ChatRoom => {
                    tokio::select! {
                        maybe_event = reader.next() => {
                            if let Some(Ok(e)) = maybe_event {
                                self.handle_event(e).await?;
                            }
                        }

                        item = self.framed_connection.next() => {
                            match item {
                                None => {
                                    // Connection closed
                                    self.state = State::Quit;
                                }

                                Some(result) => match result {
                                    Err(e) => {
                                        log::error!("Failed to read remote stream: {e}");
                                    },

                                    Ok(packet) => {
                                        if let ServerMessage::Command(cmd) = &packet.message {
                                            match cmd {
                                                ServerCommand::Welcome(id) => {
                                                    self.history.own_id = Some(*id);
                                                    self.history.login_error = None;
                                                    self.state = State::ChatRoom;
                                                }
                                                ServerCommand::LoginError(reason) => {
                                                    self.history.login_error = Some(reason.clone());
                                                    self.history.own_username = None;
                                                }
                                                ServerCommand::ActiveUsers { usernames } => {
                                                    self.history.active_usernames = usernames.clone();
                                                }
                                                ServerCommand::Joined(_) => {}
                                                ServerCommand::Left(_) => {}
                                                ServerCommand::Ping(timestamp) => {
                                                    let response = ClientRemotePacket {
                                                        timestamp: Utc::now().timestamp_millis(),
                                                        message: ClientMessage::Pong(*timestamp),
                                                    };
                                                    if let Err(e) = self.framed_connection.send(response).await {
                                                        log::error!("Failed to send pong: {e}");
                                                    }
                                                    continue;
                                                }
                                                ServerCommand::Disconnect => {}
                                            }
                                        }

                                        self.history.messages.push(ReceivedMessage {
                                            datetime: DateTime::<Utc>::from_timestamp_millis(packet.timestamp)
                                                .context("Unable to parse timestamp")?.with_timezone(&chrono::Local),
                                            message: packet.message,
                                        });
                                        self.draw()?;
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
