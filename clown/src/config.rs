use std::ops::Deref;
use std::path::PathBuf;

use clown_core::client::LoginConfig;
use clown_core::conn::ConnectionConfig;
use color_eyre::eyre::Ok;
use color_eyre::eyre::Result;

use crate::irc_view::color_user::ColorGenerator;
use crate::project_path::ProjectPath;
use color_eyre::{eyre::bail, eyre::eyre};
pub trait RemoteConfig {
    fn get_value<I>(&self, path: I, option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>;
    fn set_value<I>(&mut self, path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>;
    //should be static
    fn get_paths(prefix: &str) -> Vec<String>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct Connection {
    pub address: String,
    pub port: u16,
}

impl RemoteConfig for Connection {
    fn get_value<I>(&self, mut path: I, _option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("address") => Ok(self.address.to_string()),
            Some("port") => Ok(self.port.to_string()),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("[Connection]: Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("address") => {
                self.address = value;
                Ok(())
            }
            Some("port") => {
                self.port = value.parse::<u16>()?;
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("[Connection]: Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        ["address", "port"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Login {
    pub nickname: String,
    pub real_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl RemoteConfig for Login {
    fn get_value<I>(&self, mut path: I, _option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("nickname") => Ok(self.nickname.to_string()),
            Some("real_name") => Ok(self.real_name.clone().unwrap_or_default()),
            Some("username") => Ok(self.username.clone().unwrap_or_default()),
            Some("password") => Ok(self.password.clone().unwrap_or_default()),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("[Login]: Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("nickname") => {
                self.nickname = value;
                Ok(())
            }
            Some("real_name") => {
                self.real_name = Some(value);
                Ok(())
            }
            Some("username") => {
                self.username = Some(value);
                Ok(())
            }
            Some("password") => {
                self.password = Some(value);
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        ["nickname", "real_name", "username", "password"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Channels {
    pub list: Vec<String>,
    pub auto_join: bool,
}

impl RemoteConfig for Channels {
    fn get_value<I>(&self, mut path: I, _option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("list") => Ok(self.list.join(",")),
            Some("auto_join") => Ok(self.auto_join.to_string()),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("list") => {
                self.list = value
                    .split(',')
                    .map(|v| v.trim().to_string())
                    .collect::<Vec<String>>();
                Ok(())
            }
            Some("auto_join") => {
                self.auto_join = value.parse::<bool>()?;
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        ["list", "auto_join"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>()
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Server {
    pub name: String,
    pub connection: Connection,
    pub login: Login,
    pub channels: Channels,
}

impl RemoteConfig for Server {
    fn get_value<I>(&self, mut path: I, option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("name") => Ok(self.name.to_string()),
            Some("connection") => self.connection.get_value(path, option),
            Some("login") => self.login.get_value(path, option),
            Some("channels") => self.channels.get_value(path, option),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("name") => {
                self.name = value;
                Ok(())
            }
            Some("connection") => {
                self.connection.set_value(path, value)?;
                Ok(())
            }
            Some("login") => {
                self.login.set_value(path, value)?;
                Ok(())
            }
            Some("channels") => {
                self.channels.set_value(path, value)?;
                Ok(())
            }
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        let p = format!("{prefix}.");
        let mut fields = vec![format!("{p}name")];

        // Call static methods on child types
        fields.extend(Connection::get_paths(&format!("{p}connection")));
        fields.extend(Login::get_paths(&format!("{p}login")));
        fields.extend(Channels::get_paths(&format!("{p}channels")));

        fields
    }
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

impl RemoteConfig for Completion {
    fn get_value<I>(&self, mut path: I, option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("on_empty_input") => self.on_empty_input.get_value(path, option),
            Some("in_message") => self.in_message.get_value(path, option),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("on_empty_input") => {
                self.on_empty_input.set_value(path, value)?;
                Ok(())
            }
            Some("in_message") => {
                self.in_message.set_value(path, value)?;
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        let mut fields = ["on_empty_input", "in_message"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>();
        fields.extend(CompletionBehavior::get_paths(
            format!("{prefix}.on_empty_input").as_str(),
        ));
        fields.extend(CompletionBehavior::get_paths(
            format!("{prefix}.in_message").as_str(),
        ));
        fields
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct CompletionBehavior {
    #[serde(default)]
    pub suffix: Option<String>,
}

impl RemoteConfig for CompletionBehavior {
    fn get_value<I>(&self, mut path: I, _option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("suffix") => Ok(self.suffix.clone().unwrap_or_default()),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("suffix") => {
                self.suffix = Some(value);
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        ["suffix"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>()
    }
}
//
// COLORS
//

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct NicknameColors {
    #[serde(default)]
    pub seed: u64,

    #[serde(default)]
    pub overrides: ahash::AHashMap<String, String>,
}
impl RemoteConfig for NicknameColors {
    fn get_value<I>(&self, mut path: I, option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("seed") => Ok(self.seed.to_string()),
            Some("overrides") => {
                if let Some(name) = option {
                    if let Some(value) = self.overrides.get(name) {
                        Ok(value.to_string())
                    } else {
                        bail!("The {name} has not been overrided")
                    }
                } else {
                    bail!("Name invalid: overrides 'name'")
                }
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("seed") => {
                self.seed = value.parse::<u64>()?;
                Ok(())
            }
            Some("overrides") => {
                let mut split = value.split_ascii_whitespace();
                if let Some(name) = split.next()
                    && let Some(hexa) = split.next()
                {
                    match ColorGenerator::is_color_valid(hexa) {
                        color_eyre::Result::Ok(()) => {
                            self.overrides.insert(name.to_string(), hexa.to_string());
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Err(eyre!("Invalid value: {value}.\n should be: name #FFFFFF"))
                }
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        vec![format!("{prefix}.seed"), format!("{prefix}.overrides")]
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Topic {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl RemoteConfig for Topic {
    fn get_value<I>(&self, mut path: I, _option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("enabled") => Ok(self.enabled.to_string()),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("enabled") => {
                self.enabled = value.parse::<bool>()?;
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        ["enabled"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>()
    }
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

impl RemoteConfig for Discuss {
    fn get_value<I>(&self, mut path: I, option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("left_bar") => self.left_bar.get_value(path, option),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("left_bar") => self.left_bar.set_value(path, value),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        let mut fields = ["left_bar"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>();
        fields.extend(LeftBar::get_paths(format!("{prefix}.left_bar").as_str()));
        fields
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LeftBar {
    #[serde(default = "default_true")]
    pub time: bool,
    #[serde(default = "default_true")]
    pub nickname: bool,
}

impl RemoteConfig for LeftBar {
    fn get_value<I>(&self, mut path: I, _option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("time") => Ok(self.time.to_string()),
            Some("nickname") => Ok(self.nickname.to_string()),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("time") => {
                self.time = value.parse::<bool>()?;
                Ok(())
            }
            Some("nickname") => {
                self.nickname = value.parse::<bool>()?;
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        ["time", "nickname"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>()
    }
}
impl Default for LeftBar {
    fn default() -> Self {
        Self {
            time: true,
            nickname: true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Users {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl RemoteConfig for Users {
    fn get_value<I>(&self, mut path: I, _option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("enabled") => Ok(self.enabled.to_string()),
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("enabled") => {
                self.enabled = value.parse::<bool>()?;
                Ok(())
            }
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(prefix: &str) -> Vec<String> {
        ["enabled"]
            .iter()
            .map(|v| format!("{prefix}.{v}"))
            .collect::<Vec<String>>()
    }
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

impl RemoteConfig for Config {
    fn get_value<I>(&self, mut path: I, option: Option<&str>) -> Result<String>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("server") => {
                if let Some(option) = option {
                    let mut split = option.split_ascii_whitespace();
                    let server_id = split
                        .next()
                        .and_then(|v| v.parse::<usize>().ok())
                        .ok_or_else(|| eyre!("Invalid server index"))?;
                    let rest = split.collect::<String>();

                    self.servers
                        .get(server_id)
                        .ok_or_else(|| eyre!("Invalid server index"))?
                        .get_value(path, Some(rest.as_str()))
                } else {
                    Err(eyre!("Invalid option. Needs to have: 'id'"))
                }
            }
            Some("completion") => self.completion.get_value(path, option),
            Some("nickname_colors") => self.nickname_colors.get_value(path, option),
            Some("discuss") => self.discuss.get_value(path, option),
            Some("users") => self.users.get_value(path, option),
            Some("topic") => self.topic.get_value(path, option),
            Some("meta") => match path.next().as_ref().map(AsRef::as_ref) {
                Some("version") => Ok(self.meta.version.to_string()),
                _ => bail!("Invalid path"),
            },
            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn set_value<I>(&mut self, mut path: I, value: String) -> Result<()>
    where
        I: Iterator,
        I::Item: AsRef<str>,
    {
        match path.next().as_ref().map(AsRef::as_ref) {
            Some("server") => {
                let mut split = value.split_ascii_whitespace();
                let server_id = split
                    .next()
                    .and_then(|v| v.parse::<usize>().ok())
                    .ok_or_else(|| eyre!("Invalid server index"))?;
                let rest = split.collect::<String>();

                self.servers
                    .get_mut(server_id)
                    .ok_or_else(|| eyre!("Invalid server index"))?
                    .set_value(path, rest)
            }
            Some("completion") => self.completion.set_value(path, value),
            Some("nickname_colors") => self.nickname_colors.set_value(path, value),
            Some("discuss") => self.discuss.set_value(path, value),
            Some("users") => self.users.set_value(path, value),
            Some("topic") => self.topic.set_value(path, value),
            Some("meta") =>
            //Meta cannot be set
            {
                bail!("Invalid path, impossible to set")
            }

            Some(p) => bail!("Invalid path {p}"),
            _ => bail!("Invalid path"),
        }
    }

    fn get_paths(_prefix: &str) -> Vec<String> {
        let mut fields = [
            "server",
            "completion",
            "nickname_colors",
            "discuss",
            "users",
            "topic",
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>();

        fields.extend(Server::get_paths("server"));

        fields.extend(Completion::get_paths("completion"));
        fields.extend(NicknameColors::get_paths("nickname_colors"));
        fields.extend(Discuss::get_paths("discuss"));
        fields.extend(Users::get_paths("users"));
        fields.extend(Topic::get_paths("topic"));

        fields
    }
}
impl Config {
    pub fn new(config_name: &str) -> color_eyre::Result<Self> {
        Self::read(config_name)
    }

    pub fn list_fields() -> Vec<String> {
        Config::get_paths("")
    }

    pub fn get_value_from_root(
        &self,
        path: &str,
        option: Option<&str>,
    ) -> color_eyre::Result<String> {
        self.get_value(path.split('.'), option)
    }

    pub fn set_value_from_root(&mut self, path: &str, value: String) -> color_eyre::Result<()> {
        self.set_value(path.split('.'), value)
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
            let content = std::fs::read(config_path)?;
            toml::from_slice::<Config>(&content).map_err(|e| eyre!("{}", e))
        } else {
            bail!("Invalid config")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config {
            servers: vec![Server {
                name: "test".into(),
                connection: Connection {
                    address: "irc.example.com".into(),
                    port: 6667,
                },
                login: Login {
                    nickname: "tester".into(),
                    real_name: Some("Real".into()),
                    username: Some("user".into()),
                    password: None,
                },
                channels: Channels {
                    list: vec!["#rust".into(), "#linux".into()],
                    auto_join: true,
                },
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_get_value_from_root() {
        let config = sample_config();

        assert_eq!(
            config
                .get_value_from_root("server.connection.address", Some("0"))
                .unwrap(),
            "irc.example.com"
        );

        assert_eq!(
            config
                .get_value_from_root("server.connection.port", Some("0"))
                .unwrap(),
            "6667"
        );

        assert_eq!(
            config
                .get_value_from_root("server.login.nickname", Some("0"))
                .unwrap(),
            "tester"
        );
    }

    #[test]
    fn test_set_value_from_root() {
        let mut config = sample_config();

        config
            .set_value_from_root("server.connection.address", "0 irc.new.net".into())
            .unwrap();

        assert_eq!(config.servers[0].connection.address, "irc.new.net");

        config
            .set_value_from_root("server.connection.port", "0 7000".into())
            .unwrap();

        assert_eq!(config.servers[0].connection.port, 7000);
    }

    #[test]
    fn test_list_fields_contains_known_paths() {
        let fields = Config::list_fields();

        assert!(
            fields.contains(&".server.connection.address".to_string())
                || fields.contains(&"server.connection.address".to_string())
        );

        assert!(fields.iter().any(|v| v.contains("server.login.nickname")));
    }

    #[test]
    fn test_get_nickname() {
        let config = sample_config();

        assert_eq!(config.get_nickname(0), Some("tester"));
        assert_eq!(config.get_nickname(1), None);
    }

    #[test]
    fn test_get_channels() {
        let config = sample_config();

        let channels: Vec<_> = config.get_channels(0).collect();

        assert_eq!(channels, vec!["#rust", "#linux"]);
    }

    #[test]
    fn test_get_address() {
        let config = sample_config();

        assert_eq!(config.get_address(0), Some("irc.example.com"));
    }

    #[test]
    fn test_autojoin_helpers() {
        let config = sample_config();

        assert!(config.is_autojoin_id(0));

        let ids: Vec<_> = config.is_autojoin().collect();

        assert_eq!(ids, vec![0]);
    }

    #[test]
    fn test_get_connection_config() {
        let config = sample_config();

        let conn = config.get_connection_config(0).unwrap();

        assert_eq!(conn.address, "irc.example.com");
        assert_eq!(conn.port, 6667);
    }

    #[test]
    fn test_get_login_config() {
        let config = sample_config();

        let login = config.get_login_config(0).unwrap();

        assert_eq!(login.nickname, "tester");
        assert_eq!(login.username, Some("user".into()));
        assert_eq!(login.real_name, Some("Real".into()));
    }

    #[test]
    fn test_invalid_path() {
        let config = sample_config();

        assert!(
            config
                .get_value_from_root("servers.invalid.field", Some("0"))
                .is_err()
        );
    }
}
