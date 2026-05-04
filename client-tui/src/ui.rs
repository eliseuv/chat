use crate::history::ChatHistory;
use anyhow::Context;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{QueueableCommand, tty::IsTty};
use futures::StreamExt;
use server::protocol::{ChatMessage, MessageContent};
use server::remote::packet::{ServerCommand, ServerMessage};
use std::io;

/// Represents a high-level application event abstracted from raw terminal input.
///
/// This enum simplifies raw key strokes and terminal events into semantically
/// meaningful actions that the application loop can easily process.
pub enum AppEvent {
    /// An ignored or unhandled terminal event.
    None,
    /// A signal to quit the application (e.g., `Esc` or `Ctrl-C`).
    Quit,
    /// A standard character input typed by the user.
    InputChar(char),
    /// A backspace keystroke to delete the last character.
    Backspace,
    /// An enter/return keystroke to submit a message.
    Enter,
    /// A terminal resize event requiring a UI redraw.
    Resize,
}

/// A stream wrapper that reads raw terminal events and translates them into `AppEvent`s.
pub struct UiEventStream {
    reader: EventStream,
}

impl UiEventStream {
    /// Initializes a new `UiEventStream` connected to the standard terminal event stream.
    pub fn new() -> Self {
        Self {
            reader: EventStream::new(),
        }
    }

    /// Asynchronously waits for and returns the next parsed `AppEvent`.
    ///
    /// Intercepts special control sequences like `Ctrl-C` to trigger a `Quit` event.
    pub async fn next(&mut self) -> Option<anyhow::Result<AppEvent>> {
        let event = self.reader.next().await?;
        match event {
            Ok(Event::Key(key_event)) if key_event.kind == KeyEventKind::Press => {
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    return Some(Ok(AppEvent::Quit));
                }

                match key_event.code {
                    KeyCode::Char(c) => Some(Ok(AppEvent::InputChar(c))),
                    KeyCode::Backspace => Some(Ok(AppEvent::Backspace)),
                    KeyCode::Enter => Some(Ok(AppEvent::Enter)),
                    KeyCode::Esc => Some(Ok(AppEvent::Quit)),
                    _ => Some(Ok(AppEvent::None)),
                }
            }
            Ok(Event::Resize(_, _)) => Some(Ok(AppEvent::Resize)),
            Ok(_) => Some(Ok(AppEvent::None)),
            Err(e) => Some(Err(e.into())),
        }
    }
}

impl Default for UiEventStream {
    fn default() -> Self {
        Self::new()
    }
}

/// The main interface for managing terminal output and rendering the chat UI.
///
/// Handles drawing the chat history, user input buffer, and maintains the terminal
/// state using RAII (Resource Acquisition Is Initialization).
#[derive(Debug)]
pub struct ChatInterface<O: io::Write + QueueableCommand + IsTty> {
    /// The underlying output stream (usually `io::stdout()`) used to write rendered characters.
    pub output: O,
}

impl<O: io::Write + QueueableCommand + IsTty> ChatInterface<O> {
    /// Creates a new `ChatInterface` and initializes the terminal into raw mode.
    ///
    /// Raw mode disables canonical input processing and echoing, giving the application
    /// full control over the input stream and output rendering.
    pub fn new(output: O) -> anyhow::Result<Self> {
        crossterm::terminal::enable_raw_mode().context("Unable to enable raw terminal mode")?;
        Ok(Self { output })
    }

    /// Clears the terminal and re-draws the entire chat interface.
    ///
    /// # Arguments
    /// * `history` - The collection of received chat messages.
    /// * `input_buffer` - The current text the user is typing.
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
                ServerMessage::Command(s) => match s {
                    ServerCommand::Welcome(id) => {
                        format!(
                            "[{}] [SERVER]: Welcome to the chat! You are User {}",
                            time_str, id
                        )
                    }
                    ServerCommand::Disconnect => {
                        format!("[{}] [SERVER]: Disconnected.", time_str)
                    }
                },
                ServerMessage::Chat(ChatMessage {
                    author_id,
                    destination: _,
                    content,
                }) => {
                    let content_str = match content {
                        MessageContent::Text(t) => t.clone(),
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

/// Ensures that the terminal is properly restored to its original state when the application exits.
///
/// This RAII implementation guarantees that even during a panic, `disable_raw_mode()` will be
/// called to prevent leaving the user's terminal in an unusable state.
impl<O: io::Write + QueueableCommand + IsTty> Drop for ChatInterface<O> {
    fn drop(&mut self) {
        if let Err(e) = crossterm::terminal::disable_raw_mode() {
            log::error!("Unable to disable raw mode: {}", e);
        }
    }
}
