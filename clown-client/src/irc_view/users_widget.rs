use crate::{component::Draw, irc_view::color_user::nickname_color};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
};

#[derive(Debug, PartialEq)]
struct User {
    pub admin: bool,
    pub name: String,
    pub need_hightlight: bool,
}

impl User {
    pub fn new(name: &str) -> Self {
        let is_admin = name.contains("@");
        Self {
            admin: is_admin,
            name: name.replace("@", ""),
            need_hightlight: false,
        }
    }
}

impl From<String> for User {
    fn from(name: String) -> Self {
        let is_admin = name.contains("@");
        Self {
            admin: is_admin,
            name: name.replace("@", ""),
            need_hightlight: false,
        }
    }
}

impl From<&str> for User {
    fn from(name: &str) -> Self {
        let is_admin = name.contains("@");
        Self {
            admin: is_admin,
            name: name.replace("@", ""),
            need_hightlight: false,
        }
    }
}

pub struct UsersWidget {
    list_users: Vec<User>,
    list_state: ListState,
    focus: bool,
    area: Rect,
    main_channel: String,
}

impl UsersWidget {
    pub fn new(main_channel: &str) -> Self {
        Self {
            list_users: vec![],
            focus: false,
            area: Rect::default(),
            list_state: ListState::default(),
            main_channel: main_channel.to_string(),
        }
    }
    fn has_focus(&self) -> bool {
        self.focus
    }
    pub fn set_users(&mut self, list_users: Vec<String>) {
        self.list_state.select(Some(0));
        self.list_users.clear();
        self.list_users
            .push(User::new(self.main_channel.clone().as_str()));
        let mut users_from_strings: Vec<User> = list_users.into_iter().map(User::from).collect();
        self.list_users.append(&mut users_from_strings);
    }

    pub fn replace_user(&mut self, old: &str, new: &str) {
        if let Some(id) = self.list_users.iter().position(|v| v.name.eq(old)) {
            self.list_users.remove(id);
        }
        self.list_users.push(new.into());
    }

    pub fn remove_user(&mut self, user: &str) {
        if let Some(id) = self.list_users.iter().position(|v| v.name.eq(user)) {
            self.list_users.remove(id);
        }
    }

    pub fn hightlight_user(&mut self, user: &str) {
        if let Some(user) = self.list_users.iter_mut().find(|v| v.name.eq(user)) {
            user.need_hightlight = true;
        }
    }

    pub fn add_user(&mut self, user: &str) {
        if self
            .list_users
            .iter()
            .position(|v| v.name.eq(user))
            .is_none()
        {
            self.list_users.push(user.into());
        }
    }
}

impl Draw for UsersWidget {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        self.area = area;
        let focus_style = if self.has_focus() {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default()
        };

        let selected = self.list_state.selected().unwrap_or_default();
        let mut items = Vec::with_capacity(self.list_users.len());
        for (id, content) in self.list_users.iter().enumerate() {
            let mut spans = Vec::new();
            let mut style = Style::default().fg(nickname_color(&content.name));

            if id == selected {
                spans.push(Span::styled(">", focus_style));
                style = style.add_modifier(Modifier::BOLD)
            }

            if content.need_hightlight {
                style = style.bg(Color::LightBlue);
            }
            spans.push(Span::raw(" "));
            spans.push(Span::styled(content.name.as_str(), style));
            let item = ListItem::from(Line::from(spans));
            items.push(item);
        }

        let list = List::new(items);

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
                self.replace_user(old, new);
                None
            }
            MessageEvent::RemoveUser(user) => {
                self.remove_user(user);
                None
            }
            MessageEvent::HighlightUser(user) => {
                if !self.main_channel.eq(user) {
                    self.hightlight_user(user);
                }
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
        event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        if let Some(key) = event.get_key() {
            if key.is_release() {
                match key.code {
                    crossterm::event::KeyCode::Up => {
                        self.list_state.select_previous();
                    }
                    crossterm::event::KeyCode::Down => {
                        self.list_state.select_next();
                    }
                    crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char(' ') => {
                        if let Some(current_id) = self.list_state.selected()
                            && let Some(user) = self.list_users.get_mut(current_id)
                        {
                            user.need_hightlight = false;
                            return Some(MessageEvent::SelectChannel(user.name.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }
    fn has_focus(&self) -> bool {
        self.has_focus()
    }
    fn set_focus(&mut self, focused: bool) {
        self.focus = focused
    }
}
