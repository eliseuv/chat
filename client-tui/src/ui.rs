use std::io;
use crossterm::{QueueableCommand, tty::IsTty};
use server::{protocol, remote};
use crate::history::ChatHistory;

#[derive(Debug)]
pub struct ChatInterface<O: io::Write + QueueableCommand + IsTty> {
    pub output: O,
}

impl<O: io::Write + QueueableCommand + IsTty> ChatInterface<O> {
    /// Create new chat interface
    pub fn new(output: O) -> Self {
        Self { output }
    }

    pub fn draw(&mut self, history: &ChatHistory, input_buffer: &str) -> anyhow::Result<()> {
        use crossterm::{
            cursor,
            style::Print,
            terminal::{self, ClearType},
        };

        let (cols, rows) = terminal::size()?;

        self.output.queue(terminal::Clear(ClearType::All))?;

        let history_len = history.messages.len();
        let display_count = std::cmp::min(history_len, (rows.saturating_sub(2)) as usize);
        let start_idx = history_len - display_count;

        for (i, msg) in history.messages.iter().skip(start_idx).enumerate() {
            self.output.queue(cursor::MoveTo(0, i as u16))?;

            let time_str = msg
                .datetime
                .with_timezone(&chrono::Local)
                .format("%H:%M:%S");

            let text = match &msg.message {
                remote::packet::OutgoingMessage::ServerMessage(s) => match s {
                    remote::packet::ServerMessage::Welcome(id) => {
                        format!("[{}] [SERVER]: Welcome to the chat! You are User {}", time_str, id)
                    }
                    remote::packet::ServerMessage::Disconnect => {
                        format!("[{}] [SERVER]: Disconnected.", time_str)
                    }
                },
                remote::packet::OutgoingMessage::PeerMessage { author_id, content } => {
                    let content_str = match content {
                        protocol::message::MessageContent::Text(t) => t.clone(),
                        protocol::message::MessageContent::Binary(_) => "<binary data>".to_string(),
                    };
                    format!("[{}] [User {}]: {}", time_str, author_id, content_str)
                }
            };
            self.output.queue(Print(text))?;
        }

        // Draw separator
        self.output
            .queue(cursor::MoveTo(0, rows.saturating_sub(2)))?;
        self.output.queue(Print("-".repeat(cols as usize)))?;

        // Draw input prompt
        self.output
            .queue(cursor::MoveTo(0, rows.saturating_sub(1)))?;
        let prompt = format!("> {}", input_buffer);
        self.output.queue(Print(&prompt))?;

        self.output.flush()?;
        Ok(())
    }
}
