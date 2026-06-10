use crate::ui::{AppEvent, UiEventStream};
use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use server::protocol::MessageContent;
use server::remote::codec::ClientCodec;
use server::remote::packet::{ClientMessage, ClientRemotePacket, ServerCommand, ServerMessage};
use std::io;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::history::{ChatHistory, ReceivedMessage};
use crate::ui::ChatInterface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Quit,
    Login,
    ChatRoom,
}

/// Chat Application
pub struct ChatApp {
    state: State,
    framed_connection: Framed<TcpStream, ClientCodec>,
    input_buffer: String,
    history: ChatHistory,
    interface: ChatInterface<io::Stdout>,
    is_confirming_quit: bool,
}

impl ChatApp {
    pub fn new(stream: TcpStream) -> anyhow::Result<Self> {
        Ok(Self {
            state: State::Login,
            framed_connection: Framed::new(stream, ClientCodec::new()),
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

    async fn handle_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        if self.is_confirming_quit {
            match event {
                AppEvent::InputChar('y') | AppEvent::InputChar('Y') => {
                    self.state = State::Quit;
                }
                AppEvent::InputChar('n') | AppEvent::InputChar('N') | AppEvent::Cancel => {
                    self.is_confirming_quit = false;
                    self.draw()?;
                }
                AppEvent::Resize => {
                    self.draw()?;
                }
                _ => {}
            }
            return Ok(());
        }

        match event {
            AppEvent::None => {}

            AppEvent::Quit => {
                self.is_confirming_quit = true;
                self.draw()?;
            }

            AppEvent::Cancel => {}

            AppEvent::InputChar(c) => {
                self.input_buffer.push(c);
                self.draw()?;
            }

            AppEvent::Backspace => {
                self.input_buffer.pop();
                self.draw()?;
            }

            AppEvent::Enter => {
                if !self.input_buffer.is_empty() {
                    let text = std::mem::take(&mut self.input_buffer);
                    match self.state {
                        State::Login => {
                            self.history.own_username = Some(text.clone());
                            if let Err(e) = self.send_login(text).await {
                                log::error!("Failed to send login: {}", e);
                            }
                        }
                        State::ChatRoom => {
                            if let Err(e) = self.send_message(MessageContent::Text(text)).await {
                                log::error!("Failed to send message: {}", e);
                            }
                        }
                        _ => {}
                    }
                    self.draw()?;
                }
            }

            AppEvent::Resize => {
                self.draw()?;
            }
        }
        Ok(())
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
                                        let rtt = Utc::now().timestamp_millis() - packet.timestamp;
                                        log::debug!("Roundtrip time: {rtt}ms");

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
                                                _ => {}
                                            }
                                        }

                                        self.history.messages.push(ReceivedMessage {
                                            datetime: DateTime::<Utc>::from_timestamp_millis(packet.timestamp)
                                                .context("Unable to parse timestamp")?,
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
