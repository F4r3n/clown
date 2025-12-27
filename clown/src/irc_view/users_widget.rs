use crate::{component::Draw, irc_view::color_user::nickname_color};
use ahash::AHashMap;
use bit_vec::BitVec;
use crossterm::event::KeyModifiers;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};
use tracing::debug;
#[derive(Debug, PartialEq)]
enum ChannelMode {
    User,
    Channel,
}

#[derive(Debug, PartialEq)]
struct User {
    pub channel_mode: ChannelMode,
    pub name: String,
    pub need_hightlight: bool,
    pub color: ratatui::style::Color,
    pub connected_channels: BitVec,
}

impl User {
    pub fn new(name: String) -> Self {
        Self {
            channel_mode: ChannelMode::User,
            need_hightlight: false,
            color: nickname_color(&name),
            name,
            connected_channels: BitVec::from_elem(32, false),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.color = nickname_color(&self.name);
    }

    pub fn join_channel(&mut self, id: usize) {
        if let Some(mut channel) = self.connected_channels.get_mut(id) {
            *channel = true;
        }
    }

    pub fn quit_channel(&mut self, id: usize) {
        if let Some(mut channel) = self.connected_channels.get_mut(id) {
            *channel = false;
        }
    }
}

impl DrawableItem for User {
    fn display_color(&self) -> ratatui::style::Color {
        self.color
    }

    fn display_title(&self) -> &str {
        &self.name
    }

    fn is_highlighted(&self) -> bool {
        self.need_hightlight
    }
}

#[derive(Debug)]
struct RegisteredChannel {
    name: String,
    id: usize,
    color: ratatui::style::Color,
    highlight: bool,
}

impl RegisteredChannel {
    pub fn new(name: String, id: usize) -> Self {
        Self {
            color: nickname_color(&name),
            name,
            id,
            highlight: false,
        }
    }
}

trait DrawableItem {
    fn display_title(&self) -> &str;
    fn display_color(&self) -> ratatui::style::Color;
    fn is_highlighted(&self) -> bool;
}

#[derive(Debug)]
struct Section {
    pub channel_info: RegisteredChannel,
    pub order_user: Vec<String>,
}

impl Section {
    fn new(channel_name: String, id: usize) -> Self {
        Self {
            channel_info: RegisteredChannel::new(channel_name, id),
            order_user: Vec::new(),
        }
    }

    fn set_user_position(&mut self, user: &str) {
        if let Some(id) = self.order_user.iter().position(|v| v.eq(&user)) {
            self.order_user.remove(id);
        }
        self.order_user.push(user.to_string());
    }

    fn remove_user(&mut self, user: &str) {
        if let Some(id) = self.order_user.iter().position(|v| v.eq(&user)) {
            self.order_user.remove(id);
        }
    }
}

//List channel contains all the channels
// global is the anonymous channel where the user has sent a message it will be displayed first
// it can also contain unseen user, when we register a user if no channel found we add it in this zone
// list users contain to which channel a user belong. The ID is based on the index of the list_sections
pub struct UsersWidget {
    list_sections: Vec<Section>,
    list_users: ahash::AHashMap<String, User>,

    list_state: ListStateWidget,
    area: Rect,
}

impl UsersWidget {
    pub fn new() -> Self {
        Self {
            list_users: AHashMap::new(),
            area: Rect::default(),
            list_sections: Vec::new(),
            list_state: ListStateWidget::new(),
        }
    }

    fn get_section_id(&self, channel: &str) -> Option<usize> {
        self.list_sections
            .iter()
            .find(|c| c.channel_info.name == channel)
            .map(|c| c.channel_info.id)
    }

    fn add_channel(&mut self, channel: String) -> Option<usize> {
        let index = if let Some(i) = self
            .list_sections
            .iter()
            .position(|c| c.channel_info.name == channel)
        {
            i
        } else {
            let i = self.list_sections.len();
            self.list_sections
                .push(Section::new(channel.to_string(), i));
            i
        };
        self.list_sections.get(index).map(|c| c.channel_info.id)
    }

    fn nb_sections(&self) -> usize {
        self.list_sections.len()
    }

    fn nb_items(&self, section_id: usize) -> usize {
        if let Some(section) = self.list_sections.get(section_id) {
            section.order_user.len() + 1
        } else {
            1
        }
    }

    fn set_users(&mut self, channel: &str, list_users: Vec<String>) {
        let channel_id = self.add_channel(channel.to_string());

        if let Some(channel) = channel_id {
            for user in list_users {
                self.add_user(channel, &user, ChannelMode::User);
            }
        }
    }

    fn replace_user(&mut self, old: &str, new: &str) {
        let old = UsersWidget::sanitize_name(old);
        let new = UsersWidget::sanitize_name(new);

        let v = self.list_users.remove(old);

        if let Some(mut v) = v {
            let n = new;
            v.set_name(n.to_string());
            self.list_users.insert(n.to_string(), v);
        }
    }

