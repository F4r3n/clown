use std::collections::HashSet;

use crate::{component::Draw, irc_view::color_user::nickname_color};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Styled},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState},
};
pub struct UsersWidget {
    list_users: HashSet<String>,
    list_state: ListState,
    focus: bool,
    area: Rect,
}

impl UsersWidget {
    pub fn new() -> Self {
        Self {
            list_users: HashSet::new(),
            focus: false,
            area: Rect::default(),
            list_state: ListState::default(),
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
        self.area = area;
        let border_style = if self.has_focus() {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let items = self
            .list_users
            .iter()
            .map(|content| {
                ListItem::from(Line::from(vec![Span::styled(
                    " ".to_string() + content,
                    nickname_color(&content.replace("@", "").to_string()),
                )]))
            })
            .collect::<Vec<ListItem>>();

        let mut list = List::new(items);

        if self.has_focus() {
            list = list.set_style(border_style);
        }

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}
use crate::message_event::MessageEvent;
impl crate::component::EventHandler for UsersWidget {
    fn get_area(&self) -> Rect {
        self.area
    }
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
