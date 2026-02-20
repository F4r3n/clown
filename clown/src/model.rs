use crate::config::Config;
use clown_core::{client::LoginConfig, conn::ConnectionConfig};
use tokio::{sync::mpsc, task::JoinHandle};
#[derive(Debug, Default, PartialEq, Eq)]
pub enum RunningState {
    #[default]
    Start,
    Running,
    Done,
}

pub struct IRCConnection {
    pub message_reciever: clown_core::message::MessageReceiver,
    pub command_sender: clown_core::outgoing::CommandSender,
    pub error_receiver: mpsc::Receiver<String>,
    pub _error_sender: mpsc::Sender<String>,
    pub task: JoinHandle<()>,
}

pub struct StoredConfig {
    config: Config,
    stored_name: String,
}

impl StoredConfig {
    pub fn save(&self) -> color_eyre::Result<()> {
        self.config.save(&self.stored_name)
    }

    pub fn set_nickname(&mut self, nickname: String) {
        if let Some(server) = self.config.servers.first_mut() {
            server.login.nickname = nickname
        }
    }
}

pub struct Model {
    pub running_state: RunningState,
    stored_config: StoredConfig,
}

impl Model {
    pub fn new(config_name: String) -> Self {
        let config = Config::new(&config_name);

        Self {
            running_state: RunningState::Start,
            stored_config: StoredConfig {
                config,
                stored_name: config_name,
            },
        }
    }

    fn get_config(&self) -> &Config {
        &self.stored_config.config
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        self.stored_config.save()
    }

    pub fn set_nickname(&mut self, nickname: String) -> color_eyre::Result<()> {
        self.stored_config.set_nickname(nickname.to_string());
        self.save()?;
        Ok(())
    }

    pub fn get_nickname(&self, in_id: usize) -> Option<&str> {
        self.get_config().get_nickname(in_id)
    }

    pub fn get_address(&self, in_id: usize) -> Option<&str> {
        self.get_config().get_address(in_id)
    }

    pub fn is_autojoin_by_id(&self, in_id: usize) -> bool {
        self.get_config().is_autojoin_id(in_id)
    }

    pub fn is_autojoin(&self) -> impl Iterator<Item = usize> {
        self.get_config().is_autojoin()
    }

    pub fn get_connection_config(&self, in_id: usize) -> Option<ConnectionConfig> {
        self.get_config().get_connection_config(in_id)
    }

    pub fn get_login_config(&self, in_id: usize) -> Option<LoginConfig> {
        self.get_config().get_login_config(in_id)
    }

    pub fn get_login_channels(&self, in_id: usize) -> Option<impl Iterator<Item = &str>> {
        self.stored_config
            .config
            .servers
            .get(in_id)
            .map(|v| v.channels.list.iter().map(|v| v.as_str()))
    }

    pub fn get_channels(&mut self, in_id: usize) -> impl Iterator<Item = &str> {
        self.stored_config.config.get_channels(in_id)
    }
}
