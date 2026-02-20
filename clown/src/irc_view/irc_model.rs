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
    pub servers: Vec<Option<IrcServerModel>>,
    pub current_id: Option<usize>,
}

impl IrcModel {
    pub fn new(in_length: usize) -> Self {
        Self {
            servers: std::iter::repeat_with(|| None).take(in_length).collect(),
            current_id: None,
        }
    }

    #[cfg(test)]
    pub fn new_single_server(in_length: usize, id: usize, nickname: String) -> Self {
        let mut s = Self {
            servers: std::iter::repeat_with(|| None).take(in_length).collect(),
            current_id: None,
        };

        s.init_server(id, nickname);

        s
    }

    pub fn init_server(&mut self, in_id: usize, nickname: String) {
        if let Some(server) = self.servers.get_mut(in_id) {
            *server = Some(IrcServerModel::new_model(in_id, nickname))
        }
    }

    pub fn get_current_server(&self) -> Option<&IrcServerModel> {
        self.current_id
            .and_then(|id| self.servers.get(id))
            .and_then(|v| v.as_ref())
    }

    pub fn get_server(&self, server_id: usize) -> Option<&IrcServerModel> {
        if let Some(server) = self.servers.get(server_id) {
            server.as_ref()
        } else {
            None
        }
    }

    pub fn is_main_user(&self, server_id: usize, nick: &str) -> bool {
        if let Some(Some(server)) = self.servers.get(server_id) {
            server.is_main_user(nick)
        } else {
            false
        }
    }

    pub fn clear_server(&mut self, in_id: usize) {
        if let Some(id) = self.current_id
            && id == in_id
        {
            self.current_id = None;
        }
        if let Some(server) = self.servers.get_mut(in_id) {
            *server = None;
        }
    }

