use ahash::AHashMap;

use super::trie::Trie;

#[derive(Hash, PartialEq, Eq)]
struct KeyServerChannel {
    server_id: usize,
    channel: String,
}

pub struct InputCompletion {
    //List commands
    commands: Trie,

    //Users per channel, can be changed a lot
    channels: ahash::AHashMap<KeyServerChannel, Trie>,
}

impl Default for InputCompletion {
    fn default() -> Self {
        Self {
            commands: Trie::new(),
            channels: AHashMap::default(),
        }
    }
}

impl InputCompletion {
    pub fn add_command(&mut self, item: &str) {
        self.commands.add_word(item);
    }

    pub fn add_users(&mut self, server_id: usize, channel: &str, users: &Vec<String>) {
        let channel = Self::sanitize_key(channel);

        let channel = self
            .channels
            .entry(KeyServerChannel {
                channel: channel.to_string(),
                server_id,
            })
            .or_insert(Trie::new());
        for user in users {
            channel.add_word(InputCompletion::sanitize_name(user));
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
            trie.add_word(new);
        }
    }

    pub fn remove_channel(&mut self, server_id: usize, channel: &str) {
        let channel = Self::sanitize_key(channel);

        self.channels.remove(&KeyServerChannel {
            channel: channel.to_string(),
            server_id,
        });
    }

    pub fn disable_user(&mut self, server_id: usize, channel: &str, user: &str) {
        let channel = Self::sanitize_key(channel);

        if let Some(channel) = self.channels.get_mut(&KeyServerChannel {
            channel: channel.to_string(),
            server_id,
        }) {
            channel.disable_word(user);
        }
    }

    pub fn add_user(&mut self, server_id: usize, channel: &str, user: &str) {
        let channel = Self::sanitize_key(channel);
        if let Some(channel) = self.channels.get_mut(&KeyServerChannel {
            channel: channel.to_string(),
            server_id,
        }) {
            channel.add_word(user);
        }
    }

    pub fn list(&self, server_id: usize, channel: &str, start_word: &str) -> Option<Vec<String>> {
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
}

#[derive(Default)]
pub struct Completion {
    pub input_completion: InputCompletion,
    completion_start: Option<usize>,
    current_completion: Option<Vec<String>>,
    pub current_channel: String,
    pub server_id: usize,
    current_index: Option<usize>,
}

impl Completion {
    pub fn set_completion(&mut self, start: usize, slice: &str) {
        if self.completion_start.is_some() {
            return;
        }

        if let Some(end) = slice.strip_prefix("/") {
            self.completion_start = Some(start.saturating_add(1));
            self.current_completion = self.input_completion.list_command(end);
            self.current_index = if self
                .current_completion
                .as_ref()
                .is_some_and(|v| !v.is_empty())
            {
                Some(0)
            } else {
                None
            };
        } else {
            self.completion_start = Some(start);
            self.current_completion =
                self.input_completion
                    .list(self.server_id, &self.current_channel, slice);
            self.current_index = if self
                .current_completion
                .as_ref()
                .is_some_and(|v| !v.is_empty())
            {
                Some(0)
            } else {
                None
            };
        }
    }

    pub fn get_next_completion(&mut self) -> Option<(usize, String)> {
        self.current_index?;
        self.current_index = Some(
            self.current_index
                .as_mut()
                .map_or(0, |v| v.saturating_add(1))
                % self
                    .current_completion
                    .as_ref()
                    .map(|v| v.len())
                    .unwrap_or(1),
        );

        if let Some(list) = self.current_completion.as_ref()
            && let Some(start) = self.completion_start
            && let Some(v) = list.get(self.current_index.unwrap_or(0))
        {
            Some((start, v.to_string()))
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.completion_start = None;
        self.current_completion = None;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_insert() {
        let mut comp = Completion::default();
        comp.input_completion.add_command("quit");
        comp.input_completion.add_command("help");

        comp.input_completion
            .add_users(0, "#test", &vec!["tata".to_string(), "titi".to_string()]);
        comp.current_channel = "#test".to_string();
        comp.set_completion(0, "t");
        assert_eq!(comp.get_next_completion(), Some((0, "titi".to_string())));
        assert_eq!(comp.get_next_completion(), Some((0, "tata".to_string())));
        assert_eq!(comp.get_next_completion(), Some((0, "titi".to_string())));

        comp.current_channel = "".to_string();
        comp.reset();
        comp.set_completion(0, "t");
        assert_eq!(comp.get_next_completion(), None);

        comp.reset();
        comp.set_completion(0, "/");
        assert_eq!(comp.get_next_completion(), Some((1, "quit".to_string())));
        assert_eq!(comp.get_next_completion(), Some((1, "help".to_string())));

        comp.reset();
        comp.set_completion(0, "/h");
        assert_eq!(comp.get_next_completion(), Some((1, "help".to_string())));
    }

    #[test]
    fn test_insert_uppercase() {
        let mut comp = Completion::default();

        comp.input_completion
            .add_users(0, "#test", &vec!["tata".to_string(), "Titi".to_string()]);
        comp.current_channel = "#test".to_string();
        comp.set_completion(0, "t");
        assert_eq!(comp.get_next_completion(), Some((0, "Titi".to_string())));
    }
}
