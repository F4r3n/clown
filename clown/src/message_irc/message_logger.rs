use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::message_event::MessageEvent;
use ahash::AHashMap;

struct Logger {
    duration: std::time::Instant,
    number_messages: usize,
    buffer: std::io::BufWriter<std::fs::File>,
}

impl Logger {
    pub fn try_from_path(path: &Path) -> color_eyre::Result<Logger> {
        Ok(Self {
            duration: std::time::Instant::now(),
            number_messages: 0,
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
            || self.number_messages > 5
            || (self.duration.elapsed() > std::time::Duration::from_secs(10))
        {
            self.number_messages = 0;
            self.duration = std::time::Instant::now();
            self.buffer.flush()?;
        }
        Ok(())
    }

    fn write(&mut self, data: &str) -> color_eyre::Result<()> {
        self.number_messages = self.number_messages.saturating_add(1);
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

    fn init_buffer(
        &mut self,
        server_addres: &str,
        target: Option<&str>,
    ) -> color_eyre::Result<&mut Logger> {
        let name = format!(
            "{}.{}.log",
            server_addres.to_lowercase(),
            target
                .unwrap_or("server")
                .replace(std::path::MAIN_SEPARATOR, "_")
        );

        let logger = match self.writers.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                let logger = Logger::try_from_path(&self.folder.join(name))?;
                v.insert(logger)
            }
        };

        Ok(logger)
    }

    pub fn write_message(
        &mut self,
        server_address: &str,
        message: &MessageEvent,
    ) -> color_eyre::Result<()> {
        let mut force_flush = false;
        let (target, data) = match message {
            MessageEvent::Join(channel, user, _) => {
                let data = user
                    .as_ref()
                    .map(|v| format!("-->\t {} has joined {}", v, channel));
                (Some(channel.as_str()), data)
            }

            MessageEvent::Part(channel, user, _) => {
                force_flush = true;
                let data = Some(format!("<--\t {} has left {}", user, channel));
                (Some(channel.as_str()), data)
            }
            MessageEvent::Quit(user, _) => {
                force_flush = true;
                let data = Some(format!("<--\t {} has quit", user));
                (None, data)
            }
            MessageEvent::SetTopic(source, channel, content) => {
                if let Some(source) = source {
                    let data = Some(format!(
                        "--\t {} has changed topic for {} to \"{}\"",
                        source, channel, content
                    ));
                    (Some(channel.as_str()), data)
                } else {
                    (None, None)
                }
            }
            MessageEvent::PrivMsg(source, target, content) => {
                let data = source.as_ref().map(|v| format!("{} {}", v, content));
                (Some(target.as_str()), data)
            }

            MessageEvent::ActionMsg(source, target, content) => {
                let data = Some(format!("* {} {}", source, content));
                (Some(target.as_str()), data)
            }
            _ => (None, None),
        };

        if let Some(data) = data {
            let logger = self.init_buffer(server_address, target)?;
            logger.write(&data)?;
            logger.flush(force_flush)?;
        }

        Ok(())
    }
}
