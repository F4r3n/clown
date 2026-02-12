use ahash::AHashMap;

use crate::message_event::MessageEvent;

#[derive(Debug, PartialEq, Clone)]
pub struct User {
    name: String,
    connected_sections: bit_vec::BitVec,
    is_main: bool,
}
const NB_SECTIONS: usize = 32;

impl User {
    pub fn new(name: String, is_main: bool) -> Self {
        Self {
            name,
            is_main,
            connected_sections: bit_vec::BitVec::from_elem(NB_SECTIONS, false),
        }
    }

    #[cfg(test)]
    pub fn get_name(&self) -> &str {
        &self.name
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

    pub fn has_joined_any_section(&self) -> bool {
        self.connected_sections.any()
    }
}

#[derive(Debug)]
struct Channel {
    id: usize,
    name: String,

    has_unread_message: bool,
    has_received_message: bool,
}

#[derive(Debug)]
pub struct IrcModel {
    users: ahash::AHashMap<String, User>,

    //Usually not a lot of channels, so keeping a vector is fine
    list_channels: Vec<Channel>,
    current_channel: String,

    current_nick: String,
}

impl IrcModel {
    pub fn new_model(nick_name: String, current_channel: String) -> Self {
        let mut users = AHashMap::new();
        users.insert(
            Self::sanitize_name(&nick_name).to_lowercase(),
            User::new(Self::sanitize_name(&nick_name).to_string(), true),
        );
        Self {
            users,
            list_channels: Vec::new(),
            current_channel: current_channel.to_lowercase(),
            current_nick: Self::sanitize_name(&nick_name).to_string(),
        }
    }

    pub fn get_current_channel(&self) -> &str {
        &self.current_channel
    }

    pub fn get_current_nick(&self) -> &str {
        &self.current_nick
    }

    pub fn get_target<'b>(&self, source: &'b str, target: &'b str) -> &'b str {
        if target.eq_ignore_ascii_case(self.get_current_nick()) {
            source
        } else {
            target
        }
    }

    fn sanitize_name(user: &str) -> &str {
        user.strip_prefix('@').unwrap_or(user)
    }

    fn get_channel_id(&self, channel: &str) -> Option<usize> {
        for c in &self.list_channels {
            if c.name.eq_ignore_ascii_case(channel) {
                return Some(c.id);
            }
        }

        None
    }

    fn get_channel_mut(&mut self, channel: &str) -> Option<&mut Channel> {
        self.get_channel_id(channel)
            .and_then(|v| self.list_channels.get_mut(v))
    }

    fn get_channel(&self, channel: &str) -> Option<&Channel> {
        self.get_channel_id(channel)
            .and_then(|v| self.list_channels.get(v))
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
            has_received_message: false,
            has_unread_message: false,
        };
        let id = new_channel.id;
        self.list_channels.push(new_channel);
        id
    }

    fn join(&mut self, channel: &str, nick: &str) {
        let nick = Self::sanitize_name(nick);
        let id = self.add_channel(channel);

        let entry = self
            .users
            .entry(nick.to_lowercase())
            .or_insert_with(|| User::new(nick.to_string(), false));
        entry.join_section(id);
        if entry.is_main {
            self.current_channel = channel.to_string();
        }
    }

    fn part(&mut self, channel: &str, nick: &str) {
        let nick = Self::sanitize_name(nick).to_lowercase();
        let id = self.add_channel(channel);

        let mut should_delete = false;
        if let Some(user) = self.users.get_mut(&nick) {
            user.quit_section(id);
            should_delete = !user.has_joined_any_section();
        }

        if should_delete {
            self.users.remove(&nick);
        }
    }

    fn quit(&mut self, nick: &str) {
        let nick = Self::sanitize_name(nick).to_lowercase();

        self.users.remove(&nick);
    }

    fn nick(&mut self, old: &str, new: &str) {
        let old = Self::sanitize_name(old).to_lowercase();

        if let Some(user) = self.users.remove(&old) {
            let mut old_user = user.clone();
            old_user.name = new.to_string();
            if old_user.is_main {
                self.current_nick = new.to_string()
            }
            let new = Self::sanitize_name(new);
            if self.current_channel.eq(&old) {
                self.current_channel = new.to_string();
            }

            self.users.insert(new.to_lowercase(), old_user);
        }
    }

    pub fn is_main_user(&self, user: &str) -> bool {
        if let Some(user) = self.users.get(&Self::sanitize_name(user).to_lowercase()) {
            user.is_main
        } else {
            false
        }
    }

    pub fn get_user(&self, user: &str) -> Option<&User> {
        self.users.get(&Self::sanitize_name(user).to_lowercase())
    }

    pub fn has_user_joined_channel(&self, user: &str, channel: &str) -> bool {
        if let Some(id) = self.get_channel_id(channel) {
            self.users
                .get(&Self::sanitize_name(user).to_lowercase())
                .map(|v| v.has_joined_section(id))
                .is_some_and(|v| v)
        } else {
            false
        }
    }

