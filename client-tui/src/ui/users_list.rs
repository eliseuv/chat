use crate::ui::theme::active_theme::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Widget};

pub struct ActiveUsersList<'a> {
    active_usernames: &'a [String],
    own_username: Option<&'a str>,
    title: &'a str,
}

impl<'a> ActiveUsersList<'a> {
    pub fn new(active_usernames: &'a [String]) -> Self {
        Self {
            active_usernames,
            own_username: None,
            title: "Users",
        }
    }

    pub fn own_username(mut self, username: Option<&'a str>) -> Self {
        self.own_username = username;
        self
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }
}

impl<'a> Widget for ActiveUsersList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let active_title = Span::styled(
            format!(" {} ({}) ", self.title, self.active_usernames.len()),
            Style::default().fg(TEAL).add_modifier(Modifier::BOLD),
        );

        let mut user_items = Vec::new();
        if self.active_usernames.is_empty() {
            user_items.push(ListItem::new(Span::styled(
                "No other users online. Be the first to join!",
                Style::default().fg(SUBTEXT0),
            )));
        } else {
            for user in self.active_usernames {
                let is_me = self.own_username.is_some_and(|own| own == user);
                let span_color = if is_me { PEACH } else { TEXT };
                let mut line_spans = vec![
                    Span::styled(
                        "• ",
                        Style::default().fg(if is_me { PEACH } else { TEAL }),
                    ),
                    Span::styled(user, Style::default().fg(span_color)),
                ];
                if is_me {
                    line_spans.push(Span::styled(
                        " (you)",
                        Style::default()
                            .fg(SUBTEXT0)
                            .add_modifier(Modifier::ITALIC),
                    ));
                }
                user_items.push(ListItem::new(Line::from(line_spans)));
            }
        }

        let users_list = List::new(user_items).block(
            Block::default()
                .title(active_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(SURFACE1)),
        );

        users_list.render(area, buf);
    }
}
