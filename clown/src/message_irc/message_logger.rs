use crate::message_event::MessageEvent;
use crate::message_irc::log_parser;
use ahash::AHashMap;
use std::borrow::Cow;
use std::{
    io::{BufReader, BufWriter, Read, Seek, Write},
    path::{Path, PathBuf},
};
const LOG_FLUSH_TIMER_SECONDS: u64 = 5;

//Many filedesc can be opened without receiving any message
//Using a LRU should be more efficient, but no need for now
const LOG_OPENED_TIMER_MINUTES: u64 = 120;

#[derive(Debug)]
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

#[derive(Debug)]
pub struct LoggedTimedMessage<'a> {
    pub time: std::time::SystemTime,
    pub message: LoggedMessage<'a>,
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

struct LogReader {
    buffer: std::io::BufReader<std::fs::File>,
    seek_pos: i64, //starts from end
}

impl LogReader {
    fn init(path: &Path) -> anyhow::Result<BufReader<std::fs::File>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::options().read(true).open(path)?;
        Ok(BufReader::new(file))
    }

    pub fn try_from_path(path: &Path) -> anyhow::Result<LogReader> {
        Ok(Self {
            buffer: Self::init(path)?,
            seek_pos: 0,
        })
    }

    pub fn read(&mut self, number_lines: usize) -> anyhow::Result<Vec<LoggedTimedMessage<'_>>> {
        let mut vec = Vec::new();
        let mut start_pos = self.seek_pos;

        let mut chunk = [0; 1024];

        let mut to_read = number_lines;
        let mut carry: Vec<u8> = Vec::new();

        while to_read > 0 {
            start_pos = start_pos.saturating_sub(1024);

            let pos = std::io::SeekFrom::End(start_pos);
            self.buffer.seek(pos)?;
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
                        vec.push(log_parser::parse(line)?);
                        to_read = to_read.saturating_sub(1);

                        if to_read == 0 {
                            break;
                        }
                    }

                    start = i + 1;
                }
            }
            let consumed = (combined.len() - start) as i64;
            start_pos += consumed;
            carry = combined[..start].to_vec();
        }

        if to_read > 0 && !carry.is_empty() {
            vec.push(log_parser::parse(carry.as_slice())?);
        }
        self.seek_pos = start_pos;
        vec.reverse(); // restore correct order

        Ok(vec)
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

    fn init_buffer(
        &mut self,
        server_address: &str,
        target: Option<&str>,
    ) -> anyhow::Result<&mut LogWriter> {
        //The name is not sanitized because is only used as a key to a hashmap
        let target = target.unwrap_or("server");
        let name = format!("{}.{}.log", server_address, target);

        let logger = match self.writers.entry(name) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                //If new key, sanitize input
                let name = format!(
                    "{}.{}.{}.log",
                    Self::sanitize_path(server_address),
                    Self::sanitize_path(target),
                    // if someone is called foo/bar and foo_bar the same file will be used
                    // Use a hash to be more precise
                    Self::hash_target(LogKey {
                        server_address,
                        target
                    })
                );
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
        irc_model: Option<&crate::irc_view::irc_model::IrcModel>,
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

            MessageEvent::Notice(_, source, target, content)
            | MessageEvent::PrivMsg(_, source, target, content) => {
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
    use std::fs;
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
}