    //TODO: iterator should need to send &str
    pub fn get_all_joined_channel(&self, user: &str) -> impl Iterator<Item = String> + '_ {
        let user = Self::sanitize_name(user).to_lowercase();
        self.users.get(&user).into_iter().flat_map(move |u| {
            self.list_channels
                .iter()
                .filter(move |section| u.has_joined_section(section.id))
                .map(|section| section.name.clone())
        })
    }

    // a(source) sends to b(target)
    fn received_message(&mut self, source: &str, target: &str) {
        let target = self.get_target(source, target);
        let id = self.add_channel(target);

        let target = &Self::sanitize_name(target).to_lowercase();

        let new_message = self.current_channel.eq_ignore_ascii_case(target);

        if let Some(user) = self.get_user(target)
            && user.is_main
        {
            return;
        }
        if let Some(c) = self.list_channels.get_mut(id) {
            c.has_received_message = true;
            c.has_unread_message = !new_message;
        }
    }

    fn select_channel(&mut self, channel: &str) {
        self.current_channel = Self::sanitize_name(channel).to_string();
        if let Some(c) = self.get_channel_mut(&channel.to_lowercase()) {
            c.has_unread_message = false;
        }
    }

    pub fn has_unread_message(&self, channel: &str) -> bool {
        self.get_channel(channel)
            .is_some_and(|v| v.has_unread_message)
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
            MessageEvent::UpdateUsers(channel, list_users) => {
                for user in list_users {
                    self.join(channel, user);
                }
            }
            MessageEvent::SelectChannel(channel) => {
                self.select_channel(channel);
            }
            MessageEvent::PrivMsg(source, target, _)
            | MessageEvent::ActionMsg(source, target, _) => {
                self.received_message(source, target);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> IrcModel {
        // mixed casing on purpose
        IrcModel::new_model("Me".into(), "#General".into())
    }

    #[test]
    fn test_main_user() {
        let model = model();

        assert!(model.is_main_user("me"));
        assert!(model.is_main_user("ME"));
        assert!(model.is_main_user("Me"));
    }

    #[test]
    fn test_join_channel() {
        let mut model = model();

        model.handle_action(&MessageEvent::Join("#Rust".into(), "Alice".into()));

        assert!(model.has_user_joined_channel("alice", "#rust"));
        assert!(model.has_user_joined_channel("ALICE", "#RUST"));
    }

    #[test]
    fn test_part_channel() {
        let mut model = model();

        model.handle_action(&MessageEvent::Join("#Rust".into(), "Alice".into()));
        assert!(model.has_user_joined_channel("alice", "#rust"));

        model.handle_action(&MessageEvent::Part("#RUST".into(), "ALICE".into()));

        assert!(!model.has_user_joined_channel("alice", "#rust"));
    }

    #[test]
    fn test_quit_user() {
        let mut model = model();

        model.handle_action(&MessageEvent::Join("#rust".into(), "Alice".into()));
        assert!(model.get_user("alice").is_some());

        model.handle_action(&MessageEvent::Quit("ALICE".into(), Some("bye".into())));

        assert!(model.get_user("alice").is_none());
    }

    #[test]
    fn test_nick_change() {
        let mut model = model();

        model.handle_action(&MessageEvent::Join("#rust".into(), "Alice".into()));
        model.handle_action(&MessageEvent::ReplaceUser("ALICE".into(), "BoB".into()));

        assert!(model.get_user("alice").is_none());
        assert!(model.get_user("bob").is_some());
    }

    #[test]
    fn test_select_channel_clears_unread() {
        let mut model = model();

        model.handle_action(&MessageEvent::Join("#Rust".into(), "Alice".into()));

        model.handle_action(&MessageEvent::PrivMsg(
            "alice".into(),
            "#RUST".into(),
            "hello".into(),
        ));

        assert!(model.has_unread_message("#rust"));

        model.handle_action(&MessageEvent::SelectChannel("#RUST".into()));

        assert!(!model.has_unread_message("#rust"));
    }

    #[test]
    fn test_get_all_joined_channels() {
        let mut model = model();

        model.handle_action(&MessageEvent::Join("#Rust".into(), "Alice".into()));
        model.handle_action(&MessageEvent::Join("#Linux".into(), "ALICE".into()));

        let channels: Vec<_> = model.get_all_joined_channel("alice").collect();

        assert_eq!(channels.len(), 2);
        assert!(channels.iter().any(|c| c.eq_ignore_ascii_case("#rust")));
        assert!(channels.iter().any(|c| c.eq_ignore_ascii_case("#linux")));
    }

    #[test]
    fn test_private_message_unread() {
        let mut model = model();

        model.handle_action(&MessageEvent::Join("#Rust".into(), "Alice".into()));
        model.handle_action(&MessageEvent::SelectChannel("#General".into()));

        model.handle_action(&MessageEvent::PrivMsg(
            "ALICE".into(),
            "#rust".into(),
            "hello".into(),
        ));

        assert!(model.has_unread_message("#RUST"));
    }

    #[test]
    fn test_get_target() {
        let model = model();

        // target is main nick -> should return source
        let t = model.get_target("Alice", "ME");
        assert_eq!(t, "Alice");

        // target is channel -> should return channel
        let t = model.get_target("Alice", "#RUST");
        assert_eq!(t, "#RUST");
    }
}
