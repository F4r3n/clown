use super::log_parser;
use crate::message_event::MessageEvent;
use ahash::AHashMap;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::{
    io::{BufReader, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
};
const LOG_FLUSH_TIMER_SECONDS: u64 = 5;

//Many filedesc can be opened without receiving any message
//Using a LRU should be more efficient, but no need for now
const LOG_OPENED_TIMER_MINUTES: u64 = 120;

#[derive(Debug, Clone)]
pub enum LoggedMessage<'a> {
    Topic {
        source: Cow<'a, str>,
        channel: Cow<'a, str>,
        content: Cow<'a, str>,
    },
    Join {
        source: Cow<'a, str>,
        channel: Cow<'a, str>,
    },
    Part {
        source: Cow<'a, str>,
        channel: Cow<'a, str>,
    },
    Quit {
        source: Cow<'a, str>,
    },
    NickChange {
        old: Cow<'a, str>,
        new: Cow<'a, str>,
    },
    Message {
        source: Cow<'a, str>,
        content: Cow<'a, str>,
    },
    Action {
        source: Cow<'a, str>,
        content: Cow<'a, str>,
    },
}
impl<'a> LoggedMessage<'a> {
    pub fn content(&self) -> Option<&Cow<'a, str>> {
        match self {
            LoggedMessage::Message { source, content } => Some(content),
            LoggedMessage::Action { source, content } => Some(content),
            LoggedMessage::Topic {
                source,
                channel,
                content,
            } => Some(content),
            _ => None,
        }
    }

    pub fn source(&self) -> Option<&Cow<'a, str>> {
        match self {
            LoggedMessage::Message { source, content } => Some(source),
            LoggedMessage::Action { source, content } => Some(source),
            LoggedMessage::Topic {
                source,
                channel,
                content,
            } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoggedTimedMessage<'a> {
    pub time: std::time::SystemTime,
    pub message: LoggedMessage<'a>,
}

impl<'a> LoggedTimedMessage<'a> {
    pub fn content(&self) -> Option<&Cow<'a, str>> {
        self.message.content()
    }

    pub fn time(&self) -> std::time::SystemTime {
        self.time
    }

    pub fn source(&self) -> Option<&Cow<'a, str>> {
        self.message.source()
    }
}

struct LogWriter {
    duration: std::time::Instant,
    last_data_written: std::time::Instant,
    buffer: std::io::BufWriter<std::fs::File>,
}

impl LogWriter {
    pub fn try_from_path(path: &Path) -> anyhow::Result<LogWriter> {
        Ok(Self {
            duration: std::time::Instant::now(),
            last_data_written: std::time::Instant::now(),
            buffer: Self::init(path)?,
        })
    }

    fn init(path: &Path) -> anyhow::Result<BufWriter<std::fs::File>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::options()
            .append(true)
            .create(true)
            .open(path)?;
        Ok(BufWriter::new(file))
    }

    fn flush(&mut self, force_flush: bool) -> std::io::Result<()> {
        if force_flush
            || (self.duration.elapsed() > std::time::Duration::from_secs(LOG_FLUSH_TIMER_SECONDS))
        {
            self.duration = std::time::Instant::now();
            if !self.buffer.buffer().is_empty() {
                self.last_data_written = std::time::Instant::now();
            }
            self.buffer.flush()?;
        }
        Ok(())
    }

    fn should_close(&self) -> bool {
        self.last_data_written.elapsed() > std::time::Duration::from_mins(LOG_OPENED_TIMER_MINUTES)
    }

    fn write(&mut self, data: impl std::fmt::Display) -> std::io::Result<()> {
        writeln!(self.buffer, "{}\t{}", Self::get_current_time(), data)?;

        Ok(())
    }

    fn get_current_time() -> impl std::fmt::Display {
        let now = chrono::Utc::now();
        now.format("%Y-%m-%d %H:%M:%S")
    }

    fn write_message(&mut self, message: LoggedMessage<'_>) -> std::io::Result<()> {
        match message {
            LoggedMessage::Topic {
                source,
                channel,
                content,
            } => self.write(format_args!(
                "--\t {} has changed topic for {} to \"{}\"",
                source, channel, content
            )),
            LoggedMessage::Join { source, channel } => {
                self.write(format_args!("-->\t {} has joined {}", source, channel))
            }
            LoggedMessage::Part { source, channel } => {
                self.write(format_args!("<--\t {} has left {}", source, channel))
            }
            LoggedMessage::Quit { source } => self.write(format_args!("<--\t {} has quit", source)),
            LoggedMessage::NickChange { old, new } => self.write(format_args!(
                "<--\t {} has changed their nickname to {}",
                old, new
            )),
            LoggedMessage::Message { source, content } => {
                self.write(format_args!("{} {}", source, content))
            }
            LoggedMessage::Action { source, content } => {
                self.write(format_args!("* {} {}", source, content))
            }
        }
    }
}

