use clown_core::{command::Command, message::ServerMessage};
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
    pub config: Config,
    pub current_channel: String,
    pub irc_connection: Option<IRCConnection>,
}

impl Model {
    pub fn new() -> Self {
        let config = Config::new();
        let channel = config.login_config.channel.to_string();
        Self {
            running_state: RunningState::Start,
            current_view: View::MainView,
            current_channel: channel,
            config,
            irc_connection: None,
        }
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        self.config.save()
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
