use clown_core::client::LoginConfig;
use clown_core::conn::ConnectionConfig;
use directories::ProjectDirs;
use hashlink::LinkedHashMap;
use std::path::PathBuf;
use yaml_rust2::Yaml;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub auto_join: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig { auto_join: true }
    }
}

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

impl Config {
    pub fn new() -> Self {
        if let Ok(value) = Self::read() {
            value
        } else {
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
    // Convert ConnectionConfig to Yaml::Hash
    fn to_yaml_connection(conn: &ConnectionConfig) -> Yaml {
        let mut map = LinkedHashMap::new();
        map.insert(Yaml::from_str("address"), Yaml::from_str(&conn.address));
        map.insert(Yaml::from_str("port"), Yaml::Integer(conn.port as i64));
        Yaml::Hash(map)
    }

    // Convert ConnectionConfig to Yaml::Hash
    fn to_yaml_option(option: &ClientConfig) -> Yaml {
        let mut map = LinkedHashMap::new();
        map.insert(
            Yaml::from_str("auto_join"),
            Yaml::from_str(option.auto_join.to_string().as_str()),
        );
        Yaml::Hash(map)
    }

    // Convert LoginConfig to Yaml::Hash
    fn to_yaml_login_config(clown: &LoginConfig) -> Yaml {
        let mut map = LinkedHashMap::new();
        map.insert(Yaml::from_str("nickname"), Yaml::from_str(&clown.nickname));
        map.insert(
            Yaml::from_str("real_name"),
            Yaml::from_str(&clown.real_name),
        );
        map.insert(Yaml::from_str("username"), Yaml::from_str(&clown.username));
        if let Some(pw) = &clown.password {
            map.insert(Yaml::from_str("password"), Yaml::from_str(pw));
        }
        map.insert(Yaml::from_str("channel"), Yaml::from_str(&clown.channel));
        Yaml::Hash(map)
    }

    // Convert full Config to Yaml
    fn to_yaml_config(&self) -> Yaml {
        let mut map = LinkedHashMap::new();
        map.insert(
            Yaml::from_str("server"),
            Self::to_yaml_connection(&self.connection_config),
        );
        map.insert(
            Yaml::from_str("login"),
            Self::to_yaml_login_config(&self.login_config),
        );
        map.insert(
            Yaml::from_str("option"),
            Self::to_yaml_option(&self.client_config),
        );
        Yaml::Hash(map)
    }

    pub fn save(&self) -> color_eyre::Result<()> {
        let mut out_str = String::new();
        let mut emitter = yaml_rust2::YamlEmitter::new(&mut out_str);
        emitter.dump(&self.to_yaml_config())?;
        let config_path =
            Self::config_path().ok_or(color_eyre::eyre::Error::msg("Invalid Path"))?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_path, out_str)?;
        Ok(())
    }

    fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "share", "clown")
            .map(|proj_dirs| proj_dirs.config_dir().join("clown"))
    }

    fn read() -> color_eyre::Result<Self> {
        if let Some(config_path) = Self::config_path() {
            let content = std::fs::read_to_string(config_path)?;
            let yamls = yaml_rust2::YamlLoader::load_from_str(&content)?;
            let doc = yamls
                .first()
                .ok_or(color_eyre::eyre::Error::msg("No yaml"))?;
            if let Some(connection_config) = Self::read_connection_config(doc) {
                Ok(Self {
                    login_config: Self::read_login_config(doc).unwrap_or(LoginConfig {
                        nickname: "nickname".into(),
                        password: None,
                        real_name: "real".into(),
                        username: "username".into(),
                        channel: "#rust-spam".into(),
                    }),
                    connection_config,
                    client_config: Self::read_client_config(doc).unwrap_or_default(),
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

    fn read_client_config(doc: &Yaml) -> Option<ClientConfig> {
        yaml_path!(doc, "option", "auto_join")
            .as_bool()
            .map(|auto_join| ClientConfig { auto_join })
    }

    fn read_login_config(doc: &Yaml) -> Option<LoginConfig> {
        if let Some(channel) = yaml_path!(doc, "login", "channel").as_str()
            && let Some(nickname) = yaml_path!(doc, "login", "nickname").as_str()
            && let Some(real_name) = yaml_path!(doc, "login", "real_name").as_str()
            && let Some(username) = yaml_path!(doc, "login", "username").as_str()
        {
            Some(LoginConfig {
                channel: channel.into(),
                nickname: nickname.into(),
                password: yaml_path!(doc, "login", "nickname")
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