    fn remove_user(&mut self, channel_id: Option<usize>, user: &str) {
        let user = UsersWidget::sanitize_name(user);

        if let Some(id) = channel_id {
            if let Some(u) = self.list_users.get_mut(user) {
                if let Some(channel) = self.list_sections.get_mut(id) {
                    channel.remove_user(user);
                }

                u.quit_channel(id);
            }
        } else {
            self.list_users.remove(user);
        }
    }

    fn hightlight_user(&mut self, user: &str) {
        if let Some(user) = self.list_users.get_mut(user) {
            user.need_hightlight = true;
        } else if let Some(id) = self.get_section_id(user)
            && let Some(section) = self.list_sections.get_mut(id)
        {
            section.channel_info.highlight = true;
        }
    }

    fn sanitize_name(user: &str) -> &str {
        user.strip_prefix('@').unwrap_or(user)
    }

    fn add_user_with_channel(&mut self, channel: &str, user: &str) {
        if let Some(channel_id) = self.get_section_id(channel) {
            self.add_user(channel_id, user, ChannelMode::User);
        } else if let Some(channel_id) = self.add_channel(channel.to_string()) {
            self.add_user(channel_id, user, ChannelMode::User);
        }
    }

    fn add_user(&mut self, channel_id: usize, user: &str, mode: ChannelMode) {
        let user = UsersWidget::sanitize_name(user).to_string();
        if mode == ChannelMode::User {
            if let Some(channel) = self.list_sections.get_mut(channel_id) {
                channel.set_user_position(&user);
            }
            if let Some(user) = self.list_users.get_mut(&user) {
                user.join_channel(channel_id);
            } else {
                let mut new_user = User::new(user.to_string());
                new_user.join_channel(channel_id);
                self.list_users.insert(user, new_user);
            }
        }
    }
}

impl Draw for UsersWidget {
    fn render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect) {
        self.area = area;
        self.list_state
            .render(&self.list_sections, &self.list_users, frame, area);
    }
}
//#channel
// user1
// user2
//#channel2
// user3
// user2
#[derive(Clone, Debug)]
pub struct ListStateWidget {
    current_section: usize,  //#global, #chan
    current_selected: usize, //0 is the main channel, users starts at 1
}

impl ListStateWidget {
    fn new() -> Self {
        Self {
            current_section: 1,
            current_selected: 0,
        }
    }

    fn selected(&self) -> (usize, usize) {
        (self.current_section, self.current_selected)
    }

    fn next(&mut self, max_section: usize) {
        self.current_selected = self.current_selected.saturating_add(1) % max_section;
    }

    fn previous(&mut self, max_section: usize) {
        self.current_selected = self.current_selected.saturating_sub(1) % max_section;
    }

    fn next_section(&mut self, max_nb_sections: usize) {
        self.current_section = self.current_section.saturating_add(1) % max_nb_sections;
        self.current_selected = 0;
    }

    fn previous_section(&mut self, max_nb_sections: usize) {
        self.current_section = self.current_section.saturating_sub(1) % max_nb_sections;
        self.current_selected = 0;
    }

    fn add_item<'a>(
        &'a self,
        depth: usize,
        color: ratatui::style::Color,
        title: &'a str,
        is_highlighted: bool,
        is_selected: bool,
    ) -> Vec<ratatui::text::Span<'a>> {
        let mut spans = Vec::new();
        let mut style = Style::default().fg(color);

        if is_selected {
            style = style.add_modifier(Modifier::BOLD)
        }

        if is_highlighted {
            style = style.bg(Color::LightBlue);
        }
        spans.push(Span::raw(format!("{:<width$}", " ", width = depth + 1)));
        spans.push(Span::styled(title, style));
        spans
        //ListItem::from(Line::from(spans))
    }

    fn render(
        &mut self,
        sections: &[Section],
        users: &AHashMap<String, User>,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        let mut items = Vec::new();

        for (section_i, section) in sections.iter().enumerate() {
            let item = ListItem::from(Line::from(self.add_item(
                0,
                section.channel_info.color,
                &section.channel_info.name,
                section.channel_info.highlight,
                (self.current_section == section_i) && (self.current_selected == 0),
            )));
            items.push(item);

            for (i, user_name) in section.order_user.iter().enumerate() {
                if let Some(user) = users.get(user_name) {
                    let spans = self.add_item(
                        1,
                        user.display_color(),
                        user.display_title(),
                        user.is_highlighted(),
                        (self.current_section == section_i) && (self.current_selected == (i + 1)),
                    );

                    let item = ListItem::from(Line::from(spans));
                    items.push(item);
                }
            }
        }
        let list = List::new(items);
        frame.render_widget(list, area);
    }
}

