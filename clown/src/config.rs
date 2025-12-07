use clown_core::client::LoginConfig;
use clown_core::conn::ConnectionConfig;
use std::path::PathBuf;

use crate::project_path::ProjectPath;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientConfig {
    pub auto_join: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig { auto_join: true }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub connection_config: Option<ConnectionConfig>,
    pub login_config: LoginConfig,
    pub client_config: ClientConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            login_config: LoginConfig {
                nickname: "nickname".into(),
                password: None,
                real_name: None,
                username: None,
                channel: "#rust-spam".into(),
            },
            connection_config: None,
            client_config: ClientConfig::default(),
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
}
