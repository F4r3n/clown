use super::irc_name::IrcName;
use crate::message_event::MessageEvent;
use crate::state::server_id::ServerID;
use ahash::AHashMap;

#[derive(Debug, PartialEq, Clone)]
pub struct User {
    connected_sections: bit_vec::BitVec,
    is_main: bool,
}
const NB_SECTIONS: usize = 32;
impl User {
    pub fn new(is_main: bool) -> Self {
        Self {
            is_main,
            connected_sections: bit_vec::BitVec::from_elem(NB_SECTIONS, false),
        }
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
    name: IrcName,

    has_unread_message: bool,
    has_received_message: bool,
}

#[derive(Debug)]
pub struct IrcModel {
    pub servers: Vec<Option<IrcServerModel>>,
    pub current_id: Option<ServerID>,
}

impl IrcModel {
    pub fn new(in_length: usize) -> Self {
        Self {
            servers: std::iter::repeat_with(|| None).take(in_length).collect(),
            current_id: None,
        }
    }

    pub fn init_server(&mut self, in_id: ServerID, server_name: String, nickname: String) {
        if let Some(server) = self.get_mut_server_from_id(in_id) {
            *server = Some(IrcServerModel::new_model(in_id, server_name, nickname))
        }
    }

    pub fn get_current_server(&self) -> Option<&IrcServerModel> {
        self.current_id
            .and_then(|id| self.get_server_from_id(id))
            .and_then(|v| v.as_ref())
    }

    pub fn get_server_name(&self, server_id: ServerID) -> &str {
        if let Some(Some(server)) = self.get_server_from_id(server_id) {
            server.get_server_name()
        } else {
            "Server"
        }
    }

    fn get_server_from_id(&self, server_id: ServerID) -> Option<&Option<IrcServerModel>> {
        self.servers.get(server_id.as_usize())
    }

    fn get_mut_server_from_id(
        &mut self,
        server_id: ServerID,
    ) -> Option<&mut Option<IrcServerModel>> {
        self.servers.get_mut(server_id.as_usize())
    }

    pub fn get_server_name_from_channel<'a>(
        &'a self,
        server_id: ServerID,
        channel: Option<&'a str>,
    ) -> Option<&'a str> {
        if channel.is_none() {
            Some(self.get_server_name(server_id))
        } else {
            channel
        }
    }

    pub fn get_server(&self, server_id: ServerID) -> Option<&IrcServerModel> {
        if let Some(server) = self.get_server_from_id(server_id) {
            server.as_ref()
        } else {
            None
        }
    }

    pub fn is_main_user(&self, server_id: ServerID, nick: &IrcName) -> bool {
        if let Some(Some(server)) = self.get_server_from_id(server_id) {
            server.is_main_user(nick)
        } else {
            false
        }
    }

    pub fn clear_server(&mut self, in_id: ServerID) {
        if let Some(id) = self.current_id
            && id == in_id
        {
            self.current_id = None;
        }
        if let Some(server) = self.get_mut_server_from_id(in_id) {
            *server = None;
        }
    }

    pub fn get_all_joined_channel(
        &self,
        server_id: ServerID,
        user: &IrcName,
    ) -> impl Iterator<Item = &IrcName> + '_ {
        self.get_server_from_id(server_id)
            .and_then(|s| s.as_ref())
            .map(|server| server.get_all_joined_channel(user))
            .into_iter()
            .flatten()
    }

    pub fn handle_action(&mut self, event: &MessageEvent) {
        match event {
            MessageEvent::JoinServer(server_id) => {
                let server_name = self.get_server_name(*server_id).to_string();

                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    let server_name = IrcName::new(&server_name);
                    server.add_channel(&server_name);
                }
            }
            MessageEvent::Join(server_id, channel, user) => {
                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    server.join(channel, user);
                }
            }
            MessageEvent::Part(server_id, channel, user) => {
                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    server.part(channel, user);
                }
            }
            MessageEvent::Quit(server_id, user, _) => {
                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    server.quit(user);
                }
            }
            MessageEvent::ReplaceUser(server_id, old, new) => {
                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    server.nick(old, new);
                }
            }
            MessageEvent::UpdateUsers(server_id, channel, list_users) => {
                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    for user in list_users {
                        server.join(channel, user);
                    }
                }
            }
            MessageEvent::SelectChannel(server_id, channel) => {
                if let Some(server_id) = server_id
                    && let Some(Some(server)) = self.get_mut_server_from_id(*server_id)
                {
                    server.select_channel(channel);
                }
                self.current_id = *server_id;
            }
            MessageEvent::PrivMsg(server_id, source, target, _)
            | MessageEvent::ActionMsg(server_id, source, target, _) => {
                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    server.received_message(source, target);
                }
            }
            MessageEvent::Notice(server_id, source, target, _) => {
                if let Some(Some(server)) = self.get_mut_server_from_id(*server_id) {
                    server.received_message(source, target);
                }
            }
            _ => {}
        }
    }

    #[cfg(test)]
    pub fn new_single_server(
        in_length: usize,
        id: ServerID,
        server_name: String,
        nickname: String,
    ) -> Self {
        let mut s = Self {
            servers: std::iter::repeat_with(|| None).take(in_length).collect(),
            current_id: None,
        };

        s.init_server(id, server_name, nickname);

        s
    }
}

