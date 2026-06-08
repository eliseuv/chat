use crate::history::ChatHistory;
use crate::app::State;
use anyhow::Context;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{execute, tty::IsTty};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use server::protocol::{ChatMessage, MessageContent};
use server::remote::packet::{ServerCommand, ServerMessage};
use std::io;
use tokio::sync::mpsc;

// Catppuccin Mocha color palette constants
#[allow(dead_code)]
const MOCHA_BASE: Color = Color::Rgb(30, 30, 46);
const MOCHA_TEXT: Color = Color::Rgb(205, 214, 244);
const MOCHA_SUBTEXT0: Color = Color::Rgb(166, 173, 200);
const MOCHA_OVERLAY0: Color = Color::Rgb(108, 112, 134);
#[allow(dead_code)]
const MOCHA_SURFACE0: Color = Color::Rgb(49, 50, 68);
const MOCHA_SURFACE1: Color = Color::Rgb(69, 71, 90);

const MOCHA_MAUVE: Color = Color::Rgb(203, 166, 247);
const MOCHA_RED: Color = Color::Rgb(243, 139, 168);
const MOCHA_PEACH: Color = Color::Rgb(250, 179, 135);
const MOCHA_YELLOW: Color = Color::Rgb(249, 226, 175);
const MOCHA_GREEN: Color = Color::Rgb(166, 227, 161);
const MOCHA_TEAL: Color = Color::Rgb(148, 226, 213);
#[allow(dead_code)]
const MOCHA_SAPPHIRE: Color = Color::Rgb(116, 199, 236);
#[allow(dead_code)]
const MOCHA_BLUE: Color = Color::Rgb(137, 180, 250);
const MOCHA_LAVENDER: Color = Color::Rgb(180, 190, 254);

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
    /// * `state` - The current application state.
    /// * `history` - The collection of received chat messages.
    /// * `input_buffer` - The current text the user is typing.
    pub fn draw(&mut self, state: State, history: &ChatHistory, input_buffer: &str) -> anyhow::Result<()> {
        self.terminal.draw(|f| {
            let width = f.area().width.saturating_sub(2) as usize;
            match state {
                State::Login => {
                    let main_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints([
                            Constraint::Length(9), // Top Title / Banner (ASCII art + top/bottom padding)
                            Constraint::Min(1),    // Main Layout (Input form + Active users list)
                            Constraint::Length(1), // Footer info
                        ].as_ref())
                        .split(f.area());

                    // Top Banner (ASCII Art with padding)
                    let ascii_lines = vec![
                        Line::default(), // Top padding
                        Line::from(Span::styled(" ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓██████▓▒░▒▓████████▓▒░       ░▒▓██████▓▒░ ░▒▓██████▓▒░░▒▓████████▓▒░▒▓████████▓▒░ ", Style::default().fg(MOCHA_MAUVE))),
                        Line::from(Span::styled("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░          ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░        ", Style::default().fg(MOCHA_MAUVE))),
                        Line::from(Span::styled("░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░          ░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░        ", Style::default().fg(MOCHA_MAUVE))),
                        Line::from(Span::styled("░▒▓█▓▒░      ░▒▓████████▓▒░▒▓████████▓▒░ ░▒▓█▓▒░          ░▒▓█▓▒░      ░▒▓████████▓▒░▒▓██████▓▒░ ░▒▓██████▓▒░   ", Style::default().fg(MOCHA_MAUVE))),
                        Line::from(Span::styled("░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░          ░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░        ", Style::default().fg(MOCHA_MAUVE))),
                        Line::from(Span::styled("░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░          ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░        ", Style::default().fg(MOCHA_MAUVE))),
                        Line::from(Span::styled(" ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░           ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓████████▓▒░ ", Style::default().fg(MOCHA_MAUVE))),
                        Line::default(), // Bottom padding
                    ];
                    let title = Paragraph::new(ascii_lines)
                        .alignment(ratatui::layout::Alignment::Center)
                        .block(Block::default().borders(Borders::NONE));
                    f.render_widget(title, main_chunks[0]);

                    // Split main body horizontally
                    let body_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(50), // Left: Username Input form
                            Constraint::Percentage(50), // Right: Active Users list
                        ].as_ref())
                        .split(main_chunks[1]);

                    // Left Side: Login Form
                    let form_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3), // Instructions
                            Constraint::Length(3), // Input block
                            Constraint::Min(0),
                        ].as_ref())
                        .split(body_chunks[0]);

                    let prompt_span = if let Some(ref err) = history.login_error {
                        Span::styled(format!("✗ Error: {}", err), Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled("▶ Enter a unique username to join:", Style::default().fg(MOCHA_GREEN))
                    };
                    let instructions = Paragraph::new(vec![Line::from(prompt_span)])
                        .block(Block::default().borders(Borders::NONE));
                    f.render_widget(instructions, form_chunks[0]);

                    let prompt_text = format!("> {}", input_buffer);
                    let input_box = Paragraph::new(prompt_text)
                        .block(Block::default()
                            .title(Span::styled(" Choose Username ", Style::default().fg(MOCHA_PEACH).add_modifier(Modifier::BOLD)))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(MOCHA_SURFACE1))
                        )
                        .style(Style::default().fg(MOCHA_TEXT));
                    f.render_widget(input_box, form_chunks[1]);

                    // Right Side: Active Users List
                    let active_title = Span::styled(
                        format!(" Connected Users ({}) ", history.active_usernames.len()),
                        Style::default().fg(MOCHA_TEAL).add_modifier(Modifier::BOLD)
                    );
                    let mut user_items = Vec::new();
                    if history.active_usernames.is_empty() {
                        user_items.push(ListItem::new(Span::styled("No other users online. Be the first to join!", Style::default().fg(MOCHA_SUBTEXT0))));
                    } else {
                        for user in &history.active_usernames {
                            user_items.push(ListItem::new(Line::from(vec![
                                Span::styled("• ", Style::default().fg(MOCHA_TEAL)),
                                Span::styled(user, Style::default().fg(MOCHA_TEXT)),
                            ])));
                        }
                    }
                    let users_list = List::new(user_items)
                        .block(Block::default()
                            .title(active_title)
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(MOCHA_SURFACE1))
                        );
                    f.render_widget(users_list, body_chunks[1]);

                    // Footer
                    let footer = Paragraph::new(vec![
                        Line::from(vec![
                            Span::styled("Press ", Style::default().fg(MOCHA_SUBTEXT0)),
                            Span::styled("Enter", Style::default().fg(MOCHA_PEACH).add_modifier(Modifier::BOLD)),
                            Span::styled(" to join  |  Press ", Style::default().fg(MOCHA_SUBTEXT0)),
                            Span::styled("Esc", Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD)),
                            Span::styled(" to quit", Style::default().fg(MOCHA_SUBTEXT0)),
                        ])
                    ])
                    .alignment(ratatui::layout::Alignment::Center);
                    f.render_widget(footer, main_chunks[2]);
                }
                State::ChatRoom => {
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

                        let spans = match &msg.message {
                            ServerMessage::Command(s) => match s {
                                ServerCommand::Welcome(_id) => {
                                    let username_str = history.own_username.as_deref().unwrap_or("You");
                                    vec![
                                        Span::styled(format!("[{}] ", time_str), Style::default().fg(MOCHA_OVERLAY0)),
                                        Span::styled("[SERVER]: ", Style::default().fg(MOCHA_YELLOW).add_modifier(Modifier::BOLD)),
                                        Span::styled("Welcome to the chat! You are ", Style::default().fg(MOCHA_TEXT)),
                                        Span::styled(username_str, Style::default().fg(MOCHA_PEACH).add_modifier(Modifier::BOLD)),
                                    ]
                                }
                                ServerCommand::LoginError(reason) => {
                                    vec![
                                        Span::styled(format!("[{}] ", time_str), Style::default().fg(MOCHA_OVERLAY0)),
                                        Span::styled("[SERVER ERROR]: ", Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD)),
                                        Span::styled(reason.to_string(), Style::default().fg(MOCHA_TEXT)),
                                    ]
                                }
                                ServerCommand::ActiveUsers { .. } => {
                                    continue;
                                }
                                ServerCommand::Joined(username) => {
                                    vec![
                                        Span::styled(format!("[{}] ", time_str), Style::default().fg(MOCHA_OVERLAY0)),
                                        Span::styled("[SERVER]: ", Style::default().fg(MOCHA_YELLOW).add_modifier(Modifier::BOLD)),
                                        Span::styled(username.clone(), Style::default().fg(MOCHA_TEAL).add_modifier(Modifier::BOLD)),
                                        Span::styled(" joined the chat!", Style::default().fg(MOCHA_TEXT)),
                                    ]
                                }
                                ServerCommand::Disconnect => {
                                    vec![
                                        Span::styled(format!("[{}] ", time_str), Style::default().fg(MOCHA_OVERLAY0)),
                                        Span::styled("[SERVER]: ", Style::default().fg(MOCHA_YELLOW).add_modifier(Modifier::BOLD)),
                                        Span::styled("Disconnected.", Style::default().fg(MOCHA_TEXT)),
                                    ]
                                }
                            },
                            ServerMessage::Chat(ChatMessage {
                                author_id, author_username, content, ..
                            }) => {
                                let content_str = match content {
                                    MessageContent::Text(t) => t.clone(),
                                };
                                let (sender_name, sender_color) = if Some(*author_id) == history.own_id {
                                    ("You".to_string(), MOCHA_PEACH)
                                } else {
                                    (author_username.clone(), MOCHA_TEAL)
                                };
                                vec![
                                    Span::styled(format!("[{}] ", time_str), Style::default().fg(MOCHA_OVERLAY0)),
                                    Span::styled(format!("[{}]: ", sender_name), Style::default().fg(sender_color).add_modifier(Modifier::BOLD)),
                                    Span::styled(content_str, Style::default().fg(MOCHA_TEXT)),
                                ]
                            }
                        };
                        list_items.push(ListItem::new(Line::from(spans)));
                    }

                    let mut state = ListState::default();
                    if !list_items.is_empty() {
                        state.select(Some(list_items.len() - 1));
                    }

                    let history_list = List::new(list_items)
                        .block(Block::default()
                            .title(Span::styled(" Chat History ", Style::default().fg(MOCHA_LAVENDER).add_modifier(Modifier::BOLD)))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(MOCHA_SURFACE1))
                        );

                    f.render_stateful_widget(history_list, chunks[0], &mut state);

                    let input_paragraph = Paragraph::new(wrapped_prompt)
                        .block(Block::default()
                            .title(Span::styled(" Input ", Style::default().fg(MOCHA_PEACH).add_modifier(Modifier::BOLD)))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(MOCHA_SURFACE1))
                        )
                        .style(Style::default().fg(MOCHA_TEXT));
                    f.render_widget(input_paragraph, chunks[1]);
                }
                _ => {}
            }
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
