use crate::MessageEvent;
use crate::component::Draw;

use crossterm::event::KeyCode;

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_input::{Input, backend::crossterm::EventHandler};
#[derive(Debug, Default, Clone)]

pub struct CInput {
    input: Input,
    /// Current input mode
    area: Rect,
}

impl Draw for CInput {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.area = area;
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);

        let input = Paragraph::new(Line::from(vec![
            Span::from(">").style(Style::default().fg(ratatui::style::Color::Cyan)),
            Span::from(self.input.value()),
        ]))
        .scroll((0, scroll as u16));
        frame.render_widget(input, area);

        // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
        let x = self.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y))
    }
}

impl crate::component::EventHandler for CInput {
    fn get_area(&self) -> Rect {
        self.area
    }

    fn handle_actions(&mut self, _event: &MessageEvent) -> Option<MessageEvent> {
        None
    }

    fn handle_events(&mut self, event: &crate::event_handler::Event) -> Option<MessageEvent> {
        let mut message = None;
        if let Some(key_event) = event.get_key() {
            message = match key_event.code {
                KeyCode::Enter => {
                    let m = self.get_current_input();
                    self.reset_value();
                    if !m.is_empty() {
                        Some(MessageEvent::MessageInput(m))
                    } else {
                        None
                    }
                }
                _ => {
                    if let crate::event_handler::Event::Crossterm(cross) = &event {
                        self.input.handle_event(cross);
                    }
                    None
                }
            };
        }

        message
    }
}

impl CInput {
    pub fn new() -> Self {
        Self {
            input: Input::new(String::from("")),
            area: Rect::default(),
        }
    }

    pub fn get_current_input(&self) -> String {
        self.input.to_string()
    }

    pub fn reset_value(&mut self) {
        self.input.reset();
    }
}
