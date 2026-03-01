use std::ops::Deref;
use std::path::PathBuf;

use clown_core::client::LoginConfig;
use clown_core::conn::ConnectionConfig;

use crate::project_path::ProjectPath;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
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
    pub name: String,
    pub connection: Connection,
    pub login: Login,
    pub channels: Channels,
}

//
// COMPLETION
//

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct Completion {
    #[serde(default)]
    pub on_empty_input: CompletionBehavior,

    #[serde(default)]
    pub in_message: CompletionBehavior,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct CompletionBehavior {
    #[serde(default)]
    pub suffix: Option<String>,
}

//
// COLORS
//

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct NicknameColors {
    #[serde(default)]
    pub seed: u64,

    #[serde(default)]
    pub overrides: ahash::HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Topic {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for Topic {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct Discuss {
    #[serde(default)]
    pub left_bar: LeftBar,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LeftBar {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for LeftBar {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct Keybindings {
    #[serde(default)]
    pub bind: Vec<Keybinding>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Keybinding {
    pub action: Action,
    pub keys: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct Meta {
    pub version: u16,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub servers: Vec<Server>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub completion: Completion,

    #[serde(default, skip_serializing_if = "is_default")]
    pub nickname_colors: NicknameColors,

    #[serde(default, skip_serializing_if = "is_default")]
    pub discuss: Discuss,

    #[serde(default, skip_serializing_if = "is_default")]
    pub users: Users,

    #[serde(default, skip_serializing_if = "is_default")]
    pub topic: Topic,

    #[serde(default, skip_serializing_if = "is_default")]
    pub keybindings: Keybindings,

    #[serde(default, skip_serializing_if = "is_default")]
    pub meta: Meta,
}

fn is_default<T>(t: &T) -> bool
where
    T: Default + PartialEq,
{
    t == &T::default()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            servers: vec![Server {
                connection: Connection {
                    address: "".into(),
                    port: 6697,
                },
                channels: Channels {
                    list: vec![],
                    auto_join: false,
                },
                login: Login {
                    nickname: "nickname".into(),
                    real_name: None,
                    username: None,
                    password: None,
                },
                name: "IRC-Server".into(),
            }],
            nickname_colors: NicknameColors::default(),
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

    pub fn get_nickname(&self, in_id: usize) -> Option<&str> {
        self.servers.get(in_id).map(|v| v.login.nickname.as_str())
    }

    pub fn get_channels(&self, in_id: usize) -> impl Iterator<Item = &str> {
        self.servers
            .get(in_id)
            .into_iter()
            .flat_map(|v| v.channels.list.iter().map(|v| v.deref()))
    }

    pub fn get_address(&self, in_id: usize) -> Option<&str> {
        self.servers
            .get(in_id)
            .map(|v| v.connection.address.as_str())
    }

    pub fn is_autojoin_id(&self, in_id: usize) -> bool {
        self.servers
            .get(in_id)
            .is_some_and(|v| v.channels.auto_join)
    }

    pub fn is_autojoin(&self) -> impl Iterator<Item = usize> {
        self.servers
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.channels.auto_join.then_some(i))
    }

    pub fn get_connection_config(&self, in_id: usize) -> Option<ConnectionConfig> {
        self.servers.get(in_id).map(|v| ConnectionConfig {
            address: v.connection.address.to_string(),
            port: v.connection.port,
        })
    }

    pub fn get_login_config(&self, in_id: usize) -> Option<LoginConfig> {
        self.servers.get(in_id).map(|v| LoginConfig {
            nickname: v.login.nickname.clone(),
            password: v.login.password.clone(),
            real_name: v.login.real_name.clone(),
            username: v.login.username.clone(),
        })
    }
}
