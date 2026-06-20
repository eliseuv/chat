use crate::ui::layout::centered_rect;
use crate::ui::theme::active_theme::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

pub struct ConfirmExitPopup;

impl Widget for ConfirmExitPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_area = centered_rect(46, 8, area);
        Clear.render(popup_area, buf);

        let popup_block = Block::default()
            .title(Span::styled(
                " Confirm Exit ",
                Style::default().fg(RED).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(RED));

        let text = vec![
            Line::default(),
            Line::from(Span::styled(
                "Are you sure you want to exit?",
                Style::default().fg(TEXT),
            )),
            Line::default(),
            Line::from(vec![
                Span::styled(
                    "  [Y] ",
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Yes, Exit   ", Style::default().fg(SUBTEXT0)),
                Span::styled(
                    "[N] ",
                    Style::default().fg(RED).add_modifier(Modifier::BOLD),
                ),
                Span::styled("No, Stay  ", Style::default().fg(SUBTEXT0)),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(popup_block)
            .alignment(ratatui::layout::Alignment::Center);

        paragraph.render(popup_area, buf);
    }
}
