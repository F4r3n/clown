use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::message_event::MessageEvent;
use ahash::AHashMap;
const LOG_FLUSH_TIMER_SECONDS: u64 = 5;

struct Logger {
    duration: std::time::Instant,
    buffer: std::io::BufWriter<std::fs::File>,
}

impl Logger {
    pub fn try_from_path(path: &Path) -> color_eyre::Result<Logger> {
        Ok(Self {
            duration: std::time::Instant::now(),
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

    fn flush(&mut self, force_flush: bool) -> color_eyre::Result<()> {
        if force_flush
            || (self.duration.elapsed() > std::time::Duration::from_secs(LOG_FLUSH_TIMER_SECONDS))
        {
            self.duration = std::time::Instant::now();
            self.buffer.flush()?;
        }
        Ok(())
    }

    fn write(&mut self, data: &str) -> color_eyre::Result<()> {
        writeln!(self.buffer, "{}\t{}", Self::get_current_time(), data)?;

        Ok(())
    }

    fn get_current_time() -> String {
        let now = chrono::Local::now();
        now.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

pub struct MessageLogger {
    folder: PathBuf,
    writers: ahash::AHashMap<String, Logger>,
}

impl MessageLogger {
    pub fn new(folder: PathBuf) -> Self {
        Self {
            folder,
            writers: AHashMap::new(),
        }
    }

    fn sanitize_path(word: &str) -> String {
        word.to_lowercase()
            .chars()
            .map(|v| match v {
                '\\' | '/' => '_',
                _ => v,
            })
            .collect::<String>()
    }

    fn init_buffer(
        &mut self,
        server_addres: &str,
        target: Option<&str>,
    ) -> color_eyre::Result<&mut Logger> {
        //The name is not sanitized because is only used as a key to a hashmap
        let name = format!("{}.{}.log", server_addres, target.unwrap_or("server"));

        let logger = match self.writers.entry(name) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                //If new key, sanitize input
                let name = format!(
                    "{}.{}.log",
                    Self::sanitize_path(server_addres),
                    Self::sanitize_path(target.unwrap_or("server"))
                );
                let logger = Logger::try_from_path(&self.folder.join(name))?;
                v.insert(logger)
            }
        };

        Ok(logger)
    }

    pub fn flush_checker(&mut self) -> color_eyre::Result<()> {
        for (_, logger) in self.writers.iter_mut() {
            logger.flush(false)?;
        }
        Ok(())
    }

    fn write_to_target(
        &mut self,
        server_address: &str,
        target: Option<&str>,
        data: &str,
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
        message: &MessageEvent,
    ) -> color_eyre::Result<()> {
        match message {
            MessageEvent::Join(channel, Some(user), _) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    format!("-->\t {} has joined {}", user, channel).as_str(),
                    false,
                )?;
            }

            MessageEvent::Part(channel, user, _) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    format!("<--\t {} has left {}", user, channel).as_str(),
                    true,
                )?;
            }
            MessageEvent::QuitChannels(channels, user, _) => {
                for channel in channels {
                    self.write_to_target(
                        server_address,
                        Some(channel),
                        format!("<--\t {} has quit", user).as_str(),
                        true,
                    )?;
                }
            }
            MessageEvent::SetTopic(Some(source), channel, content) => {
                self.write_to_target(
                    server_address,
                    Some(channel),
                    format!(
                        "--\t {} has changed topic for {} to \"{}\"",
                        source, channel, content
                    )
                    .as_str(),
                    false,
                )?;
            }
            MessageEvent::PrivMsg(Some(source), target, content) => {
                self.write_to_target(
                    server_address,
                    Some(target),
                    format!("{} {}", source, content).as_str(),
                    false,
                )?;
            }

            MessageEvent::ActionMsg(source, target, content) => {
                self.write_to_target(
                    server_address,
                    Some(target),
                    format!("* {} {}", source, content).as_str(),
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
