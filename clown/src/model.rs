use crate::{
    config::{Config, Discuss},
    irc_view::color_user::ColorGenerator,
};
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

#[derive(Default)]
pub struct StoredConfig {
    config: Config,
    stored_name: String,
}

impl StoredConfig {
    pub fn save(&self) -> color_eyre::Result<()> {
        self.config.save(&self.stored_name)
    }

    pub fn set_nickname(&mut self, server_id: usize, nickname: String) {
        if let Some(server) = self.config.servers.get_mut(server_id) {
            server.login.nickname = nickname
        }
    }

    pub fn set_value(&mut self, path: &str, value: String) -> color_eyre::eyre::Result<()> {
        match self.config.set_value_from_root(path, value) {
            Ok(()) => self.save(),
            Err(e) => Err(e),
        }
    }

    pub fn get_value(
        &mut self,
        path: &str,
        option: Option<&str>,
    ) -> color_eyre::eyre::Result<String> {
        self.config.get_value_from_root(path, option)
    }

    pub fn list_fields() -> Vec<String> {
        Config::list_fields()
    }
}

pub struct Model {
    pub running_state: RunningState,
    stored_config: StoredConfig,

    color_generator: crate::irc_view::color_user::ColorGenerator,
}

impl Model {
    pub fn new_empty_config() -> Self {
        Self {
            running_state: RunningState::Start,
            stored_config: StoredConfig::default(),
            color_generator: ColorGenerator::new(0),
        }
    }

    fn load_color(config: &Config) -> ColorGenerator {
        let mut color_generator = ColorGenerator::new(config.nickname_colors.seed);
        for (input, color) in &config.nickname_colors.overrides {
            color_generator.add_override(input.to_string(), color);
        }
        color_generator
    }

    pub fn new(config_name: String) -> color_eyre::Result<Self> {
        let config = Config::new(&config_name)?;

        Ok(Self {
            running_state: RunningState::Start,
            color_generator: Self::load_color(&config),
            stored_config: StoredConfig {
                config,
                stored_name: config_name,
            },
        })
    }

    fn get_config(&self) -> &Config {
        &self.stored_config.config
    }

    pub fn list_fields_config() -> Vec<String> {
        StoredConfig::list_fields()
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        self.stored_config.save()
    }

    pub fn set_nickname(&mut self, server_id: usize, nickname: String) -> color_eyre::Result<()> {
        self.stored_config
            .set_nickname(server_id, nickname.to_string());
        self.save()?;
        Ok(())
    }

    pub fn get_nickname(&self, in_id: usize) -> Option<&str> {
        self.get_config().get_nickname(in_id)
    }

    pub fn get_address(&self, in_id: usize) -> Option<&str> {
        self.get_config().get_address(in_id)
    }

    pub fn get_completion_behaviour(&self) -> (Option<&str>, Option<&str>) {
        (
            self.get_config().completion.in_message.suffix.as_deref(),
            self.get_config()
                .completion
                .on_empty_input
                .suffix
                .as_deref(),
        )
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

    pub fn get_channels(&mut self, in_id: usize) -> impl Iterator<Item = &str> {
        self.stored_config.config.get_channels(in_id)
    }

    pub fn get_server_count(&self) -> usize {
        self.stored_config.config.servers.len()
    }

    pub fn get_color(&self, input: &str) -> ratatui::style::Color {
        self.color_generator.generate_color(input)
    }

    pub fn is_topic_ui_enabled(&self) -> bool {
        self.stored_config.config.topic.enabled
    }

    pub fn is_users_ui_enabled(&self) -> bool {
        self.stored_config.config.users.enabled
    }

    pub fn get_discuss_config(&self) -> &Discuss {
        &self.stored_config.config.discuss
    }

    pub fn set_config_value(&mut self, path: &str, value: String) -> color_eyre::eyre::Result<()> {
        match self.stored_config.set_value(path, value) {
            Ok(()) => {
                //TODO: be more granular
                self.color_generator = Self::load_color(&self.stored_config.config);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_config_value(
        &mut self,
        path: &str,
        option: Option<&str>,
    ) -> color_eyre::eyre::Result<String> {
        self.stored_config.get_value(path, option)
    }
}
