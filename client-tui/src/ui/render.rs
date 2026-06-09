use crate::history::ChatHistory;
use crate::app::State;
use crate::ui::theme::*;
use crate::ui::layout::{centered_rect, make_keybindings_footer, wrap_text};
use anyhow::Context;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{execute, tty::IsTty};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use server::protocol::{ChatMessage, MessageContent};
use server::remote::packet::{ServerCommand, ServerMessage};
use std::io;

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
    pub fn draw(
        &mut self,
        state: State,
        history: &ChatHistory,
        input_buffer: &str,
        show_confirm_quit: bool,
    ) -> anyhow::Result<()> {
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

                    // Top Banner (ASCII Art with padding from client-tui/assets/title.txt)
                    let mut ascii_lines = vec![Line::default()]; // Top padding
                    for line in include_str!("../../assets/title.txt").lines() {
                        ascii_lines.push(Line::from(Span::styled(line, Style::default().fg(MOCHA_MAUVE))));
                    }
                    ascii_lines.push(Line::default()); // Bottom padding
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
                    let footer = Paragraph::new(make_keybindings_footer(&[
                        ("Enter", "join", MOCHA_PEACH),
                        ("Ctrl+Q", "quit", MOCHA_RED),
                    ]))
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
                        .constraints([
                            Constraint::Length(1),                  // Little header
                            Constraint::Min(1),                     // Main chat + sidebar body
                            Constraint::Length(input_height),       // Input field
                            Constraint::Length(1),                  // Footer tips
                        ].as_ref())
                        .split(f.area());

                    let (history_area, users_area) = if f.area().width >= 60 {
                        let body_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Min(0),
                                Constraint::Length(25),
                            ].as_ref())
                            .split(chunks[1]);
                        (body_chunks[0], Some(body_chunks[1]))
                    } else {
                        (chunks[1], None)
                    };

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
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(MOCHA_SURFACE1))
                        );

                    f.render_stateful_widget(history_list, history_area, &mut state);

                    if let Some(area) = users_area {
                        let active_title = Span::styled(
                            format!(" Users ({}) ", history.active_usernames.len()),
                            Style::default().fg(MOCHA_TEAL).add_modifier(Modifier::BOLD)
                        );
                        let mut user_items = Vec::new();
                        for user in &history.active_usernames {
                            let is_me = history.own_username.as_ref().map_or(false, |own| own == user);
                            let span_color = if is_me { MOCHA_PEACH } else { MOCHA_TEXT };
                            let mut line_spans = vec![
                                Span::styled("• ", Style::default().fg(if is_me { MOCHA_PEACH } else { MOCHA_TEAL })),
                                Span::styled(user, Style::default().fg(span_color)),
                            ];
                            if is_me {
                                line_spans.push(Span::styled(" (you)", Style::default().fg(MOCHA_SUBTEXT0).add_modifier(Modifier::ITALIC)));
                            }
                            user_items.push(ListItem::new(Line::from(line_spans)));
                        }
                        let users_list = List::new(user_items)
                            .block(Block::default()
                                .title(active_title)
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(MOCHA_SURFACE1))
                            );
                        f.render_widget(users_list, area);
                    }

                    let header = Paragraph::new(Line::from(vec![
                        Span::styled(" CHAT CAFE", Style::default().fg(MOCHA_MAUVE).add_modifier(Modifier::BOLD)),
                    ]))
                    .alignment(ratatui::layout::Alignment::Left);
                    f.render_widget(header, chunks[0]);

                    let input_paragraph = Paragraph::new(wrapped_prompt)
                        .block(Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(MOCHA_SURFACE1))
                        )
                        .style(Style::default().fg(MOCHA_TEXT));
                    f.render_widget(input_paragraph, chunks[2]);

                    let footer = Paragraph::new(make_keybindings_footer(&[
                        ("Enter", "send", MOCHA_PEACH),
                        ("Ctrl+Q", "exit", MOCHA_RED),
                    ]))
                    .alignment(ratatui::layout::Alignment::Center);
                    f.render_widget(footer, chunks[3]);
                }
                _ => {}
            }

            if show_confirm_quit {
                let area = f.area();
                let popup_area = centered_rect(46, 8, area);
                f.render_widget(Clear, popup_area);
                
                let popup_block = Block::default()
                    .title(Span::styled(" Confirm Exit ", Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MOCHA_RED));
                
                let text = vec![
                    Line::default(),
                    Line::from(Span::styled("Are you sure you want to exit?", Style::default().fg(MOCHA_TEXT))),
                    Line::default(),
                    Line::from(vec![
                        Span::styled("  [Y] ", Style::default().fg(MOCHA_GREEN).add_modifier(Modifier::BOLD)),
                        Span::styled("Yes, Exit   ", Style::default().fg(MOCHA_SUBTEXT0)),
                        Span::styled("[N] ", Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD)),
                        Span::styled("No, Stay  ", Style::default().fg(MOCHA_SUBTEXT0)),
                    ]),
                ];
                
                let paragraph = Paragraph::new(text)
                    .block(popup_block)
                    .alignment(ratatui::layout::Alignment::Center);
                
                f.render_widget(paragraph, popup_area);
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
