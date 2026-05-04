use crate::ui::{AppEvent, UiEventStream};
use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use server::protocol::MessageContent;
use server::remote::codec::ClientCodec;
use server::remote::packet::ClientRemotePacket;
use std::io;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::history::{ChatHistory, ReceivedMessage};
use crate::ui::ChatInterface;

#[derive(Debug)]
pub enum State {
    Quit,
    Default,
}

/// Chat Application
pub struct ChatApp {
    state: State,
    framed_connection: Framed<TcpStream, ClientCodec>,
    input_buffer: String,
    history: ChatHistory,
    interface: ChatInterface<io::Stdout>,
}

impl ChatApp {
    pub fn new(stream: TcpStream) -> anyhow::Result<Self> {
        Ok(Self {
            state: State::Default,
            framed_connection: Framed::new(stream, ClientCodec::new()),
            input_buffer: String::new(),
            history: ChatHistory {
                messages: Vec::new(),
            },
            interface: ChatInterface::new(io::stdout())?,
        })
    }

    pub async fn send_message(&mut self, message: MessageContent) -> anyhow::Result<()> {
        let packet = ClientRemotePacket {
            timestamp: Utc::now().timestamp_millis(),
            message_content: message,
        };
        self.framed_connection
            .send(packet)
            .await
            .context("Unable to send message")?;
        Ok(())
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        self.interface.draw(&self.history, &self.input_buffer)
    }

    async fn handle_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::None => {}

            AppEvent::Quit => {
                self.state = State::Quit;
            }

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
                    if let Err(e) = self.send_message(MessageContent::Text(text)).await {
                        log::error!("Failed to send message: {}", e);
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
                State::Default => {
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
