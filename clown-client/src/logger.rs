use std::fs::OpenOptions;
use std::{fs::File, io::Write};

use anyhow::anyhow;
use once_cell::sync::OnceCell;
use std::sync::Mutex;
pub static LOGGER: OnceCell<Mutex<Logger>> = OnceCell::new();

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn try_new(path: &str) -> anyhow::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file })
    }

    pub fn info(&mut self, in_info: &str) -> anyhow::Result<()> {
        self.file.write_all(in_info.as_bytes())?;
        self.file.flush()?;
        Ok(())
    }
}

pub fn log_info_sync(msg: &str) {
    let logger_mutex = LOGGER.get_or_init(|| {
        Mutex::new(Logger::try_new("test_log.txt").expect("Failed to open log file"))
    });

    if let Ok(mut logger) = logger_mutex.lock().map_err(|_e| anyhow!("Error lock")) {
        let _ = logger.info(msg);
    }
}

pub async fn log_info_async(msg: &str) {
    let message = msg.to_string();
    let _ = tokio::task::spawn_blocking(move || log_info_sync(&message)).await;
}
