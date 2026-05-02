use anyhow::Context;
use chrono::{DateTime, Utc};
use crossterm::terminal;
use futures::{SinkExt, StreamExt};
use std::io;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::history::{ChatHistory, ReceivedMessage};
use crate::ui::ChatInterface;
use server::{protocol, remote};

#[derive(Debug)]
pub enum State {
    Quit,
    Default,
}

/// Chat Application
pub struct ChatApp {
    state: State,
    framed_connection: Framed<TcpStream, remote::codec::ClientCodec>,
    input_buffer: String,
    history: ChatHistory,
    interface: ChatInterface<io::Stdout>,
}

impl ChatApp {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            state: State::Default,
            framed_connection: Framed::new(stream, remote::codec::ClientCodec::new()),
            input_buffer: String::new(),
            history: ChatHistory {
                messages: Vec::new(),
            },
            interface: ChatInterface::new(io::stdout()),
        }
    }

    pub async fn send_message(
        &mut self,
        message: protocol::message::MessageContent,
    ) -> anyhow::Result<()> {
        let packet = remote::packet::IncomingPacket {
            timestamp: Utc::now().timestamp_millis(),
            message,
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

    async fn handle_event(&mut self, event: crossterm::event::Event) -> anyhow::Result<()> {
        use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    self.state = State::Quit;
                    return Ok(());
                }

                match key_event.code {
                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                        self.draw()?;
                    }
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                        self.draw()?;
                    }
                    KeyCode::Enter => {
                        if !self.input_buffer.is_empty() {
                            let text = std::mem::take(&mut self.input_buffer);
                            if let Err(e) = self
                                .send_message(protocol::message::MessageContent::Text(text))
                                .await
                            {
                                log::error!("Failed to send message: {}", e);
                            }
                            self.draw()?;
                        }
                    }
                    KeyCode::Esc => {
                        self.state = State::Quit;
                    }
                    _ => {}
                }
            }
            Event::Resize(_, _) => {
                self.draw()?;
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        use crossterm::event::EventStream;

        // Initialize terminal
        terminal::enable_raw_mode()?;
        let mut reader = EventStream::new();
        self.draw()?;

        // Enter main loop
        loop {
            match self.state {
                // Close the application
                State::Quit => {
                    terminal::disable_raw_mode()?;
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
                        result = self.framed_connection.next() => {
                            match result {
                                Some(Ok(packet)) => {
                                    let rtt = Utc::now().timestamp_millis() - packet.timestamp;
                                    log::debug!("Roundtrip time: {rtt}ms");

                                    self.history.messages.push(ReceivedMessage {
                                        datetime: DateTime::<Utc>::from_timestamp_millis(packet.timestamp)
                                            .context("Unable to parse timestamp")?,
                                        message: packet.message,
                                    });
                                    self.draw()?;
                                }
                                Some(Err(e)) => {
                                    log::error!("Failed to read remote stream: {e}");
                                }
                                None => {
                                    // Connection closed
                                    self.state = State::Quit;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
