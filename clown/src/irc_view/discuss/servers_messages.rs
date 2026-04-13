use crate::message_irc::message_content::MessageContent;
use crate::message_irc::message_content::WordPos;
use crate::message_irc::message_logger::{
    LogReader, LoggedMessage, LoggedTimedMessage, MessageLogger,
};
use ahash::AHashMap;

use std::path::PathBuf;

fn create_message(log: LoggedTimedMessage<'_>) -> MessageContent {
    match log.message {
        LoggedMessage::Action { source, content } => {
            MessageContent::action(source.to_string(), content.to_string())
                .with_time(log.time)
                .with_log()
        }
        LoggedMessage::Topic {
            source,
            channel,
            content,
        } => {
            let data = format!(
                "{} has changed topic for {} to \"{}\"",
                source, channel, content
            );
            MessageContent::info(data).with_time(log.time).with_log()
        }
        LoggedMessage::Join { source, channel } => {
            MessageContent::info(format!("{source} has joined {channel}"))
                .with_time(log.time)
                .with_log()
        }
        LoggedMessage::Part { source, channel } => {
            MessageContent::info(format!("{} has left {}", source, channel))
                .with_time(log.time)
                .with_log()
        }
        LoggedMessage::Quit { source } => MessageContent::info(format!("{} has quit", source))
            .with_time(log.time)
            .with_log(),
        LoggedMessage::NickChange { old, new } => {
            MessageContent::info(format!("{} has changed their nickname to {}", old, new))
                .with_time(log.time)
                .with_log()
        }
        LoggedMessage::Message { source, content } => {
            MessageContent::message(Some(source.to_string()), content.to_string())
                .with_time(log.time)
                .with_log()
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Range {
    line: usize,
    word_pos: WordPos,
}

#[derive(Debug, Default)]
pub struct ServersMessages {
    messages: Vec<ChannelMessages>,
    log_folder: PathBuf,
}

impl ServersMessages {
    pub fn new(log_folder: PathBuf) -> Self {
        let mut result = Self {
            log_folder,
            messages: vec![],
        };
        result.add_server_group(None, None);
        result
    }

    pub fn add_message(
        &mut self,
        server_id: Option<usize>,
        channel: &str,
        in_message: MessageContent,
    ) {
        if let Some(server_group) = self.get_server_group_mut(server_id) {
            server_group.add_message(channel, in_message);
        } else if let Some(server_group) = self.get_server_group_mut(None) {
            //If not connected (has not passed Welcome yet)  but received messages
            // send all messages to main channel
            server_group.add_message(channel, in_message);
        }
    }

    pub fn open_log(&mut self, server_id: usize, channel: &str) {
        if let Some(server_group) = self.get_server_group(Some(server_id))
            && server_group.is_log_open(channel)
        {
            return;
        }
        let path = self.log_folder.clone();
        if let Some(server_group) = self.get_server_group_mut(Some(server_id)) {
            server_group.open_log(path, channel);
        }
    }

    pub fn read_log(
        &mut self,
        number_lines: usize,
        server_id: usize,
        channel: &str,
    ) -> anyhow::Result<usize> {
        if let Some(server_group) = self.get_server_group_mut(Some(server_id)) {
            server_group.read_log(channel, number_lines)
        } else {
            Ok(0)
        }
    }

    fn server_id_position(server_id: Option<usize>) -> usize {
        server_id.map(|v| v.saturating_add(1)).unwrap_or(0)
    }

    pub fn add_server_group(
        &mut self,
        server_id: Option<usize>,
        server_address: Option<String>,
    ) -> Option<&mut ChannelMessages> {
        let position = Self::server_id_position(server_id);
        let new_length = position.saturating_add(1);
        self.messages.resize_with(new_length, Default::default);
        if let Some(server_group) = self.messages.get_mut(position) {
            server_group.server_address = server_address;
        }

        self.get_server_group_mut(server_id)
    }

    fn get_server_group_mut(&mut self, server_id: Option<usize>) -> Option<&mut ChannelMessages> {
        self.messages.get_mut(Self::server_id_position(server_id))
    }

    fn get_server_group(&self, server_id: Option<usize>) -> Option<&ChannelMessages> {
        self.messages.get(Self::server_id_position(server_id))
    }

    pub fn rename(&mut self, server_id: Option<usize>, old: &str, new: &str) {
        if let Some(server_group) = self.get_server_group_mut(server_id) {
            server_group.rename(old, new);
        }
    }

    pub fn has_messages(&self, server_id: Option<usize>, channel: &str) -> bool {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.has_messages(channel)
        } else {
            false
        }
    }

    pub fn get_messages(&self, server_id: Option<usize>, channel: &str) -> Option<&Messages> {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.get_messages(channel)
        } else {
            None
        }
    }

    pub fn get_url_from_range(
        &self,
        server_id: Option<usize>,
        channel: &str,
        range: &Range,
    ) -> Option<String> {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.get_url_from_range(channel, range)
        } else {
            None
        }
    }

    pub fn get_word_pos(
        &self,
        server_id: Option<usize>,
        channel: &str,
        index: usize,
        character_pos: usize,
    ) -> Option<Range> {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.get_word_pos(channel, index, character_pos)
        } else {
            None
        }
    }
}

