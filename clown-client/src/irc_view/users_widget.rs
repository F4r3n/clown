use crate::component::Draw;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};
pub struct UsersWidget {
    list_users: Vec<String>,
}

impl UsersWidget {
    pub fn new() -> Self {
        Self { list_users: vec![] }
    }
    pub fn set_users(&mut self, list_users: Vec<String>) {
        self.list_users = list_users;
    }
}

impl Draw for UsersWidget {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let text_style = Style::default().fg(Color::White);

        let lines = self
            .list_users
            .iter()
            .map(|content| Line::from(vec![Span::styled(content, text_style)]))
            .collect::<Vec<Line>>();

        let text = Text::from(lines);

        let paragraph = Paragraph::new(text).block(
            Block::bordered()
                .title("users")
                .border_style(Style::default()),
        );

        frame.render_widget(paragraph, area);
    }
}
impl crate::component::EventHandler for UsersWidget {
    fn handle_actions(
        &mut self,
        event: &crate::message_event::MessageEvent,
    ) -> Option<crate::message_event::MessageEvent> {
        match event {
            crate::message_event::MessageEvent::UpdateUsers(list_users) => {
                self.set_users(list_users.to_vec());
                None
            }
            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        None
    }
    fn has_focus(&self) -> bool {
        false
    }
    fn set_focus(&mut self, _focused: bool) {}
}
