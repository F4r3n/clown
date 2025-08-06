use clown_core::command::Command;
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

pub struct Model {
    pub running_state: RunningState,
    pub current_view: View,
    pub config: Config,

    pub message_reciever: Option<clown_core::message::MessageReceiver>,
    pub command_sender: Option<clown_core::outgoing::CommandSender>,

    pub task: Option<JoinHandle<()>>,

    pub current_channel: String,

    //How to manage errors?
    pub error_receiver: Option<mpsc::UnboundedReceiver<String>>,
    pub error_sender: Option<mpsc::UnboundedSender<String>>,
}

impl Model {
    pub fn new() -> Self {
        let config = Config::new();
        let channel = config.login_config.channel.to_string();
        Self {
            running_state: RunningState::Start,
            current_view: View::MainView,
            config,
            message_reciever: None,
            command_sender: None,
            task: None,
            current_channel: channel,
            error_receiver: None,
            error_sender: None,
        }
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        self.config.save()
    }

    pub fn send_command(&mut self, in_command: Command) {
        self.command_sender
            .as_mut()
            .map(|value| value.send(in_command));
    }

    pub fn is_irc_finished(&self) -> bool {
        self.task.as_ref().map(|v| v.is_finished()).unwrap_or(true)
    }
}
