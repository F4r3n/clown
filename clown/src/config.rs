use std::collections;
use std::path::PathBuf;

use clown_core::client::LoginConfig;
use clown_core::conn::ConnectionConfig;

use crate::project_path::ProjectPath;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Connection {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Login {
    pub nickname: String,
    pub real_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Channels {
    pub list: Vec<String>,
    pub auto_join: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Server {
    pub connection: Connection,
    pub login: Login,
    pub channels: Channels,
}

//
// COMPLETION
//

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Completion {
    #[serde(default)]
    pub on_empty_input: CompletionBehavior,

    #[serde(default)]
    pub in_message: CompletionBehavior,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct CompletionBehavior {
    #[serde(default)]
    pub suffix: Option<String>,
}

//
// COLORS
//

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Colors {
    #[serde(default)]
    pub nickname: Nickname,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Nickname {
    #[serde(default)]
    pub seed: Vec<String>,

    #[serde(default)]
    pub overrides: collections::hash_map::HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Topic {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for Topic {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Discuss {
    #[serde(default)]
    pub left_bar: LeftBar,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LeftBar {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for LeftBar {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Users {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for Users {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_true() -> bool {
    true
}

//
// KEYBINDINGS
//

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Keybindings {
    #[serde(default)]
    pub bind: Vec<Keybinding>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keybinding {
    pub action: Action,
    pub keys: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Meta {
    pub version: u16,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub servers: Vec<Server>,
    pub completion: Completion,
    pub colors: Colors,
    pub discuss: Discuss,
    pub users: Users,
    pub topic: Topic,
    pub keybindings: Keybindings,
    pub meta: Meta,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            servers: vec![],
            colors: Colors::default(),
            completion: Completion::default(),
            keybindings: Keybindings::default(),
            discuss: Discuss::default(),
            users: Users::default(),
            topic: Topic::default(),
            meta: Meta { version: 0 },
        }
    }
}

impl Config {
    pub fn new(config_name: &str) -> Self {
        Self::read(config_name).unwrap_or_default()
    }

    pub fn save(&self, config_name: &str) -> color_eyre::Result<()> {
        let result = toml::to_string(self)?;

        let config_path =
            Self::config_path(config_name).ok_or(color_eyre::eyre::Error::msg("Invalid Path"))?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_path, result)?;
        Ok(())
    }

    fn config_path(config_name: &str) -> Option<PathBuf> {
        ProjectPath::project_dir().map(|proj_dirs| proj_dirs.config_dir().join(config_name))
    }

    fn read(config_name: &str) -> color_eyre::Result<Self> {
        if let Some(config_path) = Self::config_path(config_name) {
            let content = std::fs::read_to_string(config_path)?;
            if let Ok(config) = toml::from_str::<Config>(&content) {
                Ok(config)
            } else {
                Ok(Self::default())
            }
        } else {
            Err(color_eyre::eyre::eyre!("Invalid config"))
        }
    }

    pub fn get_nickname(&self) -> Option<&str> {
        self.servers.first().map(|v| v.login.nickname.as_str())
    }

    pub fn get_channel(&self) -> Option<&str> {
        self.servers
            .first()
            .and_then(|v| v.channels.list.first().map(|v| v.as_str()))
    }

    pub fn get_address(&self) -> Option<&str> {
        self.servers.first().map(|v| v.connection.address.as_str())
    }

    pub fn is_autojoin(&self) -> bool {
        self.servers.first().map_or(false, |v| v.channels.auto_join)
    }

    pub fn get_connection_config(&self) -> Option<ConnectionConfig> {
        self.servers.first().map(|v| ConnectionConfig {
            address: v.connection.address.to_string(),
            port: v.connection.port,
        })
    }

    pub fn get_login_config(&self) -> Option<LoginConfig> {
        self.servers.first().map(|v| LoginConfig {
            nickname: v.login.nickname.clone(),
            password: v.login.password.clone(),
            real_name: v.login.real_name.clone(),
            username: v.login.username.clone(),
        })
    }
}
