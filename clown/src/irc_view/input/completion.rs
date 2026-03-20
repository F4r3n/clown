use ahash::AHashMap;

use crate::config::{Config, ValueParameter};

use super::trie::Trie;

#[derive(Hash, PartialEq, Eq)]
struct KeyServerChannel {
    server_id: Option<usize>,
    channel: String,
}

pub struct InputCompletion {
    //List commands
    commands: Trie,

    //List config
    config: Trie,

    //Users per channel, can be changed a lot
    channels: ahash::AHashMap<KeyServerChannel, Trie>,
}

impl Default for InputCompletion {
    fn default() -> Self {
        Self {
            commands: Trie::new(),
            config: Trie::new(),
            channels: AHashMap::default(),
        }
    }
}

impl InputCompletion {
    pub fn add_command(&mut self, item: String) {
        self.commands.add_word(item);
    }

    pub fn add_config_field(&mut self, item: String) {
        self.config.add_word(item);
    }

    pub fn clear_config(&mut self) {
        self.config = Trie::new();
    }

    pub fn add_users(&mut self, server_id: usize, channel: &str, users: &Vec<String>) {
        let channel = Self::sanitize_key(channel);

        let channel = self
            .channels
            .entry(KeyServerChannel {
                channel: channel.to_string(),
                server_id: Some(server_id),
            })
            .or_insert(Trie::new());
        for user in users {
            channel.add_word(Self::sanitize_name(user).to_string());
        }
    }

    fn sanitize_name(user: &str) -> &str {
        user.strip_prefix('@').unwrap_or(user)
    }

    fn sanitize_key(key: &str) -> String {
        key.to_lowercase()
    }

    pub fn replace_user(&mut self, old: &str, new: &str) {
        for (_, trie) in self.channels.iter_mut() {
            trie.disable_word(old);
            trie.add_word(new.to_string());
        }
    }

    pub fn remove_channel(&mut self, server_id: usize, channel: &str) {
        let channel = Self::sanitize_key(channel);

        self.channels.remove(&KeyServerChannel {
            channel: channel.to_string(),
            server_id: Some(server_id),
        });
    }

    pub fn disable_user(&mut self, server_id: usize, channel: &str, user: &str) {
        let channel = Self::sanitize_key(channel);

        if let Some(channel) = self.channels.get_mut(&KeyServerChannel {
            channel: channel.to_string(),
            server_id: Some(server_id),
        }) {
            channel.disable_word(user);
        }
    }

    pub fn add_user(&mut self, server_id: usize, channel: &str, user: String) {
        let channel = Self::sanitize_key(channel);
        self.channels
            .entry(KeyServerChannel {
                channel: channel.to_string(),
                server_id: Some(server_id),
            })
            .or_insert(Trie::new())
            .add_word(user);
    }

    pub fn list(
        &self,
        server_id: Option<usize>,
        channel: &str,
        start_word: &str,
    ) -> Option<Vec<String>> {
        let channel = Self::sanitize_key(channel);

        if let Some(channel) = self.channels.get(&KeyServerChannel {
            channel: channel.to_string(),
            server_id,
        }) {
            channel.list(start_word)
        } else {
            None
        }
    }

    pub fn list_command(&self, start_word: &str) -> Option<Vec<String>> {
        self.commands.list(start_word)
    }

    pub fn list_config(&self, start_word: &str) -> Option<Vec<String>> {
        self.config.list(start_word)
    }
}

#[derive(PartialEq, Eq)]
enum CompletionKind {
    Command,
    Config,
    Nickname,
}

struct CompletionState {
    list: Vec<String>,
    kind: CompletionKind,
    start_character_pos: usize,
    index_list: usize,
}

#[derive(Default)]
pub struct Completion {
    pub input_completion: InputCompletion,
    state: Option<CompletionState>,
    pub current_channel: String,
    pub server_id: Option<usize>,
    on_empty_input_suffix: String,
    in_message_suffix: String,
}

impl Completion {
    pub fn set_completion_behaviour(
        &mut self,
        on_empty_input_suffix: String,
        in_message_suffix: String,
    ) {
        self.on_empty_input_suffix = on_empty_input_suffix;
        self.in_message_suffix = in_message_suffix;
    }

    pub fn set_completion(&mut self, start: usize, slice: &str, full_phrase: &str) {
        if self.state.is_some() {
            return;
        }

        //TODO: the completion context should be cursor dependant
        if let Some(end) = full_phrase.strip_prefix("/") {
            let tokens = end.split_whitespace().collect::<Vec<&str>>();

            match tokens.as_slice() {
                ["config", "get" | "set", config_option, ..] => {
                    let last = tokens.last().unwrap_or(&"");
                    if let Ok(expected_parameters) =
                        Config::expected_parameters_from_root(config_option)
                    {
                        let index = tokens.len().saturating_sub(4);

                        if let Some(expected_parameter) = expected_parameters.get(index)
                            && *expected_parameter == ValueParameter::Nickname
                            && let Some(list) = self.input_completion.list(
                                self.server_id,
                                &self.current_channel,
                                last,
                            )
                        {
                            self.apply_state(
                                list,
                                CompletionKind::Nickname,
                                start.saturating_add(full_phrase.len() - end.len() - 1),
                            );
                        }
                    } else if let Some(list) = self.input_completion.list_config(last) {
                        self.apply_state(
                            list,
                            CompletionKind::Config,
                            start.saturating_add(full_phrase.len() - end.len() - 1),
                        );
                    }
                }
                ["config", "get" | "set", ..] => {
                    let last = tokens.last().unwrap_or(&"");
                    if let Some(list) = self.input_completion.list_config(last) {
                        self.apply_state(
                            list,
                            CompletionKind::Config,
                            start.saturating_add(full_phrase.len() - end.len() - 1),
                        );
                    }
                }

                ["config", ..] => {
                    let mut list: Vec<String> = vec!["set".into(), "get".into()];
                    list.retain(|v| v.starts_with(slice));
                    self.apply_state(
                        list,
                        CompletionKind::Config,
                        start.saturating_add(full_phrase.len() - end.len() - 1),
                    );
                }
                _ => {
                    if let Some(list) = self.input_completion.list_command(end) {
                        self.apply_state(list, CompletionKind::Command, start.saturating_add(1));
                    }
                }
            };
        } else if let Some(list) =
            self.input_completion
                .list(self.server_id, &self.current_channel, slice)
        {
            self.state = Some(CompletionState {
                list,
                kind: CompletionKind::Nickname,
                start_character_pos: start,
                index_list: 0,
            });
        }
    }