//Cannot have only one vector
// The logged message would have been inserted at the beginning
// The logged messages are reversed in order
#[derive(Debug, Default)]
pub struct Messages {
    logged_messages: Vec<MessageContent>,
    messages: Vec<MessageContent>,
    log_reader: Option<LogReader<std::fs::File>>,
}

impl Messages {
    pub fn push_new(&mut self, in_message: MessageContent) {
        self.messages.push(in_message);
    }

    pub fn is_empty(&self) -> bool {
        self.logged_messages.is_empty() && self.messages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.logged_messages.len() + self.messages.len()
    }

    // Logged -1 -2 -3 -4
    // Message 0 1 2 3
    // -1 -2 -3 -4 0 1 2 3
    pub fn get(&self, index: usize) -> Option<&MessageContent> {
        let logged_len = self.logged_messages.len();

        if index < logged_len {
            // reverse access
            let rev_index = logged_len - 1 - index;
            self.logged_messages.get(rev_index)
        } else {
            self.messages.get(index - logged_len)
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &MessageContent> {
        self.logged_messages
            .iter()
            .rev()
            .chain(self.messages.iter())
    }

    fn open_log(&mut self, log_folder: PathBuf, server_address: &str, channel: &str) {
        let name = MessageLogger::compute_filename(server_address, Some(channel));
        let path = log_folder.join(name);
        self.log_reader = LogReader::try_from_path(path.as_path()).ok();
        if let Some(log_reader) = self.log_reader.as_mut() {
            log_reader.seek_last_time(
                self.messages
                    .first()
                    .map(|v| v.get_time())
                    .unwrap_or(std::time::SystemTime::now()),
            );
        }
    }

    fn did_day_change(last_check: std::time::SystemTime, now: std::time::SystemTime) -> bool {
        let last_date: chrono::DateTime<chrono::Local> = last_check.into();
        let now_date: chrono::DateTime<chrono::Local> = now.into();

        last_date.date_naive() != now_date.date_naive()
    }

    fn read_log(&mut self, number_lines: usize) -> anyhow::Result<usize> {
        let log_reader = match self.log_reader.as_mut() {
            Some(reader) => reader,
            None => return Ok(0),
        };
        let mut last_time = self.logged_messages.last().map(|v| v.get_time()).unwrap_or(
            self.messages
                .first()
                .map(|v| v.get_time())
                .unwrap_or(std::time::SystemTime::now()),
        );
        let list_read = log_reader.read(number_lines)?;
        let number_lines_before = self.logged_messages.len();
        for msg in list_read.into_iter() {
            if Self::did_day_change(last_time, msg.time) {
                let now_date: chrono::DateTime<chrono::Local> = msg.time.into();

                self.logged_messages.push(
                    MessageContent::info(now_date.format("%d/%m/%Y").to_string())
                        .with_time(msg.time),
                );
            }
            last_time = msg.time;

            self.logged_messages.push(create_message(msg));
        }

        Ok(self
            .logged_messages
            .len()
            .saturating_sub(number_lines_before))
    }
}

#[derive(Debug, Default)]
pub struct ChannelMessages {
    messages: AHashMap<String, Messages>,
    server_address: Option<String>,
}

impl ChannelMessages {
    pub fn add_message(&mut self, channel: &str, in_message: MessageContent) {
        self.messages
            .entry(channel.to_string())
            .or_default()
            .push_new(in_message);
    }

    fn rename(&mut self, old: &str, new: &str) {
        if let Some(messages) = self.messages.remove(old) {
            self.messages.insert(new.to_string(), messages);
        }
    }

    fn is_log_open(&self, channel: &str) -> bool {
        self.messages
            .get(channel)
            .is_some_and(|v| v.log_reader.is_some())
    }

    fn open_log(&mut self, log_folder: PathBuf, channel: &str) {
        if let Some(server_address) = self.server_address.as_deref() {
            self.messages
                .entry(channel.to_string())
                .or_default()
                .open_log(log_folder, server_address, channel);
        }
    }

    fn read_log(&mut self, channel: &str, number_lines: usize) -> anyhow::Result<usize> {
        self.messages
            .entry(channel.to_string())
            .or_default()
            .read_log(number_lines)
    }

    pub fn has_messages(&self, channel: &str) -> bool {
        self.messages.get(channel).is_some_and(|c| !c.is_empty())
    }

    fn get_messages(&self, channel: &str) -> Option<&Messages> {
        self.messages.get(channel)
    }

    fn get_url_from_range(&self, channel: &str, range: &Range) -> Option<String> {
        self.messages
            .get(channel)
            .and_then(|messages| messages.get(range.line))
            .and_then(|message| message.get_url_from_pos(&range.word_pos))
            .map(|str| str.to_string())
    }

    fn get_word_pos(&self, channel: &str, index: usize, character_pos: usize) -> Option<Range> {
        self.messages
            .get(channel)
            .and_then(|messages| messages.get(index))
            .and_then(|message| message.get_word_pos(character_pos))
            .map(|w| Range {
                line: index,
                word_pos: w,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_irc::message_content::MessageContent;

    #[test]
    fn test_messages_indexing_logic() {
        let mut msgs = Messages::default();

        // Simulating 2 logged messages (Stored -1, -2 but viewed as 0, 1)
        // In your impl, logged_messages are pushed normally but iter/get rev them.
        msgs.logged_messages
            .push(MessageContent::info("Oldest".to_string())); // index 1 in logical view
        msgs.logged_messages
            .push(MessageContent::info("Older".to_string())); // index 0 in logical view

        // Simulating 2 live messages
        msgs.messages
            .push(MessageContent::info("Live 1".to_string())); // index 2
        msgs.messages
            .push(MessageContent::info("Live 2".to_string())); // index 3

        assert_eq!(msgs.len(), 4);

        // Check logical order: [Older, Oldest, Live 1, Live 2]
        assert_eq!(msgs.get(0).unwrap().get_content(), "Older");
        assert_eq!(msgs.get(1).unwrap().get_content(), "Oldest");
        assert_eq!(msgs.get(2).unwrap().get_content(), "Live 1");
        assert_eq!(msgs.get(3).unwrap().get_content(), "Live 2");
        assert!(msgs.get(4).is_none());
    }

    #[test]
    fn test_messages_iterator() {
        let mut msgs = Messages::default();
        msgs.logged_messages
            .push(MessageContent::info("Log".to_string()));
        msgs.messages.push(MessageContent::info("Live".to_string()));

        let combined: Vec<String> = msgs.iter().map(|m| m.get_content().to_string()).collect();

        assert_eq!(combined, vec!["Log", "Live"]);
    }

    #[test]
    fn test_server_message_routing() {
        let mut storage = ServersMessages::new(PathBuf::from("/tmp"));
        let server_id = Some(1);
        let channel = "#rust";

        storage.add_server_group(server_id, Some("irc.libera.chat".to_string()));
        storage.add_message(
            server_id,
            channel,
            MessageContent::message(Some("alice".into()), "hello".into()),
        );

        // Ensure we can retrieve it
        assert!(storage.has_messages(server_id, channel));
        let msgs = storage.get_messages(server_id, channel).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs.get(0).unwrap().get_content(), "hello");

        // Ensure it didn't leak to another channel
        assert!(!storage.has_messages(server_id, "#other"));
    }

    #[test]
    fn test_channel_rename() {
        let mut storage = ServersMessages::new(PathBuf::from("/tmp"));
        let server_id = Some(0);
        storage.add_server_group(server_id, Some("localhost".to_string()));
        storage.add_message(server_id, "Bob", MessageContent::info("Hi Bob".to_string()));

        // Bob changes nick to Bobby
        storage.rename(server_id, "Bob", "Bobby");
        assert!(!storage.has_messages(server_id, "Bob"));
        assert!(storage.has_messages(server_id, "Bobby"));

        let msgs = storage.get_messages(server_id, "Bobby").unwrap();
        assert_eq!(msgs.get(0).unwrap().get_content(), "Hi Bob");
    }

    #[test]
    fn test_unconnected_message_fallback() {
        let mut storage = ServersMessages::new(PathBuf::from("/tmp"));

        // This should fall back to the default group (index 0)
        storage.add_message(
            None,
            "Status",
            MessageContent::info("Connecting...".to_string()),
        );

        assert!(storage.has_messages(None, "Status"));
        let msgs = storage.get_messages(None, "Status").unwrap();
        assert_eq!(msgs.get(0).unwrap().get_content(), "Connecting...");
    }
}