    pub fn get_all_joined_channel(
        &self,
        server_id: usize,
        user: &str,
    ) -> impl Iterator<Item = &str> + '_ {
        self.servers
            .get(server_id)
            .and_then(|s| s.as_ref())
            .map(|server| server.get_all_joined_channel(user))
            .into_iter()
            .flatten()
    }

    pub fn handle_action(&mut self, event: &MessageEvent) {
        match event {
            MessageEvent::JoinServer(server_id, server_name) => {
                if let Some(Some(server)) = self.servers.get_mut(*server_id) {
                    server.add_channel(server_name);
                }
            }
            MessageEvent::Join(server_id, channel, user) => {
                if let Some(Some(server)) = self.servers.get_mut(*server_id) {
                    server.join(channel, user);
                }
            }
            MessageEvent::Part(server_id, channel, user) => {
                if let Some(Some(server)) = self.servers.get_mut(*server_id) {
                    server.part(channel, user);
                }
            }
            MessageEvent::Quit(server_id, user, _) => {
                if let Some(Some(server)) = self.servers.get_mut(*server_id) {
                    server.quit(user);
                }
            }
            MessageEvent::ReplaceUser(server_id, old, new) => {
                if let Some(Some(server)) = self.servers.get_mut(*server_id) {
                    server.nick(old, new);
                }
            }
            MessageEvent::UpdateUsers(server_id, channel, list_users) => {
                if let Some(Some(server)) = self.servers.get_mut(*server_id) {
                    for user in list_users {
                        server.join(channel, user);
                    }
                }
            }
            MessageEvent::SelectChannel(server_id, channel) => {
                if let Some(server_id) = server_id
                    && let Some(Some(server)) = self.servers.get_mut(*server_id)
                {
                    server.select_channel(channel);
                }
                self.current_id = *server_id;
            }
            MessageEvent::PrivMsg(server_id, source, target, _)
            | MessageEvent::ActionMsg(server_id, source, target, _) => {
                if let Some(Some(server)) = self.servers.get_mut(*server_id) {
                    server.received_message(source, target);
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct IrcServerModel {
    users: ahash::AHashMap<String, User>,

    //Usually not a lot of channels, so keeping a vector is fine
    list_channels: Vec<Channel>,
    current_channel: Option<String>,

    current_nick: String,
    server_id: usize,
}

impl IrcServerModel {
    pub fn new_model(server_id: usize, nick_name: String) -> Self {
        let mut users = AHashMap::new();
        users.insert(
            Self::sanitize_name(&nick_name).to_lowercase(),
            User::new(Self::sanitize_name(&nick_name).to_string(), true),
        );
        Self {
            users,
            server_id,
            list_channels: Vec::new(),
            current_channel: None,
            current_nick: Self::sanitize_name(&nick_name).to_string(),
        }
    }

    pub fn get_current_channel(&self) -> Option<&str> {
        self.current_channel.as_deref()
    }

    pub fn get_current_nick(&self) -> &str {
        &self.current_nick
    }

    pub fn get_server_id(&self) -> usize {
        self.server_id
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

    pub fn add_channel(&mut self, channel: &str) -> usize {
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

    fn rename_channel(&mut self, old: &str, new: &str) {
        if let Some(c) = self.get_channel_mut(old) {
            c.name = new.to_string();
        }
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
            self.current_channel = Some(channel.to_string());
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
        let old = Self::sanitize_name(old);
        let new = Self::sanitize_name(new);
        let old_lower = old.to_lowercase();

        if let Some(user) = self.users.remove(&old_lower) {
            self.rename_channel(old, new);

            let mut old_user = user.clone();
            old_user.name = new.to_string();
            if old_user.is_main {
                self.current_nick = new.to_string()
            }
            if let Some(channel) = self.current_channel.as_mut()
                && (*channel).eq(&old_lower)
            {
                *channel = new.to_string();
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

    pub fn get_all_joined_channel(&self, user: &str) -> impl Iterator<Item = &str> + '_ {
        let user = Self::sanitize_name(user).to_lowercase();
        let maybe_user = self.users.get(&user);

        maybe_user.into_iter().flat_map(|u| {
            self.list_channels
                .iter()
                .filter(|section| u.has_joined_section(section.id))
                .map(|section| section.name.as_str())
        })
    }

    // a(source) sends to b(target)
    fn received_message(&mut self, source: &str, target: &str) {
        let target = Self::sanitize_name(target);
        let source = Self::sanitize_name(source);

        let target = self.get_target(source, target);
        let id = self.add_channel(target);

        let target = target.to_lowercase();

        let new_message = self
            .current_channel
            .as_ref()
            .is_some_and(|v| v.eq_ignore_ascii_case(&target));

        if let Some(user) = self.get_user(&target)
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
        self.current_channel = Some(Self::sanitize_name(channel).to_string());
        if let Some(c) = self.get_channel_mut(&channel.to_lowercase()) {
            c.has_unread_message = false;
        }
    }

    pub fn has_unread_message(&self, channel: &str) -> bool {
        self.get_channel(channel)
            .is_some_and(|v| v.has_unread_message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> IrcModel {
        // create one usable server slot
        IrcModel {
            servers: vec![None],
            current_id: None,
        }
    }

    fn setup_server(m: &mut IrcModel) {
        m.init_server(0, "Me".into());
        m.current_id = Some(0);
    }

    fn server(m: &IrcModel) -> &IrcServerModel {
        m.get_current_server().unwrap()
    }

    #[test]
    fn test_main_user() {
        let mut m = model();
        setup_server(&mut m);

        let s = server(&m);

        assert!(s.is_main_user("me"));
        assert!(s.is_main_user("ME"));
        assert!(s.is_main_user("Me"));
    }

    #[test]
    fn test_join_channel() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(0, "#Rust".into(), "Alice".into()));

        let s = server(&m);

        assert!(s.has_user_joined_channel("alice", "#rust"));
        assert!(s.has_user_joined_channel("ALICE", "#RUST"));
    }

    #[test]
    fn test_part_channel() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(0, "#Rust".into(), "Alice".into()));
        assert!(server(&m).has_user_joined_channel("alice", "#rust"));

        m.handle_action(&MessageEvent::Part(0, "#RUST".into(), "ALICE".into()));

        assert!(!server(&m).has_user_joined_channel("alice", "#rust"));
    }

    #[test]
    fn test_quit_user() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(0, "#rust".into(), "Alice".into()));
        assert!(server(&m).get_user("alice").is_some());

        m.handle_action(&MessageEvent::Quit(0, "ALICE".into(), Some("bye".into())));

        assert!(server(&m).get_user("alice").is_none());
    }

    #[test]
    fn test_nick_change() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(0, "#rust".into(), "Alice".into()));
        m.handle_action(&MessageEvent::Join(0, "#rust".into(), "Jack".into()));

        // Create private channel with Jack
        m.handle_action(&MessageEvent::PrivMsg(
            0,
            "jack".into(),
            "me".into(),
            "hello".into(),
        ));

        assert!(server(&m).get_channel_id("jack").is_some());

        // Rename Alice → Bob
        m.handle_action(&MessageEvent::ReplaceUser(0, "ALICE".into(), "BoB".into()));

        assert!(server(&m).get_user("alice").is_none());
        assert!(server(&m).get_user("bob").is_some());

        // Rename Jack → Miki
        m.handle_action(&MessageEvent::ReplaceUser(0, "Jack".into(), "miki".into()));

        let s = server(&m);

        assert!(s.get_user("jack").is_none());
        assert!(s.get_user("miki").is_some());
        assert!(s.get_channel_id("jack").is_none());
        assert!(s.get_channel_id("miki").is_some());
    }

    #[test]
    fn test_select_channel_clears_unread() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(0, "#Rust".into(), "Alice".into()));

        m.handle_action(&MessageEvent::PrivMsg(
            0,
            "alice".into(),
            "#RUST".into(),
            "hello".into(),
        ));

        assert!(server(&m).has_unread_message("#rust"));

        m.handle_action(&MessageEvent::SelectChannel(Some(0), "#RUST".into()));

        assert!(!server(&m).has_unread_message("#rust"));
    }

    #[test]
    fn test_get_all_joined_channels() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(0, "#Rust".into(), "Alice".into()));
        m.handle_action(&MessageEvent::Join(0, "#Linux".into(), "ALICE".into()));

        let channels: Vec<_> = server(&m).get_all_joined_channel("alice").collect();

        assert_eq!(channels.len(), 2);
        assert!(channels.iter().any(|c| c.eq_ignore_ascii_case("#rust")));
        assert!(channels.iter().any(|c| c.eq_ignore_ascii_case("#linux")));
    }

    #[test]
    fn test_private_message_unread() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(0, "#Rust".into(), "Alice".into()));
        m.handle_action(&MessageEvent::SelectChannel(Some(0), "#General".into()));

        m.handle_action(&MessageEvent::PrivMsg(
            0,
            "ALICE".into(),
            "#rust".into(),
            "hello".into(),
        ));

        assert!(server(&m).has_unread_message("#RUST"));
    }

    #[test]
    fn test_get_target() {
        let mut m = model();
        setup_server(&mut m);

        let s = server(&m);

        // target is main nick -> return source
        let t = s.get_target("Alice", "ME");
        assert_eq!(t, "Alice");

        // target is channel -> return channel
        let t = s.get_target("Alice", "#RUST");
        assert_eq!(t, "#RUST");
    }
}
