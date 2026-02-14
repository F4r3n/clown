use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::message_event::MessageEvent;
use ahash::AHashMap;
const LOG_FLUSH_TIMER_SECONDS: u64 = 5;

//Many filedesc can be opened without receiving any message
//Using a LRU should be more efficient, but no need for now
const LOG_OPENED_TIMER_MINUTES: u64 = 120;

struct Logger {
    duration: std::time::Instant,
    last_data_written: std::time::Instant,
    buffer: std::io::BufWriter<std::fs::File>,
}

impl Logger {
    pub fn try_from_path(path: &Path) -> color_eyre::Result<Logger> {
        Ok(Self {
            duration: std::time::Instant::now(),
            last_data_written: std::time::Instant::now(),
            buffer: Self::init_writer(path)?,
        })
    }

    fn init_writer(path: &Path) -> color_eyre::Result<BufWriter<std::fs::File>> {
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
        let now = chrono::Local::now();
        now.format("%Y-%m-%d %H:%M:%S")
    }
}

pub struct MessageLogger {
    folder: PathBuf,
    writers: ahash::AHashMap<String, Logger>,
}

#[derive(Debug, Hash, PartialEq)]
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

    fn hash_target(target: LogKey<'_>) -> u64 {
        let state = ahash::RandomState::with_seeds(1, 2, 3, 4);
        state.hash_one(target)
    }

    fn init_buffer(
        &mut self,
        server_address: &str,
        target: Option<&str>,
    ) -> color_eyre::Result<&mut Logger> {
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
                let logger = Logger::try_from_path(&self.folder.join(name))?;
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
        data: impl std::fmt::Display,
        force_flush: bool,
    ) -> color_eyre::Result<()> {
        let logger = self.init_buffer(server_address, target)?;
        logger.write(data)?;
        logger.flush(force_flush)?;

        Ok(())
    }

    pub fn write_message(
        &mut self,
        server_address: &str,
        irc_model: &crate::irc_view::irc_model::IrcModel,
        message: &MessageEvent,
    ) -> color_eyre::Result<()> {
        match message {
            MessageEvent::Join(channel, user) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    format_args!("-->\t {} has joined {}", user, channel),
                    false,
                )?;
            }
            MessageEvent::ReplaceUser(old, new) => {
                for channel in irc_model.get_all_joined_channel(old) {
                    self.write_to_target(
                        server_address,
                        Some(channel),
                        format_args!("<--\t {} has changed their nickname to {}", &old, &new),
                        true,
                    )?;
                }
            }
            MessageEvent::Part(channel, user) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    format_args!("<--\t {} has left {}", user, channel),
                    true,
                )?;
            }
            MessageEvent::Quit(user, _) => {
                for channel in irc_model.get_all_joined_channel(user) {
                    self.write_to_target(
                        server_address,
                        Some(channel),
                        format_args!("<--\t {} has quit", user),
                        true,
                    )?;
                }
            }
            MessageEvent::SetTopic(Some(source), channel, content) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    format_args!(
                        "--\t {} has changed topic for {} to \"{}\"",
                        source, channel, content
                    ),
                    false,
                )?;
            }
            MessageEvent::PrivMsg(source, target, content) => {
                self.write_to_target(
                    server_address,
                    Some(target),
                    format_args!("{} {}", source, content),
                    false,
                )?;
            }

            MessageEvent::ActionMsg(source, target, content) => {
                self.write_to_target(
                    server_address,
                    Some(target),
                    format_args!("* {} {}", source, content),
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

        let mut logger = Logger::try_from_path(&log_path).unwrap();
        logger.write("Hello, Rust!").unwrap();
        logger.flush(true).unwrap(); // Force flush to ensure it hits the disk

        let content = fs::read_to_string(log_path).unwrap();
        assert!(content.contains("Hello, Rust!"));
        // Check for timestamp format (YYYY-MM-DD)
        assert!(content.contains(&chrono::Local::now().format("%Y-%m-%d").to_string()));
    }
}
