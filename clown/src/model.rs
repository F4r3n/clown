use crate::irc_view::irc_model::IrcModel;
use crate::message_irc::message_logger::MessageLogger;
use crate::project_path::ProjectPath;
use crate::{config::Config, message_event::MessageEvent};
use clown_core::{
    client::LoginConfig, command::Command, conn::ConnectionConfig, message::ServerMessage,
};
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
        self.config.login_config.nickname = nickname;
    }
}

pub struct Model {
    pub running_state: RunningState,
    stored_config: StoredConfig,
    pub irc_connection: Option<IRCConnection>,
    pub retry: u8,

    pub logger: MessageLogger,
    pub irc_model: IrcModel,
}

impl Model {
    pub fn new(config_name: String) -> Self {
        let config = Config::new(&config_name);
        let channel = config.login_config.channel.to_string();
        let log_dir = ProjectPath::log_dir()
            .unwrap_or(std::env::current_dir().unwrap_or(std::path::Path::new("").to_path_buf()));
        Self {
            running_state: RunningState::Start,
            irc_model: IrcModel::new_model(
                config.login_config.nickname.to_string(),
                channel.to_string(),
            ),
            stored_config: StoredConfig {
                config,
                stored_name: config_name,
            },
            irc_connection: None,
            logger: MessageLogger::new(log_dir),
            retry: 5,
        }
    }

    fn get_config(&self) -> &Config {
        &self.stored_config.config
    }

    pub fn reset_retry(&mut self) {
        self.retry = 5;
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        self.stored_config.save()
    }

    pub fn set_nickname(&mut self, nickname: String) -> color_eyre::Result<&str> {
        self.stored_config.set_nickname(nickname);

        self.save()?;
        Ok(&self.get_config().login_config.nickname)
    }

    pub fn get_nickname(&self) -> &str {
        &self.get_config().login_config.nickname
    }

    pub fn get_login_channel(&self) -> &str {
        &self.get_config().login_config.channel
    }

    pub fn get_address(&self) -> Option<&str> {
        self.get_config()
            .connection_config
            .as_ref()
            .map(|v| v.address.as_ref())
    }

    pub fn is_autojoin(&self) -> bool {
        self.get_config().client_config.auto_join
    }

    pub fn get_connection_config(&self) -> Option<ConnectionConfig> {
        self.get_config().connection_config.clone()
    }

    pub fn get_login_config(&self) -> LoginConfig {
        self.get_config().login_config.clone()
    }

    pub fn send_command(&mut self, in_command: Command) {
        self.irc_connection
            .as_mut()
            .map(|value| value.command_sender.send(in_command));
    }

    pub fn is_irc_finished(&self) -> bool {
        self.irc_connection
            .as_ref()
            .map(|v| v.task.is_finished())
            .unwrap_or(true)
    }

    pub fn pull_server_message(&mut self) -> Option<ServerMessage> {
        self.irc_connection
            .as_mut()
            .and_then(|v| v.message_reciever.inner.try_recv().ok())
    }

    pub fn pull_server_error(&mut self) -> Option<String> {
        self.irc_connection
            .as_mut()
            .and_then(|v| v.error_receiver.try_recv().ok())
    }

    pub fn log(&mut self, message: &MessageEvent) -> color_eyre::Result<()> {
        if let Some(connection_config) = self.get_connection_config() {
            self.logger
                .write_message(&connection_config.address, &self.irc_model, message)
        } else {
            Err(color_eyre::eyre::eyre!("No address set"))
        }
    }

    pub fn flush_log(&mut self) -> std::io::Result<()> {
        self.logger.flush_checker()
    }
}