#[derive(Debug)]
pub struct LogReader<R: Read + Seek> {
    buffer: std::io::BufReader<R>,
    seek_pos: u64, //starts from start, because the logs appends at the end
}

impl LogReader<std::fs::File> {
    pub fn try_from_path(path: &Path) -> anyhow::Result<Self> {
        let file = Self::init(path)?;
        Ok(Self {
            seek_pos: file.metadata()?.len(),
            buffer: BufReader::new(file),
        })
    }

    //Starts in reverse
    pub fn try_from_path_with_start_pos(path: &Path, position: u64) -> anyhow::Result<Self> {
        let file = Self::init(path)?;
        Ok(Self {
            seek_pos: position,
            buffer: BufReader::new(file),
        })
    }
}

impl<R: Read + Seek> LogReader<R> {
    fn init(path: &Path) -> anyhow::Result<std::fs::File> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::options().read(true).open(path)?;
        Ok(file)
    }

    #[cfg(test)]
    pub fn new(mut reader: R) -> anyhow::Result<Self> {
        let end_pos = reader.seek(std::io::SeekFrom::End(0))?;
        Ok(Self {
            buffer: BufReader::new(reader),
            seek_pos: end_pos,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.seek_pos == 0
    }

    fn find_last_offset(&mut self, target: std::time::SystemTime) -> anyhow::Result<u64> {
        let mut pos = self.seek_pos;
        let mut chunk = [0; 1024];
        let mut carry: Vec<u8> = Vec::new();

        loop {
            if pos == 0 {
                break;
            }

            let read_start = pos.saturating_sub(1024);
            self.buffer.seek(std::io::SeekFrom::Start(read_start))?;

            let read_size = self.buffer.read(&mut chunk)?;
            if read_size == 0 {
                break;
            }

            let mut combined = Vec::with_capacity(read_size + carry.len());
            combined.extend_from_slice(&chunk[..read_size]);
            combined.extend_from_slice(&carry);

            let mut start = combined.len();

            for (i, b) in combined.iter().enumerate().rev() {
                if *b == b'\n' {
                    let line = &combined[i + 1..start];

                    if !line.is_empty() {
                        let parsed = log_parser::parse(line)?;

                        if parsed.time < target {
                            return Ok(read_start + start as u64);
                        }
                    }

                    start = i + 1;
                }
            }

            // keep incomplete line
            carry = combined[..start].to_vec();

            pos = read_start;
        }

        // If nothing found, return start of file
        Ok(0)
    }

    //target should be in utc
    pub fn seek_last_time(&mut self, target: std::time::SystemTime) -> bool {
        if let Ok(offset) = self.find_last_offset(target) {
            self.seek_pos = offset;
            true
        } else {
            false
        }
    }

    pub fn read(&mut self, number_lines: usize) -> anyhow::Result<Vec<LoggedTimedMessage<'_>>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }
        let mut vec = Vec::new();
        let mut to_read = number_lines;
        let mut carry: Vec<u8> = Vec::new();

        while to_read > 0 && self.seek_pos > 0 {
            let amount_to_read = self.seek_pos.min(1024);
            self.seek_pos -= amount_to_read;

            self.buffer.seek(std::io::SeekFrom::Start(self.seek_pos))?;
            let mut chunk = vec![0; amount_to_read as usize];
            self.buffer.read_exact(&mut chunk)?;

            // BACKWARDS STITCHING: chunk (earlier) + carry (later)
            let mut combined = chunk;
            combined.extend_from_slice(&carry);

            let mut end_idx = combined.len();
            for i in (0..combined.len()).rev() {
                if combined.get(i) == Some(&b'\n') {
                    if let Some(line) = combined.get(i + 1..end_idx)
                        && !line.is_empty()
                    {
                        if let Ok(parsed_message) = log_parser::parse(line) {
                            vec.push(parsed_message);
                        } else {
                            tracing::error!("Cannot parse {}", std::str::from_utf8(line)?);
                        }
                        to_read -= 1;
                        if to_read == 0 {
                            self.seek_pos += (i + 1) as u64;
                            break;
                        }
                    }

                    end_idx = i;
                }
            }

            if to_read == 0 {
                break;
            }
            carry = combined[..end_idx].to_vec();
        }

        if to_read > 0 && !carry.is_empty() {
            if let Ok(parsed_message) = log_parser::parse(&carry) {
                vec.push(parsed_message);
            } else {
                tracing::error!("Cannot parse {}", std::str::from_utf8(&carry)?);
            }
        }

        Ok(vec)
    }

    pub fn iter(&mut self) -> LogReaderIter<'_, R> {
        let exhausted = self.is_empty();
        LogReaderIter {
            reader: self,
            carry: Vec::new(),
            pending: VecDeque::new(),
            exhausted,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoggedMessageWithPos(LoggedTimedMessage<'static>, u64);

impl LoggedMessageWithPos {
    pub fn get_message(&self) -> &LoggedTimedMessage<'static> {
        &self.0
    }

    pub fn get_offset(&self) -> u64 {
        self.1
    }
}

pub struct LogReaderIter<'r, R: Read + Seek> {
    reader: &'r mut LogReader<R>,
    carry: Vec<u8>,
    pending: VecDeque<LoggedMessageWithPos>,
    exhausted: bool,
}

impl<R: Read + Seek> LogReaderIter<'_, R> {
    fn fill(&mut self) -> anyhow::Result<()> {
        if self.reader.seek_pos == 0 {
            if !self.carry.is_empty() {
                if let Ok(msg) = log_parser::parse(&self.carry) {
                    self.pending.push_back(LoggedMessageWithPos(msg, 0));
                } else {
                    tracing::error!(
                        "Cannot parse {}",
                        std::str::from_utf8(&self.carry).unwrap_or("<invalid utf8>")
                    );
                }
                self.carry.clear();
            }
            self.exhausted = true;
            return Ok(());
        }

        let amount_to_read = self.reader.seek_pos.min(1024);
        self.reader.seek_pos -= amount_to_read;

        self.reader
            .buffer
            .seek(std::io::SeekFrom::Start(self.reader.seek_pos))?;
        let mut chunk = vec![0; amount_to_read as usize];
        self.reader.buffer.read_exact(&mut chunk)?;

        let old_carry = std::mem::take(&mut self.carry);
        let mut combined = chunk;
        combined.extend_from_slice(&old_carry);

        let mut end_idx = combined.len();
        for i in (0..combined.len()).rev() {
            if combined.get(i) == Some(&b'\n') {
                if let Some(line) = combined.get(i + 1..end_idx)
                    && !line.is_empty()
                {
                    if let Ok(msg) = log_parser::parse(line) {
                        self.pending.push_back(LoggedMessageWithPos(msg, i as u64));
                    } else {
                        tracing::error!(
                            "Cannot parse {}",
                            std::str::from_utf8(line).unwrap_or("<invalid utf8>")
                        );
                    }
                }
                end_idx = i;
            }
        }

        self.carry = combined[..end_idx].to_vec();
        Ok(())
    }
}

