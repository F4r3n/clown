use std::collections::HashSet;

use crate::component::Draw;
use ratatui::{
    style::{Color, Style, Styled},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};
pub struct UsersWidget {
    list_users: HashSet<String>,
    focus: bool,
}

impl UsersWidget {
    pub fn new() -> Self {
        Self {
            list_users: HashSet::new(),
            focus: false,
        }
    }
    fn has_focus(&self) -> bool {
        self.focus
    }
    pub fn set_users(&mut self, list_users: Vec<String>) {
        self.list_users = HashSet::from_iter(list_users.iter().cloned());
    }

    pub fn replace_user(&mut self, old: &str, new: &str) {
        self.list_users.remove(old);
        self.list_users.insert(new.to_string());
    }

    pub fn remove_user(&mut self, user: &str) {
        self.list_users.remove(user);
    }

    pub fn add_user(&mut self, user: &str) {
        self.list_users.insert(user.to_string());
    }
}

impl Draw for UsersWidget {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let text_style = Style::default().fg(Color::White);
        let border_style = if self.has_focus() {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let lines = self
            .list_users
            .iter()
            .map(|content| Line::from(vec![Span::styled(content, text_style)]))
            .collect::<Vec<Line>>();

        let text = Text::from(lines);

        let mut paragraph = Paragraph::new(text);
        if self.has_focus() {
            paragraph = paragraph
                .block(Block::bordered().title("Users"))
                .set_style(border_style);
        }

        frame.render_widget(paragraph, area);
    }
}
use crate::message_event::MessageEvent;
impl crate::component::EventHandler for UsersWidget {
    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::UpdateUsers(list_users) => {
                self.set_users(list_users.to_vec());
                None
            }
            MessageEvent::ReplaceUser(old, new) => {
                crate::logger::log_info_sync(
                    format!("Replace User {:?} {}\n", &old, &new).as_str(),
                );

                self.replace_user(old, new);
                None
            }
            MessageEvent::RemoveUser(user) => {
                self.remove_user(user);
                None
            }
            MessageEvent::JoinUser(user) => {
                self.add_user(user);
                None
            }
            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        _event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        None
    }
    fn has_focus(&self) -> bool {
        self.has_focus()
    }
    fn set_focus(&mut self, focused: bool) {
        self.focus = focused
    }
}
