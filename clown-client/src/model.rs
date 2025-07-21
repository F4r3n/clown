use clown_core::command::Command;
use tokio::task::JoinHandle;
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

    pub task : Option<JoinHandle<anyhow::Result<()>>>
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
            task: None,
        }
    }

    pub fn send_command(&mut self, in_command: Command) {
        self.command_sender
            .as_mut()
            .map(|value| value.send(in_command));
    }

    pub fn is_irc_finished(&self) -> bool {
        self.task.as_ref().map(|v|v.is_finished()).unwrap_or(true)
    }
}