impl<R: Read + Seek> Iterator for LogReaderIter<'_, R> {
    type Item = anyhow::Result<LoggedMessageWithPos>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pending.is_empty() && !self.exhausted {
            if let Err(e) = self.fill() {
                self.exhausted = true;
                return Some(Err(e));
            }
        }
        self.pending.pop_front().map(Ok)
    }
}

pub struct MessageLogger {
    folder: PathBuf,
    writers: ahash::AHashMap<String, LogWriter>,
}

#[derive(Debug, PartialEq)]
struct LogKey<'a> {
    server_address: &'a str,
    target: &'a str,
}

impl MessageLogger {
    pub fn new(folder: PathBuf) -> Self {
        Self {
            folder,
            writers: AHashMap::new(),
        }
    }

    fn sanitize_path(word: &str) -> String {
        word.to_ascii_lowercase()
            .chars()
            .map(|v| match v {
                '\\' | '/' => '_',
                _ => v,
            })
            .collect::<String>()
    }

    //Non cryptographic hash
    // will stay stable across rust and crates versions
    // Risk of collision very low
    fn hash_target(target: LogKey<'_>) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;

        fn write(hash: &mut u64, bytes: &[u8]) {
            for b in bytes {
                *hash ^= u64::from(*b);
                *hash = hash.wrapping_mul(FNV_PRIME);
            }
        }

        write(&mut hash, target.server_address.as_bytes());
        write(&mut hash, target.target.as_bytes());

        hash
    }

