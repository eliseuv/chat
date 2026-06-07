use crate::history::ChatHistory;
use anyhow::Context;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::crossterm::{execute, tty::IsTty};
use tokio::sync::mpsc;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
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
    rx: mpsc::UnboundedReceiver<anyhow::Result<AppEvent>>,
}

impl UiEventStream {
    /// Initializes a new `UiEventStream` connected to the standard terminal event stream.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        std::thread::spawn(move || {
            loop {
                match event::read() {
                    Ok(Event::Key(key_event)) if key_event.kind == KeyEventKind::Press => {
                        let app_event = if key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && key_event.code == KeyCode::Char('c')
                        {
                            AppEvent::Quit
                        } else {
                            match key_event.code {
                                KeyCode::Char(c) => AppEvent::InputChar(c),
                                KeyCode::Backspace => AppEvent::Backspace,
                                KeyCode::Enter => AppEvent::Enter,
                                KeyCode::Esc => AppEvent::Quit,
                                _ => AppEvent::None,
                            }
                        };
                        if tx.send(Ok(app_event)).is_err() {
                            break;
                        }
                    }
                    Ok(Event::Resize(_, _)) => {
                        if tx.send(Ok(AppEvent::Resize)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {
                        if tx.send(Ok(AppEvent::None)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.into()));
                        break;
                    }
                }
            }
        });

        Self { rx }
    }

    /// Asynchronously waits for and returns the next parsed `AppEvent`.
    ///
    /// Intercepts special control sequences like `Ctrl-C` to trigger a `Quit` event.
    pub async fn next(&mut self) -> Option<anyhow::Result<AppEvent>> {
        self.rx.recv().await
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
pub struct ChatInterface<O: io::Write + IsTty> {
    terminal: Terminal<CrosstermBackend<O>>,
}

impl<O: io::Write + ratatui::crossterm::QueueableCommand + IsTty> ChatInterface<O> {
    /// Creates a new `ChatInterface` and initializes the terminal into raw mode.
    ///
    /// Raw mode disables canonical input processing and echoing, giving the application
    /// full control over the input stream and output rendering.
    pub fn new(mut output: O) -> anyhow::Result<Self> {
        enable_raw_mode().context("Unable to enable raw terminal mode")?;
        execute!(output, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(output);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Clears the terminal and re-draws the entire chat interface.
    ///
    /// # Arguments
    /// * `history` - The collection of received chat messages.
    /// * `input_buffer` - The current text the user is typing.
    pub fn draw(&mut self, history: &ChatHistory, input_buffer: &str) -> anyhow::Result<()> {
        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints(
                    [
                        Constraint::Min(1),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let mut list_items = Vec::new();
            for msg in &history.messages {
                let time_str = msg
                    .datetime
                    .with_timezone(&chrono::Local)
                    .format("%H:%M:%S");

                let text = match &msg.message {
                    ServerMessage::Command(s) => match s {
                        ServerCommand::Welcome(id) => {
                            format!("[{}] [SERVER]: Welcome to the chat! You are User {}", time_str, id)
                        }
                        ServerCommand::Disconnect => {
                            format!("[{}] [SERVER]: Disconnected.", time_str)
                        }
                    },
                    ServerMessage::Chat(ChatMessage {
                        author_id,
                        content,
                        ..
                    }) => {
                        let content_str = match content {
                            MessageContent::Text(t) => t.clone(),
                        };
                        format!("[{}] [User {}]: {}", time_str, author_id, content_str)
                    }
                };
                list_items.push(ListItem::new(Line::from(text)));
            }

            let mut state = ListState::default();
            if !list_items.is_empty() {
                state.select(Some(list_items.len() - 1));
            }

            let history_list = List::new(list_items)
                .block(Block::default().title("Chat History").borders(Borders::ALL));
            
            f.render_stateful_widget(history_list, chunks[0], &mut state);

            let input_paragraph = Paragraph::new(format!("> {}", input_buffer))
                .block(Block::default().title("Input").borders(Borders::ALL));
            f.render_widget(input_paragraph, chunks[1]);
        })?;
        Ok(())
    }
}

/// Ensures that the terminal is properly restored to its original state when the application exits.
///
/// This RAII implementation guarantees that even during a panic, `disable_raw_mode()` will be
/// called to prevent leaving the user's terminal in an unusable state.
impl<O: io::Write + ratatui::crossterm::QueueableCommand + IsTty> Drop for ChatInterface<O> {
    fn drop(&mut self) {
        if let Err(e) = disable_raw_mode() {
            log::error!("Unable to disable raw mode: {}", e);
        }
        if let Err(e) = execute!(self.terminal.backend_mut(), LeaveAlternateScreen) {
            log::error!("Unable to leave alternate screen: {}", e);
        }
    }
}
