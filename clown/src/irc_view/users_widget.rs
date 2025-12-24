use crate::{
    component::Draw,
    irc_view::{color_user::nickname_color, users_widget},
};
use ahash::AHashMap;
use bit_vec::BitVec;
use crossterm::event::KeyModifiers;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
};

#[derive(Debug, PartialEq)]
enum ChannelMode {
    User,
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

    pub fn has_joined_channel(&self, id: usize) -> bool {
        self.connected_channels.get(id).unwrap_or_default()
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
}

impl RegisteredChannel {
    pub fn new(name: String, id: usize) -> Self {
        Self {
            color: nickname_color(&name),
            name,
            id,
        }
    }
}

trait DrawableItem {
    fn display_title(&self) -> &str;
    fn display_color(&self) -> ratatui::style::Color;
    fn is_highlighted(&self) -> bool;
}

#[derive(Debug)]
struct Channel {
    pub channel_info: RegisteredChannel,
    pub order_user: Vec<String>,
}

impl Channel {
    fn new(channel_name: String, id: usize) -> Self {
        Self {
            channel_info: RegisteredChannel::new(channel_name, id),
            order_user: Vec::new(),
        }
    }

    fn get_channel_id(&self) -> usize {
        self.channel_info.id
    }

    fn set_user_position(&mut self, user: &str) {
        if let Some(id) = self.order_user.iter().position(|v| v.eq(&user)) {
            self.order_user.remove(id);
            self.order_user.push(user.to_string());
        } else {
            self.order_user.push(user.to_string());
        }
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
// list users contain to which channel a user belong. The ID is based on the index of the list_channels
pub struct UsersWidget {
    list_channels: Vec<Channel>,
    list_users: ahash::AHashMap<String, User>,

    list_state: ListStateWidget,
    area: Rect,
}

impl UsersWidget {
    pub fn new() -> Self {
        let mut c = Vec::<Channel>::new();
        c.push(Channel::new("glob".to_string(), 0));
        Self {
            list_users: AHashMap::new(),
            area: Rect::default(),
            list_channels: c,
            list_state: ListStateWidget::default(),
        }
    }

    fn get_channel_id(&self, channel: &str) -> Option<usize> {
        self.list_channels
            .iter()
            .find(|c| c.channel_info.name == channel)
            .map(|c| c.channel_info.id)
    }

    fn add_channel(&mut self, channel: String) -> Option<usize> {
        let index = if let Some(i) = self
            .list_channels
            .iter()
            .position(|c| c.channel_info.name == channel)
        {
            i
        } else {
            let i = self.list_channels.len();
            self.list_channels
                .push(Channel::new(channel.to_string(), i));
            i
        };
        self.list_channels.get(index).map(|c| c.channel_info.id)
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
        let old = self.sanitize_name(old);
        let new = self.sanitize_name(new);

        let v = self.list_users.remove(&old);

        if let Some(mut v) = v {
            let n = new;
            v.set_name(n.to_string());
            self.list_users.insert(n, v);
        }
    }

    fn remove_user(&mut self, channel_id: Option<usize>, user: &str) {
        let user = self.sanitize_name(user);

        if let Some(id) = channel_id {
            if let Some(u) = self.list_users.get_mut(&user) {
                if let Some(channel) = self.list_channels.get_mut(id) {
                    channel.remove_user(&user);
                }

                u.quit_channel(id);
            }
        } else {
            self.list_users.remove(&user);
        }
    }

    fn hightlight_user(&mut self, user: &str) {
        if let Some(user) = self.list_users.get_mut(user) {
            user.need_hightlight = true;
        }
    }

    fn sanitize_name(&self, user: &str) -> String {
        user.replace("@", "")
    }

    fn add_user(&mut self, channel_id: usize, user: &str, mode: ChannelMode) {
        let user = self.sanitize_name(user);
        if mode == ChannelMode::User {
            if let Some(user) = self.list_users.get_mut(&user) {
                if let Some(channel) = self.list_channels.get_mut(channel_id) {
                    channel.set_user_position(&user.name);
                }
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
            .render(&self.list_channels, &self.list_users, frame, area);
    }
}
//#channel
// user1
// user2
//#channel2
// user3
// user2
#[derive(Default, Clone, Debug)]
pub struct ListStateWidget {
    current_channel: String,
    current_selected: usize,
    current_max: usize,
}

impl ListStateWidget {
    fn selected(&self) -> (&str, usize) {
        (&self.current_channel, self.current_selected)
    }

    fn next(&mut self) {
        self.current_selected = self.current_selected.saturating_add(1) % self.current_max;
    }

    fn previous(&mut self) {
        self.current_selected = self.current_selected.saturating_sub(1) % self.current_max;
    }

    fn add_item<'a>(
        &'a self,
        color: ratatui::style::Color,
        title: &'a str,
        is_highlighted: bool,
    ) -> Vec<ratatui::text::Span<'a>> {
        let mut spans = Vec::new();
        let mut style = Style::default().fg(color);

        if title.eq(&self.current_channel) {
            style = style.add_modifier(Modifier::BOLD)
        }

        if is_highlighted {
            style = style.bg(Color::LightBlue);
        }
        spans.push(Span::raw(" "));
        spans.push(Span::styled(title, style));
        spans
        //ListItem::from(Line::from(spans))
    }

    fn render(
        &mut self,
        channels: &[Channel],
        users: &AHashMap<String, User>,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        let mut items = Vec::new();

        for channel in channels {
            let spans = self.add_item(
                channel.channel_info.color,
                &channel.channel_info.name,
                false,
            );

            let item = ListItem::from(Line::from(spans));
            items.push(item);

            for (i, user_name) in channel.order_user.iter().enumerate() {
                if let Some(user) = users.get(user_name) {
                    if user.has_joined_channel(channel.channel_info.id) {
                        let spans = self.add_item(
                            user.display_color(),
                            user.display_title(),
                            user.is_highlighted(),
                        );

                        let item = ListItem::from(Line::from(spans));
                        items.push(item);
                    }
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
        tracing::debug!("Handle actions {:?}", event);
        match event {
            MessageEvent::UpdateUsers(channel, list_users) => {
                self.set_users(channel, list_users.to_vec());
                tracing::debug!("Handle channels {:?}", self.list_channels);
                tracing::debug!("Handle users {:?}", self.list_users);

                None
            }
            MessageEvent::ReplaceUser(old, new) => {
                self.replace_user(old, new);
                None
            }
            MessageEvent::RemoveUser(channel, user) => {
                self.remove_user(channel.as_ref().and_then(|c| self.get_channel_id(c)), user);
                None
            }
            MessageEvent::HighlightUser(user) => {
                self.hightlight_user(user);

                None
            }
            MessageEvent::JoinUser(channel, user) => {
                if let Some(channel_id) = self.get_channel_id(channel) {
                    self.add_user(channel_id, user, ChannelMode::User);
                } else if let Some(channel_id) = self.add_channel(channel.to_string()) {
                    self.add_user(channel_id, user, ChannelMode::Channel);
                }
                tracing::debug!("Handle channels {:?}", self.list_channels);
                tracing::debug!("Handle users {:?}", self.list_users);
                tracing::debug!("Handle order {:?}", self.order);
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
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    crossterm::event::KeyCode::Char('p') => {
                        self.list_state.previous();
                    }
                    crossterm::event::KeyCode::Char('n') => {
                        self.list_state.next();
                    }
                    _ => {}
                }
            }
            let (selected, id) = self.list_state.selected();
            if let Some(user) = self.list_users.get_mut(selected)
                && let (previous_str, previoud_id) = previous_selected
                && previous_str != selected
                && previoud_id != id
            {
                user.need_hightlight = false;
                return Some(MessageEvent::SelectChannel(user.name.to_string()));
            }
        }
        None
    }
}
