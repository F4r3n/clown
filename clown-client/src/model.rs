use std::io::Write;

use clown_core;
use clown_core::client;
use clown_core::command::Command;
#[derive(Default, Debug, PartialEq, Eq, Hash)]
pub enum View {
    #[default]
    MainView,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum RunningState {
    #[default]
    Running,
    Done,
}

pub struct Model {
    pub running_state: RunningState,
    pub current_view: View,
    pub connection_config: Option<clown_core::conn::ConnectionConfig>,
    pub irc_config: Option<clown_core::client::IRCConfig>,

    pub message_reciever: Option<clown_core::message::MessageReceiver>,
    pub command_sender: Option<clown_core::outgoing::CommandSender>,
    pub logger: Option<std::fs::File>,
}

impl Model {
    pub fn new(
        connection_config: Option<clown_core::conn::ConnectionConfig>,
        irc_config: Option<clown_core::client::IRCConfig>,
    ) -> Self {
        Self {
            running_state: RunningState::Running,
            current_view: View::MainView,
            connection_config,
            irc_config,
            message_reciever: None,
            command_sender: None,
            logger: std::fs::File::create("debug.txt").ok(),
        }
    }

    pub fn send_command(&mut self, in_command: Command) {
        self.command_sender
            .as_mut()
            .map(|value| value.send(in_command));
    }

    pub fn log(&mut self, in_content: &str) {
        if let Some(file) = self.logger.as_mut() {
            file.write_all(in_content.as_bytes());
            file.flush();
        }
    }
}
