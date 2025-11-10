use clown_core::{
    client::LoginConfig, command::Command, conn::ConnectionConfig, message::ServerMessage,
};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::config::Config;
#[derive(Default, Debug, PartialEq, Eq, Hash)]
pub enum View {
    #[default]
    MainView,
}

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
    pub error_receiver: mpsc::UnboundedReceiver<String>,
    pub _error_sender: mpsc::UnboundedSender<String>,
    pub task: JoinHandle<()>,
}

pub struct Model {
    pub running_state: RunningState,
    pub current_view: View,
    config: Config,
    pub current_channel: String,
    pub irc_connection: Option<IRCConnection>,
    pub retry: u8,
}

impl Model {
    pub fn new() -> Self {
        let config = Config::new();
        let channel = config.login_config.channel.to_string();
        Self {
            running_state: RunningState::Start,
            current_view: View::MainView,
            current_channel: channel.to_string(),
            config,
            irc_connection: None,
            retry: 5,
        }
    }

    pub fn reset_retry(&mut self) {
        self.retry = 5;
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        self.config.save()
    }

    pub fn set_nickname(&mut self, nickname: &str) -> color_eyre::Result<&str> {
        self.config.login_config.nickname = nickname.to_string();
        self.save()?;
        Ok(&self.config.login_config.nickname)
    }

    pub fn get_nickname(&self) -> &str {
        &self.config.login_config.nickname
    }

    pub fn get_login_channel(&self) -> &str {
        &self.config.login_config.channel
    }

    pub fn get_address(&self) -> Option<&str> {
        self.config
            .connection_config
            .as_ref()
            .map(|v| v.address.as_ref())
    }

    pub fn is_autojoin(&self) -> bool {
        self.config.client_config.auto_join
    }

    pub fn get_connection_config(&self) -> Option<ConnectionConfig> {
        self.config.connection_config.clone()
    }

    pub fn get_login_config(&self) -> LoginConfig {
        self.config.login_config.clone()
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
}
