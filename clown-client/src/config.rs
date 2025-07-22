use clown_core::client::ClownConfig;
use clown_core::conn::ConnectionConfig;
use directories::ProjectDirs;
use yaml_rust2::Yaml;
pub struct Config {
    pub connection_config: ConnectionConfig,
    pub clown_config: ClownConfig,
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

impl Config {
    pub fn new() -> Self {
        if let Ok(value) = Self::read() {
            value
        } else {
            Self {
                clown_config: ClownConfig {
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
            }
        }
    }

    fn read() -> color_eyre::Result<Self> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "share", "clown") {
            let config_path = proj_dirs.config_dir().join(".config");
            let content = std::fs::read_to_string(config_path)?;
            let yamls = yaml_rust2::YamlLoader::load_from_str(&content)?;
            let doc = yamls
                .first()
                .ok_or(color_eyre::eyre::Error::msg("No yaml"))?;
            if let Some(connection_config) = Self::read_connection_config(doc)
                && let Some(clown_config) = Self::read_clown_config(doc)
            {
                Ok(Self {
                    clown_config,
                    connection_config,
                })
            } else {
                Err(color_eyre::eyre::eyre!("Invalid config"))
            }
        } else {
            Err(color_eyre::eyre::eyre!("file not found"))
        }
    }

    fn read_connection_config(doc: &Yaml) -> Option<ConnectionConfig> {
        if let Some(address) = yaml_path!(doc, "server", "address").as_str()
            && let Some(port) = yaml_path!(doc, "server", "port").as_i64()
        {
            Some(ConnectionConfig {
                address: address.into(),
                port: port as u16,
            })
        } else {
            None
        }
    }

    fn read_clown_config(doc: &Yaml) -> Option<ClownConfig> {
        if let Some(channel) = yaml_path!(doc, "clown", "channel").as_str()
            && let Some(nickname) = yaml_path!(doc, "clown", "nickname").as_str()
            && let Some(real_name) = yaml_path!(doc, "clown", "real_name").as_str()
            && let Some(username) = yaml_path!(doc, "clown", "username").as_str()
        {
            Some(ClownConfig {
                channel: channel.into(),
                nickname: nickname.into(),
                password: yaml_path!(doc, "clown", "nickname")
                    .as_str()
                    .map(|v| v.to_string()),
                real_name: real_name.into(),
                username: username.into(),
            })
        } else {
            None
        }
    }
}
