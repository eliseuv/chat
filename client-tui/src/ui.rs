use crate::history::ChatHistory;
use anyhow::Context;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{execute, tty::IsTty};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use server::protocol::{ChatMessage, MessageContent};
use server::remote::packet::{ServerCommand, ServerMessage};
use std::io;
use tokio::sync::mpsc;

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
            let width = f.area().width.saturating_sub(2) as usize;
            let prompt_text = format!("> {}", input_buffer);
            let (wrapped_prompt, input_lines) = wrap_text(&prompt_text, width);
            let input_height = (input_lines as u16).clamp(1, 5) + 2; // Clamp text to 1-5 lines, add 2 for borders

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Min(1), Constraint::Length(input_height)].as_ref())
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
                        author_id, content, ..
                    }) => {
                        let content_str = match content {
                            MessageContent::Text(t) => t.clone(),
                        };
                        let sender = if Some(*author_id) == history.own_id {
                            "You".to_string()
                        } else {
                            format!("User {}", author_id)
                        };
                        format!("[{}] [{}]: {}", time_str, sender, content_str)
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

            let input_paragraph = Paragraph::new(wrapped_prompt)
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

/// Simulates word wrapping on the input buffer and returns the wrapped string and line count.
fn wrap_text(text: &str, max_width: usize) -> (String, usize) {
    if max_width == 0 {
        return (text.to_string(), 1);
    }
    
    let mut words = Vec::new();
    let mut current_word = String::new();
    for c in text.chars() {
        if c == ' ' {
            if !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
            words.push(" ".to_string());
        } else {
            current_word.push(c);
        }
    }
    if !current_word.is_empty() {
        words.push(current_word);
    }
    
    let mut wrapped = String::new();
    let mut current_line_len = 0;
    let mut line_count = 0;
    
    for word in words {
        let word_width = word.chars().count();
        
        if current_line_len + word_width <= max_width {
            wrapped.push_str(&word);
            current_line_len += word_width;
        } else {
            if word_width > max_width {
                // Word is wider than max_width. We must split it.
                let remaining = max_width.saturating_sub(current_line_len);
                let mut chars = word.chars();
                
                // Fill the rest of the current line first
                if remaining > 0 {
                    let first_part: String = chars.by_ref().take(remaining).collect();
                    wrapped.push_str(&first_part);
                }
                
                // Start a new line for the rest
                wrapped.push('\n');
                line_count += 1;
                current_line_len = 0;
                
                // Keep chunking the rest of the word
                let mut chunk = String::new();
                for c in chars {
                    chunk.push(c);
                    if chunk.chars().count() == max_width {
                        wrapped.push_str(&chunk);
                        wrapped.push('\n');
                        line_count += 1;
                        chunk.clear();
                    }
                }
                if !chunk.is_empty() {
                    wrapped.push_str(&chunk);
                    current_line_len = chunk.chars().count();
                }
            } else {
                // Normal wrap at word boundary
                if !wrapped.is_empty() && !wrapped.ends_with('\n') {
                    wrapped.push('\n');
                    line_count += 1;
                }
                wrapped.push_str(&word);
                current_line_len = word_width;
            }
        }
    }
    
    if current_line_len > 0 || line_count == 0 {
        line_count += 1;
    }
    
    (wrapped, line_count)
}
