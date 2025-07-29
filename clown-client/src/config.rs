use clown_core::client::LoginConfig;
use clown_core::conn::ConnectionConfig;
use directories::ProjectDirs;
use std::path::PathBuf;

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
    pub connection_config: ConnectionConfig,
    pub login_config: LoginConfig,
    pub client_config: ClientConfig,
}

#[macro_export]
macro_rules! yaml_path {
    ($d:expr, $( $x:expr ),* ) => {
        {
            let mut temp =$d;
            $(
                temp = &temp[$x];
            )*
            temp
        }
    };
}

impl Default for Config {
    fn default() -> Self {
        Self {
            login_config: LoginConfig {
                nickname: "nickname".into(),
                password: None,
                real_name: "real".into(),
                username: "username".into(),
                channel: "#rust-spam".into(),
            },
            connection_config: ConnectionConfig {
                address: "localhost".into(),
                port: 6667,
            },
            client_config: ClientConfig::default(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        if let Ok(value) = Self::read() {
            value
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        let result = toml::to_string(self)?;

        let config_path =
            Self::config_path().ok_or(color_eyre::eyre::Error::msg("Invalid Path"))?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_path, result)?;
        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "share", "clown")
            .map(|proj_dirs| proj_dirs.config_dir().join("clown.toml"))
    }

    fn read() -> color_eyre::Result<Self> {
        if let Some(config_path) = Self::config_path() {
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