#[derive(Debug)]
pub struct IrcServerModel {
    users: ahash::AHashMap<IrcName, User>,

    //Usually not a lot of channels, so keeping a vector is fine
    list_channels: Vec<Channel>,
    current_channel: Option<IrcName>,

    current_nick: IrcName,
    server_id: ServerID,
    name: String,
}

impl IrcServerModel {
    pub fn new_model(server_id: ServerID, server_name: String, nick_name: String) -> Self {
        let mut users = AHashMap::new();
        users.insert(IrcName::new(&nick_name), User::new(true));
        Self {
            users,
            name: server_name,
            server_id,
            list_channels: Vec::new(),
            current_channel: None,
            current_nick: IrcName::new(&nick_name),
        }
    }

    pub fn get_current_channel(&self) -> Option<&IrcName> {
        self.current_channel.as_ref()
    }

    pub fn get_server_name(&self) -> &str {
        self.name.as_str()
    }

    //TODO:replace with IrcName
    pub fn get_current_nick(&self) -> &IrcName {
        &self.current_nick
    }

    pub fn get_server_id(&self) -> ServerID {
        self.server_id
    }

    pub fn get_target<'b>(&self, source: &'b IrcName, target: &'b IrcName) -> &'b IrcName {
        if target.eq(self.get_current_nick()) {
            source
        } else {
            target
        }
    }

    fn get_channel(&self, channel: &IrcName) -> Option<&Channel> {
        self.list_channels.iter().find(|v| v.name.eq(channel))
    }

    fn get_channel_mut(&mut self, channel: &IrcName) -> Option<&mut Channel> {
        self.list_channels.iter_mut().find(|v| v.name.eq(channel))
    }

    pub fn add_channel(&mut self, channel: &IrcName) -> usize {
        if let Some(c) = self.get_channel(channel) {
            return c.id;
        }
        let new_channel = Channel {
            id: self.list_channels.len(),
            name: channel.clone(),
            has_received_message: false,
            has_unread_message: false,
        };
        let id = new_channel.id;
        self.list_channels.push(new_channel);
        id
    }

    fn rename_channel(&mut self, old: &IrcName, new: &IrcName) {
        if let Some(c) = self.get_channel_mut(old) {
            c.name = new.clone();
        }
    }

    fn join(&mut self, channel: &IrcName, nick: &IrcName) {
        let id = self.add_channel(channel);

        let entry = self
            .users
            .entry(nick.clone())
            //TODO: might avoid clone, if the args are in the other order
            .or_insert_with(|| User::new(self.current_nick.eq(nick)));

        entry.join_section(id);
    }

    fn part(&mut self, channel: &IrcName, nick: &IrcName) {
        let id = self.add_channel(channel);

        let mut should_delete = false;
        if let Some(user) = self.users.get_mut(nick) {
            user.quit_section(id);
            should_delete = !user.has_joined_any_section();
        }

        if should_delete {
            self.users.remove(nick);
        }
    }

    fn quit(&mut self, nick: &IrcName) {
        self.users.remove(nick);
    }

    fn nick(&mut self, old: &IrcName, new: &IrcName) {
        if let Some(user) = self.users.remove(old) {
            self.rename_channel(old, new);

            if let Some(channel) = self.current_channel.as_mut()
                && *old == *channel.as_str()
            {
                *channel = new.clone();
            }

            self.users.insert(new.clone(), user);
        }

        if self.current_nick == *old {
            self.current_nick = new.clone();
        }
    }

    pub fn is_main_user(&self, user: &IrcName) -> bool {
        if let Some(user) = self.users.get(user) {
            user.is_main
        } else {
            false
        }
    }

    pub fn get_user(&self, user: &IrcName) -> Option<&User> {
        self.users.get(user)
    }

