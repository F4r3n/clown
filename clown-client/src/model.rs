use clown_core::command::Command;
use tokio::task::JoinHandle;

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

    pub task: Option<JoinHandle<Result<(), color_eyre::Report>>>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            running_state: RunningState::Start,
            current_view: View::MainView,
            config: Config::new(),
            message_reciever: None,
            command_sender: None,
            task: None,
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