use crate::message_event::MessageEvent;
impl crate::component::EventHandler for UsersWidget {
    fn get_area(&self) -> Rect {
        self.area
    }
    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::UpdateUsers(channel, list_users) => {
                self.set_users(channel, list_users.to_vec());
                None
            }
            MessageEvent::ReplaceUser(old, new) => {
                self.replace_user(old, new);
                None
            }
            MessageEvent::RemoveUser(channel, user) => {
                self.remove_user(channel.as_ref().and_then(|c| self.get_section_id(c)), user);
                None
            }
            MessageEvent::HighlightUser(user) => {
                self.hightlight_user(user);

                None
            }
            MessageEvent::JoinChannel(channel) => {
                self.add_channel(channel.to_string());
                None
            }
            MessageEvent::JoinUser(channel, user) => {
                self.add_user_with_channel(channel, user);
                None
            }
            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        if let Some(key) = event.get_key()
            && key.is_press()
            && !key.is_repeat()
        {
            let previous = self.list_state.clone();
            let previous_selected = previous.selected();
            let number_items = self.nb_items(self.list_state.current_section);
            let number_sections = self.nb_sections();

            if key.modifiers.contains(KeyModifiers::CONTROL) && number_items > 0 {
                match key.code {
                    crossterm::event::KeyCode::Char('p') => {
                        self.list_state.previous(number_items);
                    }
                    crossterm::event::KeyCode::Char('n') => {
                        self.list_state.next(number_items);
                    }
                    _ => {}
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) && number_sections > 0 {
                match key.code {
                    crossterm::event::KeyCode::Up => {
                        self.list_state.previous_section(number_sections);
                    }
                    crossterm::event::KeyCode::Down => {
                        self.list_state.next_section(number_sections);
                    }
                    _ => {}
                }
            }

            let (selected, id) = self.list_state.selected();
            if (previous_selected.0 != selected) || (previous_selected.1 != id) {
                if id > 0
                    && let Some(channel) = self.list_sections.get(selected)
                    && let Some(user_name) = channel.order_user.get(id - 1)
                    && let Some(user) = self.list_users.get_mut(user_name)
                {
                    user.need_hightlight = false;
                    return Some(MessageEvent::SelectChannel(user.name.to_string()));
                } else if let Some(channel) = self.list_sections.get_mut(selected) {
                    channel.channel_info.highlight = false;
                    return Some(MessageEvent::SelectChannel(
                        channel.channel_info.name.clone(),
                    ));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_user() {
        let mut users_widget = UsersWidget::new();
        let user_name = "farine";
        users_widget.add_user_with_channel("#spam", user_name);
        assert_eq!(users_widget.list_sections.len(), 1);
        assert!(users_widget.list_users.get(user_name).is_some());
        let user = users_widget.list_users.get(user_name).unwrap();
        assert_eq!(user.name, user_name.to_string());
        assert_eq!(user.color, nickname_color(user_name));
        assert_eq!(users_widget.list_users.len(), 1);
    }

    #[test]
    fn test_add_user_multiple_channel() {
        let mut users_widget = UsersWidget::new();
        let user_name = "farine";
        users_widget.add_user_with_channel("#spam", user_name);
        users_widget.add_user_with_channel("#spam_2", user_name);
        assert_eq!(users_widget.list_sections.len(), 2);
        assert_eq!(users_widget.list_users.len(), 1);
        assert_eq!(
            users_widget.list_sections.get(0).unwrap().order_user.len(),
            1
        );
        assert_eq!(
            users_widget.list_sections.get(1).unwrap().order_user.len(),
            1
        );

        users_widget.add_user_with_channel("#spam_2", "@farine");
        assert_eq!(users_widget.list_users.len(), 1);
    }

    #[test]
    fn test_add_gloabl_channel() {
        let mut users_widget = UsersWidget::new();
        let user_name = "IRC-Server";
        users_widget.add_user_with_channel("", user_name);
        assert_eq!(users_widget.list_sections.len(), 1);
        assert_eq!(
            users_widget.list_sections.get(0).unwrap().order_user.len(),
            1
        );

        assert_eq!(users_widget.list_users.len(), 1);
    }

    #[test]
    fn test_number_sections() {
        let mut users_widget = UsersWidget::new();
        let user_name = "IRC-Server";
        users_widget.add_user_with_channel(user_name, user_name);
        assert_eq!(users_widget.nb_sections(), 1);

        users_widget.add_user_with_channel("#new-chan", user_name);
        assert_eq!(users_widget.nb_sections(), 2);

        assert_eq!(users_widget.nb_items(0), 2);
        assert_eq!(users_widget.nb_items(1), 2);
    }
}
