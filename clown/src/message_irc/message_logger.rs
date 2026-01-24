use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::message_event::MessageEvent;
use ahash::AHashMap;

pub struct MessageLogger {
    folder: PathBuf,
    writers: ahash::AHashMap<String, std::io::BufWriter<std::fs::File>>,
}

impl MessageLogger {
    pub fn new(folder: PathBuf) -> Self {
        Self {
            folder,
            writers: AHashMap::new(),
        }
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

    fn init_buffer(
        &mut self,
        server_addres: &str,
        target: Option<&str>,
    ) -> color_eyre::Result<&mut std::io::BufWriter<std::fs::File>> {
        let name = format!(
            "{}.{}.log",
            server_addres.to_lowercase(),
            target.unwrap_or("server")
        );

        let logger = match self.writers.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                let logger = Self::init_writer(&self.folder.join(name))?;
                v.insert(logger)
            }
        };

        Ok(logger)
    }

    fn get_current_time() -> String {
        let now = chrono::Local::now();
        now.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    pub fn write_message(
        &mut self,
        server_address: &str,
        message: &MessageEvent,
    ) -> color_eyre::Result<()> {
        let (target, data) = match message {
            MessageEvent::Join(channel, user, _) => {
                let data = user
                    .as_ref()
                    .map(|v| format!("-->\t {} has joined {}", v, channel));
                (Some(channel.as_str()), data)
            }

            MessageEvent::Part(channel, user, _) => {
                let data = Some(format!("<--\t {} has left {}", user, channel));
                (Some(channel.as_str()), data)
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
            let buffer = self.init_buffer(server_address, target)?;
            writeln!(buffer, "{}\t{}", Self::get_current_time(), data)?;
            // TODO: flush when full? or user if leaving
            buffer.flush()?;
        }

        Ok(())
    }
}
