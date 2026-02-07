use ahash::AHashMap;

use crate::message_event::MessageEvent;

#[derive(Debug, PartialEq, Clone)]
pub struct User {
    name: String,
    connected_sections: bit_vec::BitVec,
    is_main: bool,
    has_unread_message: bool,
    has_received_message: bool,
}
const NB_SECTIONS: usize = 32;

impl User {
    pub fn new(name: String, is_main: bool) -> Self {
        Self {
            name,
            is_main,
            connected_sections: bit_vec::BitVec::from_elem(NB_SECTIONS, false),
            has_unread_message: false,
            has_received_message: false,
        }
    }

    pub fn has_unread_message(&self) -> bool {
        self.has_unread_message
    }

    pub fn join_section(&mut self, id: usize) {
        if let Some(mut channel) = self.connected_sections.get_mut(id) {
            *channel = true;
        }
    }

    pub fn has_joined_section(&self, id: usize) -> bool {
        self.connected_sections.get(id).unwrap_or(false)
    }

    pub fn quit_section(&mut self, id: usize) {
        if let Some(mut channel) = self.connected_sections.get_mut(id) {
            *channel = false;
        }
    }

    pub fn has_joined_any_section(&mut self) -> bool {
        self.connected_sections.any()
    }
}

#[derive(Debug)]
struct Channel {
    id: usize,
    name: String,
}

pub struct IrcModel {
    users: ahash::AHashMap<String, User>,
    list_channels: Vec<Channel>,
    current_channel: String,
}

impl IrcModel {
    pub fn new_model(nick_name: String, current_channel: String) -> Self {
        let mut users = AHashMap::new();
        users.insert(
            Self::sanitize_name(&nick_name).to_string(),
            User::new(nick_name.to_string(), true),
        );
        Self {
            users,
            list_channels: Vec::new(),
            current_channel: current_channel.to_ascii_lowercase(),
        }
    }

    fn sanitize_name(user: &str) -> String {
        user.strip_prefix('@').unwrap_or(user).to_lowercase()
    }

    fn add_channel(&mut self, channel: &str) -> usize {
        for c in &self.list_channels {
            if c.name.eq_ignore_ascii_case(channel) {
                return c.id;
            }
        }
        let new_channel = Channel {
            id: self.list_channels.len(),
            name: channel.to_string(),
        };
        let id = new_channel.id;
        self.list_channels.push(new_channel);
        id
    }

    fn join(&mut self, channel: &str, nick: &str) {
        let nick = Self::sanitize_name(nick).to_string();
        let id = self.add_channel(channel);

        let entry = self
            .users
            .entry(nick.to_string())
            .or_insert_with(|| User::new(nick.to_string(), false));
        entry.join_section(id);
    }

    fn part(&mut self, channel: &str, nick: &str) {
        let nick = Self::sanitize_name(nick);
        let id = self.add_channel(channel);

        let mut should_delete = false;
        if let Some(user) = self.users.get_mut(&nick) {
            user.quit_section(id);
            should_delete = user.has_joined_any_section();
        }

        if should_delete {
            self.users.remove(&nick);
        }
    }

    fn quit(&mut self, nick: &str) {
        let nick = Self::sanitize_name(nick);

        self.users.remove(&nick);
    }

    fn nick(&mut self, old: &str, new: &str) {
        let old = Self::sanitize_name(old);

        if let Some(user) = self.users.get(&old) {
            let mut old_user = (*user).clone();
            old_user.name = new.to_string();
            self.users.insert(Self::sanitize_name(new), old_user);
        }
    }

    pub fn is_main_user(&self, user: &str) -> bool {
        if let Some(user) = self.users.get(Self::sanitize_name(user).as_str()) {
            user.is_main
        } else {
            false
        }
    }

    pub fn get_user(&self, user: &str) -> Option<&User> {
        self.users.get(&Self::sanitize_name(user))
    }

    pub fn get_all_joined_channel(&self, user: &str) -> impl Iterator<Item = String> + '_ {
        self.users.get(user).into_iter().flat_map(move |u| {
            self.list_channels
                .iter()
                .filter(move |section| u.has_joined_section(section.id))
                .map(|section| section.name.clone())
        })
    }

    fn received_message(&mut self, target: &str) {
        let target = Self::sanitize_name(target);
        if let Some(user) = self.users.get_mut(&target) {
            user.has_unread_message = self.current_channel.eq(&target);
            user.has_received_message = true;
        }
    }

    fn select_channel(&mut self, channel: &str) {
        self.current_channel = Self::sanitize_name(channel);
        if let Some(user) = self.users.get_mut(&self.current_channel) {
            user.has_unread_message = false;
        }
    }

    pub fn handle_action(&mut self, event: &MessageEvent) {
        match event {
            MessageEvent::JoinServer(server) => {
                self.add_channel(server);
            }
            MessageEvent::Join(channel, user) => {
                self.join(channel, user);
            }
            MessageEvent::Part(channel, user) => {
                self.part(channel, user);
            }
            MessageEvent::Quit(user, _) => {
                self.quit(user);
            }
            MessageEvent::ReplaceUser(old, new) => {
                self.nick(old, new);
            }
            MessageEvent::SelectChannel(channel) => {
                self.select_channel(channel);
            }
            MessageEvent::PrivMsg(_, target, _) | MessageEvent::ActionMsg(_, target, _) => {
                self.received_message(target);
            }
            _ => {}
        }
    }
}
