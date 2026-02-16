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
        if let Some(server) = self.config.servers.first_mut() {
            server.login.nickname = nickname
        }
    }
}

pub struct Model {
    pub running_state: RunningState,
    stored_config: StoredConfig,
    pub irc_connection: Option<IRCConnection>,
    pub retry: u8,

    pub logger: MessageLogger,
    pub irc_model: Option<IrcModel>,
}

impl Model {
    pub fn new(config_name: String) -> Self {
        let config = Config::new(&config_name);

        let log_dir = ProjectPath::log_dir()
            .unwrap_or(std::env::current_dir().unwrap_or(std::path::Path::new("").to_path_buf()));
        Self {
            running_state: RunningState::Start,
            irc_model: None,
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

    pub fn set_nickname(&mut self, nickname: String) -> color_eyre::Result<()> {
        self.stored_config.set_nickname(nickname.to_string());
        self.save()?;
        Ok(())
    }

    pub fn get_nickname(&self) -> Option<&str> {
        self.get_config().get_nickname()
    }

    pub fn get_channel(&self) -> Option<&str> {
        self.get_config().get_channel()
    }

    pub fn get_address(&self) -> Option<&str> {
        self.get_config().get_address()
    }

    pub fn is_autojoin(&self) -> bool {
        self.get_config().is_autojoin()
    }

    pub fn get_connection_config(&self) -> Option<ConnectionConfig> {
        self.get_config().get_connection_config()
    }

    pub fn get_login_config(&self) -> Option<LoginConfig> {
        self.get_config().get_login_config()
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

    pub fn is_connected(&self) -> bool {
        self.irc_connection.is_some()
    }

    pub fn log(&mut self, message: &MessageEvent) -> color_eyre::Result<()> {
        if let Some(connection_config) = self.get_connection_config()
            && let Some(irc) = &self.irc_model
        {
            self.logger
                .write_message(&connection_config.address, irc, message)
        } else {
            Ok(())
        }
    }

    pub fn flush_log(&mut self) -> std::io::Result<()> {
        self.logger.flush_checker()
    }

    pub fn init_irc_model(&mut self) {
        if let Some(nick) = self.get_nickname()
            && let Some(channel) = self.get_channel()
        {
            self.irc_model = Some(IrcModel::new_model(nick.to_string(), channel.to_string()));
        }
    }
}