    fn apply_state(&mut self, list: Vec<String>, kind: CompletionKind, pos: usize) {
        self.state = Some(CompletionState {
            list,
            kind,
            start_character_pos: pos,
            index_list: 0,
        });
    }

    pub fn get_next_completion(&mut self, is_first_word: bool) -> Option<(usize, String)> {
        if let Some(state) = self.state.as_mut()
            && !state.list.is_empty()
        {
            state.index_list = state.index_list.saturating_add(1) % state.list.len();

            if let Some(v) = state.list.get(state.index_list) {
                let mut v = v.to_string();
                if state.kind == CompletionKind::Nickname {
                    if is_first_word {
                        v.push_str(self.on_empty_input_suffix.as_str());
                    } else {
                        v.push_str(self.in_message_suffix.as_ref());
                    };
                }

                Some((state.start_character_pos, v))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.state = None;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_insert() {
        let mut comp = Completion::default();

        comp.input_completion.add_command("quit".into());
        comp.input_completion.add_command("help".into());

        comp.input_completion
            .add_users(0, "#test", &vec!["tata".to_string(), "titi".to_string()]);

        comp.current_channel = "#test".to_string();
        comp.server_id = Some(0);

        // Normal user completion (not first word)
        comp.set_completion(0, "t", "t");

        assert_eq!(
            comp.get_next_completion(false),
            Some((0, "titi".to_string()))
        );
        assert_eq!(
            comp.get_next_completion(false),
            Some((0, "tata".to_string()))
        );
        assert_eq!(
            comp.get_next_completion(false),
            Some((0, "titi".to_string()))
        );

        // No channel → no completion
        comp.current_channel = "".to_string();
        comp.reset();
        comp.set_completion(0, "t", "t");
        assert_eq!(comp.get_next_completion(false), None);

        // Command completion
        comp.reset();
        comp.set_completion(0, "/", "/");

        assert_eq!(
            comp.get_next_completion(true),
            Some((1, "quit".to_string()))
        );
        assert_eq!(
            comp.get_next_completion(true),
            Some((1, "help".to_string()))
        );

        comp.reset();
        comp.set_completion(0, "/h", "/h");
        assert_eq!(
            comp.get_next_completion(true),
            Some((1, "help".to_string()))
        );
    }

    #[test]
    fn test_insert_config() {
        let mut comp = Completion::default();

        comp.input_completion.add_command("/quit".into());
        comp.input_completion.add_command("/help".into());
        comp.input_completion.add_command("/config".into());
        comp.input_completion.add_user(0, "#test", "yolo".into());

        comp.input_completion
            .add_config_field("nickname_colors.seed".into());

        comp.current_channel = "#test".to_string();
        comp.server_id = Some(0);

        comp.set_completion(0, "n", "/config set n");
        assert_eq!(
            comp.get_next_completion(true),
            Some((0, "nickname_colors.seed".to_string()))
        );

        comp.reset();
        comp.set_completion(0, "s", "/config s");
        assert_eq!(comp.get_next_completion(true), Some((0, "set".to_string())));

        comp.reset();
        comp.set_completion(0, "", "/config");
        assert_eq!(comp.get_next_completion(true), Some((0, "get".to_string())));

        comp.reset();
        comp.set_completion(0, "y", "/config set nickname_colors.overrides y");
        assert_eq!(
            comp.get_next_completion(true),
            Some((0, "yolo".to_string()))
        );
    }

    #[test]
    fn test_insert_uppercase() {
        let mut comp = Completion {
            server_id: Some(0),
            ..Completion::default()
        };

        comp.input_completion
            .add_users(0, "#test", &vec!["tata".to_string(), "Titi".to_string()]);

        comp.current_channel = "#test".to_string();

        comp.set_completion(0, "t", "t");

        assert_eq!(
            comp.get_next_completion(false),
            Some((0, "Titi".to_string()))
        );
    }

    #[test]
    fn test_completion_suffixes() {
        let mut comp = Completion::default();

        comp.set_completion_behaviour(
            ": ".to_string(), // on_empty_input_suffix
            " ".to_string(),  // in_message_suffix
        );

        comp.input_completion
            .add_users(0, "#test", &vec!["tata".to_string()]);

        comp.current_channel = "#test".to_string();
        comp.server_id = Some(0);

        // First word completion → should use on_empty_input_suffix
        comp.set_completion(0, "t", "t");

        assert_eq!(
            comp.get_next_completion(true),
            Some((0, "tata: ".to_string()))
        );

        comp.reset();

        // In-message completion → should use in_message_suffix
        comp.set_completion(5, "t", "t");

        assert_eq!(
            comp.get_next_completion(false),
            Some((5, "tata ".to_string()))
        );
    }
}