    pub fn compute_filename(server_address: &str, target: Option<&str>) -> String {
        let target = target.unwrap_or("server").to_lowercase();

        let name = format!(
            "{}.{}.{}.log",
            Self::sanitize_path(server_address),
            Self::sanitize_path(&target),
            // if someone is called foo/bar and foo_bar the same file will be used
            // Use a hash to be more precise
            Self::hash_target(LogKey {
                server_address,
                target: &target
            })
        );
        name
    }

    pub fn get_log_info(path: &Path) -> Option<(&str, &str, &str)> {
        if path.extension().is_some_and(|ext| ext != "log") {
            return None;
        }
        if let Some(name) = path.file_name()
            && let Some(name) = name.to_str()
        {
            let mut split = name.split('.');
            let server_name = split.next();
            let channel = split.next();
            let hash = split.next();
            if let Some(server_name) = server_name
                && let Some(channel) = channel
                && let Some(hash) = hash
            {
                Some((server_name, channel, hash))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn init_buffer(
        &mut self,
        server_address: &str,
        target: Option<&str>,
    ) -> anyhow::Result<&mut LogWriter> {
        //The name is not sanitized because is only used as a key to a hashmap
        let name = format!(
            "{}{}",
            server_address,
            target.unwrap_or("server").to_ascii_lowercase()
        );

        let logger = match self.writers.entry(name) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                //If new key, sanitize input
                let name = Self::compute_filename(server_address, target);
                let logger = LogWriter::try_from_path(&self.folder.join(name))?;
                v.insert(logger)
            }
        };

        Ok(logger)
    }

    pub fn flush_checker(&mut self) -> std::io::Result<()> {
        let mut error: Option<std::io::Error> = None;

        self.writers.retain(|_, logger| {
            if let Err(e) = logger.flush(false) {
                error = Some(e);
                return true;
            }
            !logger.should_close()
        });

        if let Some(e) = error {
            return Err(e);
        }

        Ok(())
    }

    fn write_to_target(
        &mut self,
        server_address: &str,
        target: Option<&str>,
        data: LoggedMessage<'_>,
        force_flush: bool,
    ) -> anyhow::Result<()> {
        let logger = self.init_buffer(server_address, target)?;
        logger.write_message(data)?;
        logger.flush(force_flush)?;

        Ok(())
    }

    pub fn write_message(
        &mut self,
        server_address: &str,
        irc_model: Option<&crate::state::irc_model::IrcModel>,
        message: &MessageEvent,
    ) -> anyhow::Result<()> {
        match message {
            MessageEvent::Join(_, channel, user) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    LoggedMessage::Join {
                        source: Cow::Borrowed(user),
                        channel: Cow::Borrowed(channel),
                    },
                    false,
                )?;
            }

            MessageEvent::ReplaceUser(server_id, old, new) => {
                if let Some(irc_model) = irc_model.as_ref() {
                    for channel in irc_model.get_all_joined_channel(*server_id, old) {
                        self.write_to_target(
                            server_address,
                            Some(channel),
                            LoggedMessage::NickChange {
                                old: Cow::Borrowed(old),
                                new: Cow::Borrowed(new),
                            },
                            true,
                        )?;
                    }
                }
            }

            MessageEvent::Part(_, channel, user) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    LoggedMessage::Part {
                        source: Cow::Borrowed(user),
                        channel: Cow::Borrowed(channel),
                    },
                    true,
                )?;
            }

            MessageEvent::Quit(server_id, user, _) => {
                if let Some(irc_model) = irc_model.as_ref() {
                    for channel in irc_model.get_all_joined_channel(*server_id, user) {
                        self.write_to_target(
                            server_address,
                            Some(channel),
                            LoggedMessage::Quit {
                                source: Cow::Borrowed(user),
                            },
                            true,
                        )?;
                    }
                }
            }

            MessageEvent::SetTopic(_, Some(source), channel, content) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    LoggedMessage::Topic {
                        source: Cow::Borrowed(source),
                        channel: Cow::Borrowed(channel),
                        content: Cow::Borrowed(content),
                    },
                    false,
                )?;
            }

            MessageEvent::Notice(server_id, source, target, content)
            | MessageEvent::PrivMsg(server_id, source, target, content) => {
                if let Some(irc_model) = irc_model.as_ref()
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    let target = irc_server.get_target(source, target);

                    self.write_to_target(
                        server_address,
                        Some(target),
                        LoggedMessage::Message {
                            source: Cow::Borrowed(source),
                            content: Cow::Borrowed(content),
                        },
                        false,
                    )?;
                }
            }

            MessageEvent::ActionMsg(_, source, target, content) => {
                self.write_to_target(
                    server_address,
                    Some(target),
                    LoggedMessage::Action {
                        source: Cow::Borrowed(source),
                        content: Cow::Borrowed(content),
                    },
                    false,
                )?;
            }

            _ => {}
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use std::fs;
    use std::io::Cursor;
    use std::time::SystemTime;
    use tempfile::tempdir;
    #[test]
    fn test_sanitize_path() {
        assert_eq!(
            MessageLogger::sanitize_path("../../Chat/Room"),
            ".._.._chat_room"
        );
        assert_eq!(MessageLogger::sanitize_path("Admin\\Tasks"), "admin_tasks");
        assert_eq!(MessageLogger::sanitize_path("General"), "general");
    }

    #[test]
    fn test_logger_creation_and_writing() {
        let dir = tempdir().expect("Cannot create dir");
        let log_path = dir.path().join("test.log");

        let mut logger = LogWriter::try_from_path(&log_path).unwrap();
        logger.write("Hello, Rust!").unwrap();
        logger.flush(true).unwrap(); // Force flush to ensure it hits the disk

        let content = fs::read_to_string(log_path).unwrap();
        assert!(content.contains("Hello, Rust!"));
        // Check for timestamp format (YYYY-MM-DD)
        assert!(content.contains(&chrono::Local::now().format("%Y-%m-%d").to_string()));
    }
    use chrono::TimeZone;
    fn parse_utc_to_system_time(date_str: &str) -> anyhow::Result<SystemTime> {
        let format = "%Y-%m-%d %H:%M:%S";

        let naive = NaiveDateTime::parse_from_str(date_str, format)?;

        let datetime_utc: DateTime<Utc> = Utc.from_utc_datetime(&naive);

        Ok(SystemTime::from(datetime_utc))
    }

    fn system_time_to_utc_string(st: SystemTime) -> String {
        let dt: DateTime<Utc> = st.into();

        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    #[test]
    fn test_read_from_time() {
        let data = "2026-03-28 09:42:01\tfarine a\n2026-03-28 09:42:02\tfarine b\n2026-03-28 09:42:03\tfarine c\n";
        let cursor = Cursor::new(data.as_bytes());

        let mut reader = LogReader::new(cursor).unwrap();

        assert!(reader.seek_last_time(parse_utc_to_system_time("2026-03-28 09:42:03").unwrap()));
        let results = reader.read(1).unwrap();

        assert_eq!(
            system_time_to_utc_string(results[0].time),
            "2026-03-28 09:42:02"
        );
    }

    #[test]
    fn test_read_backwards_simple() {
        let data = "2026-03-28 09:42:01\tfarine a\n2026-03-28 09:42:02\tfarine b\n2026-03-28 09:42:03\tfarine c\n";
        let cursor = Cursor::new(data.as_bytes());

        let mut reader = LogReader::new(cursor).unwrap();

        let results = reader.read(2).unwrap();
        assert_eq!(
            system_time_to_utc_string(results[0].time),
            "2026-03-28 09:42:03"
        );
        assert_eq!(
            system_time_to_utc_string(results[1].time),
            "2026-03-28 09:42:02"
        );

        reader.seek_pos = data.len() as u64;

        let results = reader.read(1).unwrap();
        assert_eq!(
            system_time_to_utc_string(results[0].time),
            "2026-03-28 09:42:03"
        );
        let results = reader.read(1).unwrap();

        assert_eq!(
            system_time_to_utc_string(results[0].time),
            "2026-03-28 09:42:02"
        );
    }

    #[test]
    fn test_iter_empty() {
        let cursor = Cursor::new(b"");
        let mut reader = LogReader::new(cursor).unwrap();
        assert!(reader.iter().next().is_none());
    }

    #[test]
    fn test_iter_single_message() {
        let data = "2026-03-28 09:42:01\tfarine a\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut reader = LogReader::new(cursor).unwrap();
        let mut iter = reader.iter();

        let msg = iter.next().unwrap().unwrap();
        assert_eq!(
            system_time_to_utc_string(msg.get_message().time),
            "2026-03-28 09:42:01"
        );
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iter_order_newest_first() {
        // The iterator reads backwards: newest message comes out first.
        let data = "2026-03-28 09:42:01\tfarine a\n2026-03-28 09:42:02\tfarine b\n2026-03-28 09:42:03\tfarine c\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut reader = LogReader::new(cursor).unwrap();

        let messages: Vec<_> = reader.iter().map(|r| r.unwrap()).collect();

        assert_eq!(messages.len(), 3);
        assert_eq!(
            system_time_to_utc_string(messages[0].get_message().time),
            "2026-03-28 09:42:03"
        );
        assert_eq!(
            system_time_to_utc_string(messages[1].get_message().time),
            "2026-03-28 09:42:02"
        );
        assert_eq!(
            system_time_to_utc_string(messages[2].get_message().time),
            "2026-03-28 09:42:01"
        );
    }

    #[test]
    fn test_iter_offsets() {
        // Each line is 29 bytes. The offset stored is the byte position of the
        // preceding '\n' (or 0 for the first line, which goes through the carry path).
        let data = "2026-03-28 09:42:01\tfarine a\n2026-03-28 09:42:02\tfarine b\n2026-03-28 09:42:03\tfarine c\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut reader = LogReader::new(cursor).unwrap();

        let messages: Vec<_> = reader.iter().map(|r| r.unwrap()).collect();

        assert_eq!(messages[0].get_offset(), 57); // '\n' after line 2
        assert_eq!(messages[1].get_offset(), 28); // '\n' after line 1
        assert_eq!(messages[2].get_offset(), 0); // first line via carry
    }

    #[test]
    fn test_iter_across_chunk_boundary() {
        // Build enough data to span multiple 1024-byte read chunks.
        // Each line is 29 bytes; 40 lines = 1160 bytes > 1024.
        let mut data = String::new();
        for i in 0..40u32 {
            data.push_str(&format!(
                "2026-03-28 09:{:02}:{:02}\tfarine x\n",
                i / 60,
                i % 60
            ));
        }
        let cursor = Cursor::new(data.as_bytes().to_vec());
        let mut reader = LogReader::new(cursor).unwrap();

        let messages: Vec<_> = reader.iter().map(|r| r.unwrap()).collect();

        assert_eq!(messages.len(), 40);
        // Newest line (i=39) comes first.
        assert_eq!(
            system_time_to_utc_string(messages[0].get_message().time),
            "2026-03-28 09:00:39"
        );
        // Oldest line (i=0) comes last.
        assert_eq!(
            system_time_to_utc_string(messages[39].get_message().time),
            "2026-03-28 09:00:00"
        );
    }

    #[test]
    fn test_iter_after_seek() {
        // Seeking before iterating should exclude messages after the seek point.
        let data = "2026-03-28 09:42:01\tfarine a\n2026-03-28 09:42:02\tfarine b\n2026-03-28 09:42:03\tfarine c\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut reader = LogReader::new(cursor).unwrap();

        reader.seek_last_time(parse_utc_to_system_time("2026-03-28 09:42:03").unwrap());

        let messages: Vec<_> = reader.iter().map(|r| r.unwrap()).collect();

        assert_eq!(messages.len(), 2);
        assert_eq!(
            system_time_to_utc_string(messages[0].get_message().time),
            "2026-03-28 09:42:02"
        );
        assert_eq!(
            system_time_to_utc_string(messages[1].get_message().time),
            "2026-03-28 09:42:01"
        );
    }
}
