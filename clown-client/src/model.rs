use clown_core;

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
    pub config: clown_core::conn::ConnectionConfig,
}

impl Model {
    pub fn new(config: clown_core::conn::ConnectionConfig) -> Self {
        Self {
            running_state: RunningState::Running,
            current_view: View::MainView,
            config,
        }
    }
}