    pub fn has_user_joined_channel(&self, user: &IrcName, channel: &IrcName) -> bool {
        if let Some(id) = self.get_channel(channel).map(|v| v.id) {
            self.users
                .get(user)
                .map(|v| v.has_joined_section(id))
                .is_some_and(|v| v)
        } else {
            false
        }
    }

    pub fn get_all_joined_channel(&self, user: &IrcName) -> impl Iterator<Item = &IrcName> + '_ {
        let maybe_user = self.users.get(user);
        maybe_user.into_iter().flat_map(|u| {
            self.list_channels
                .iter()
                .filter(|section| u.has_joined_section(section.id))
                .map(|section| &section.name)
        })
    }

    // a(source) sends to b(target)
    fn received_message(&mut self, source: &IrcName, target: &IrcName) {
        let target = self.get_target(source, target);
        let id = self.add_channel(target);

        let new_message = self.current_channel.as_ref().is_some_and(|v| v.eq(target));

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

    fn select_channel(&mut self, channel: &IrcName) {
        self.current_channel = Some(channel.clone());
        if let Some(c) = self.get_channel_mut(channel) {
            c.has_unread_message = false;
        }
    }

    pub fn has_unread_message(&self, channel: &IrcName) -> bool {
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
        m.init_server(ServerID::new(0), "TEST".into(), "Me".into());
        m.current_id = Some(ServerID::new(0));
    }

    fn server(m: &IrcModel) -> &IrcServerModel {
        m.get_current_server().unwrap()
    }

    #[test]
    fn test_main_user() {
        let mut m = model();
        setup_server(&mut m);

        let s = server(&m);

        assert!(s.is_main_user(&"me".into()));
        assert!(s.is_main_user(&"ME".into()));
        assert!(s.is_main_user(&"Me".into()));
    }

    #[test]
    fn test_join_channel() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#Rust".into(),
            "Alice".into(),
        ));

        let s = server(&m);

        assert!(s.has_user_joined_channel(&"alice".into(), &"#rust".into()));
        assert!(s.has_user_joined_channel(&"ALICE".into(), &"#RUST".into()));
    }

    #[test]
    fn test_part_channel() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#Rust".into(),
            "Alice".into(),
        ));
        assert!(server(&m).has_user_joined_channel(&"alice".into(), &"#rust".into()));

        m.handle_action(&MessageEvent::Part(
            ServerID::new(0),
            "#RUST".into(),
            "ALICE".into(),
        ));

        assert!(!server(&m).has_user_joined_channel(&"alice".into(), &"#rust".into()));
    }

    #[test]
    fn test_quit_user() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "Alice".into(),
        ));
        assert!(server(&m).get_user(&IrcName::new("alice")).is_some());

        m.handle_action(&MessageEvent::Quit(
            ServerID::new(0),
            "ALICE".into(),
            Some("bye".into()),
        ));

        assert!(server(&m).get_user(&IrcName::new("alice")).is_none());
    }

    #[test]
    fn test_nick_change() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "Alice".into(),
        ));
        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "Jack".into(),
        ));

        // Create private channel with Jack
        m.handle_action(&MessageEvent::PrivMsg(
            ServerID::new(0),
            "jack".into(),
            "me".into(),
            "hello".into(),
        ));

        assert!(server(&m).get_channel(&"jack".into()).is_some());

        // Rename Alice → Bob
        m.handle_action(&MessageEvent::ReplaceUser(
            ServerID::new(0),
            "ALICE".into(),
            "BoB".into(),
        ));

        assert!(server(&m).get_user(&IrcName::new("alice")).is_none());
        assert!(server(&m).get_user(&IrcName::new("bob")).is_some());

        // Rename Jack → Miki
        m.handle_action(&MessageEvent::ReplaceUser(
            ServerID::new(0),
            "Jack".into(),
            "miki".into(),
        ));

        let s = server(&m);

        assert!(s.get_user(&IrcName::new("jack")).is_none());
        assert!(s.get_user(&IrcName::new("miki")).is_some());
        assert!(s.get_channel(&"jack".into()).is_none());
        assert!(s.get_channel(&"miki".into()).is_some());
    }

    #[test]
    fn test_select_channel_clears_unread() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#Rust".into(),
            "Alice".into(),
        ));

        m.handle_action(&MessageEvent::PrivMsg(
            ServerID::new(0),
            "alice".into(),
            "#RUST".into(),
            "hello".into(),
        ));

        assert!(server(&m).has_unread_message(&"#rust".into()));

        m.handle_action(&MessageEvent::SelectChannel(
            Some(ServerID::new(0)),
            "#RUST".into(),
        ));

        assert!(!server(&m).has_unread_message(&"#rust".into()));
    }

    #[test]
    fn test_get_all_joined_channels() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#Rust".into(),
            "Alice".into(),
        ));
        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#Linux".into(),
            "ALICE".into(),
        ));

        let channels: Vec<_> = server(&m).get_all_joined_channel(&"alice".into()).collect();

        assert_eq!(channels.len(), 2);
        assert!(channels.iter().any(|c| **c == "#rust"));
        assert!(channels.iter().any(|c| **c == "#linux"));
    }

    #[test]
    fn test_private_message_unread() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#Rust".into(),
            "Alice".into(),
        ));
        m.handle_action(&MessageEvent::SelectChannel(
            Some(ServerID::new(0)),
            "#General".into(),
        ));

        m.handle_action(&MessageEvent::PrivMsg(
            ServerID::new(0),
            "ALICE".into(),
            "#rust".into(),
            "hello".into(),
        ));

        assert!(server(&m).has_unread_message(&"#RUST".into()));
    }

    #[test]
    fn test_get_target() {
        let mut m = model();
        setup_server(&mut m);

        let s = server(&m);

        let alice = IrcName::new("Alice");
        let me = IrcName::new("ME");
        let rust = IrcName::new("#RUST");

        // target is main nick -> return source
        let t = s.get_target(&alice, &me);
        assert_eq!(t, "Alice");

        // target is channel -> return channel
        let t = s.get_target(&alice, &rust);
        assert_eq!(t, "#RUST");
    }

    #[test]
    fn test_peer_nick_change_in_private_channel() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "Me".into(),
        ));
        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "Jack".into(),
        ));
        m.handle_action(&MessageEvent::PrivMsg(
            ServerID::new(0),
            "Jack".into(),
            "Me".into(),
            "hello".into(),
        ));
        m.handle_action(&MessageEvent::SelectChannel(
            Some(ServerID::new(0)),
            "Jack".into(),
        ));

        m.handle_action(&MessageEvent::ReplaceUser(
            ServerID::new(0),
            "Jack".into(),
            "Miki".into(),
        ));

        // Messages must now be sent to the new nick
        assert_eq!(
            server(&m).get_current_channel().map(IrcName::as_str),
            Some("Miki")
        );
    }

    #[test]
    fn test_main_user_kept_after_rejoin() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "Me".into(),
        ));
        m.handle_action(&MessageEvent::Part(
            ServerID::new(0),
            "#rust".into(),
            "Me".into(),
        ));
        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "ME".into(),
        ));

        assert!(server(&m).is_main_user(&"me".into()));

        m.handle_action(&MessageEvent::ReplaceUser(
            ServerID::new(0),
            "Me".into(),
            "NewMe".into(),
        ));

        assert_eq!(server(&m).get_current_nick(), "NewMe");
        assert!(server(&m).is_main_user(&"newme".into()));
    }

    #[test]
    fn test_own_nick_change_without_channel() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "Me".into(),
        ));
        m.handle_action(&MessageEvent::Part(
            ServerID::new(0),
            "#rust".into(),
            "Me".into(),
        ));

        // Not in any channel, but the server still sends back our NICK
        m.handle_action(&MessageEvent::ReplaceUser(
            ServerID::new(0),
            "Me".into(),
            "NewMe".into(),
        ));
        assert_eq!(server(&m).get_current_nick(), "NewMe");

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#rust".into(),
            "NewMe".into(),
        ));
        assert!(server(&m).is_main_user(&"newme".into()));
        assert!(!server(&m).is_main_user(&"me".into()));
    }

    #[test]
    fn test_names_reply_does_not_change_current_channel() {
        let mut m = model();
        setup_server(&mut m);

        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#a".into(),
            "Me".into(),
        ));
        m.handle_action(&MessageEvent::SelectChannel(
            Some(ServerID::new(0)),
            "#a".into(),
        ));

        // NAMES for another channel, containing our own nick
        m.handle_action(&MessageEvent::UpdateUsers(
            ServerID::new(0),
            "#b".into(),
            vec!["@Me".into(), "Jack".into()],
        ));
        assert_eq!(
            server(&m).get_current_channel().map(IrcName::as_str),
            Some("#a")
        );
        assert!(server(&m).has_user_joined_channel(&"me".into(), &"#b".into()));

        // A JOIN alone does not switch either, only SelectChannel does
        m.handle_action(&MessageEvent::Join(
            ServerID::new(0),
            "#c".into(),
            "Me".into(),
        ));
        assert_eq!(
            server(&m).get_current_channel().map(IrcName::as_str),
            Some("#a")
        );
    }
}
