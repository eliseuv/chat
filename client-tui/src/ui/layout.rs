use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use crate::ui::theme::active_theme::*;

/// Simulates word wrapping on the input buffer and returns the wrapped string and line count.
pub fn wrap_text(text: &str, max_width: usize) -> (String, usize) {
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

/// Helper function to create a centered rect of a specific size inside a parent rect.
pub fn centered_rect(width: u16, height: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}

/// Helper function to dynamically construct a footer line showing colored keybindings with dim descriptions.
pub fn make_keybindings_footer<'a>(bindings: &[(&'a str, &'a str, Color)]) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, &(key, desc, color)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  |  ", Style::default().fg(SUBTEXT0)));
        }
        spans.push(Span::styled(key, Style::default().fg(color).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(format!(" {}", desc), Style::default().fg(SUBTEXT0)));
    }
    Line::from(spans)
}
