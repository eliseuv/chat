use crate::ui::theme::{MOCHA_SURFACE1, MOCHA_TEXT};
use crate::ui::layout::wrap_text;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub struct InputPrompt<'a> {
    input_buffer: &'a str,
    title: Option<Span<'a>>,
}

impl<'a> InputPrompt<'a> {
    pub fn new(input_buffer: &'a str) -> Self {
        Self {
            input_buffer,
            title: None,
        }
    }

    pub fn title(mut self, title: Span<'a>) -> Self {
        self.title = Some(title);
        self
    }

    fn text(&self) -> String {
        format!("> {}█", self.input_buffer)
    }

    pub fn lines_required(&self, width: usize) -> u16 {
        let (_, lines) = wrap_text(&self.text(), width);
        (lines as u16).clamp(1, 5) + 2
    }
}

impl<'a> Widget for InputPrompt<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (wrapped_prompt, _) = wrap_text(&self.text(), area.width as usize);

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MOCHA_SURFACE1));

        if let Some(t) = self.title {
            block = block.title(t);
        }

        Paragraph::new(wrapped_prompt)
            .block(block)
            .style(Style::default().fg(MOCHA_TEXT))
            .render(area, buf);
    }
}
