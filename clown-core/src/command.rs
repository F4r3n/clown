use phf::phf_map;
use tokio::sync::mpsc;

pub struct CommandReceiver {
    pub inner: mpsc::UnboundedReceiver<Command>,
}

enum CommandName {
    Nick,
    PrivMsg,
    Pass,
    User,
    Ping,
    Pong,
    Quit,
    Join,
    Topic,
    Part,
    Notice,
    Mode,
    Who,
    List,
    Invite,
    Kick,
    Cap,
    Error,
}

static COMMAND_NAME: phf::Map<&'static str, CommandName> = phf_map! {
        "NICK" => CommandName::Nick,
        "PASS" => CommandName::Pass,
        "QUIT" => CommandName::Quit,
        "PING" => CommandName::Ping,
        "PONG" => CommandName::Pong,
        "USER" => CommandName::User,
        "PRIVMSG" => CommandName::PrivMsg,
        "JOIN" => CommandName::Join,
        "PART" => CommandName::Part,
        "NOTICE" => CommandName::Notice,
        "TOPIC" => CommandName::Topic,
        "MODE" => CommandName::Mode,
        "WHO" => CommandName::Who,
        "LIST" => CommandName::List,
        "INVITE" => CommandName::Invite,
        "KICK" => CommandName::Kick,
        "CAP" => CommandName::Cap,
        "ERROR" => CommandName::Error
};

#[derive(Debug)]
pub enum Command {
    /// Change your nickname.
    /// NICK <nickname>
    Nick(String),

    /// Send a private message to a channel or user.
    /// PRIVMSG <target> <message>
    PrivMsg(String, String),

    /// Set a connection password (must be sent before NICK/USER).
    /// PASS <password>
    Pass(String),

    /// Specify username and real name at connection.
    /// USER <username> <realname>
    User(String, String),

    /// Server ping, expects a PONG reply.
    /// PING <token>
    Ping(String),

    /// Reply to a server PING.
    /// PONG <token>
    Pong(String),

    /// Disconnect from the server with an optional reason.
    /// QUIT [<reason>]
    Quit(Option<String>),

    /// Join a channel.
    /// JOIN <channel>
    Join(String),

    /// Leave a channel, optionally with a reason.
    /// PART <channel> [<reason>]
    Part(String, Option<String>),

    /// Send a notice to a user or channel.
    /// NOTICE <target> <message>
    Notice(String, String),

    /// Set or query a channel's topic.
    /// TOPIC <channel> <topic>
    Topic(String, String),

    /// Change or view user/channel modes.
    /// MODE <target> <mode>
    Mode(String, String),

    /// List users matching a mask.
    /// WHO <mask>
    Who(String),

    /// List channels, optionally filtered by channel name.
    /// LIST [<channel>]
    List(Option<String>),

    /// Invite a user to a channel.
    /// INVITE <nick> <channel>
    Invite(String, String),

    /// Kick a user from a channel, optionally with a reason.
    /// KICK <channel> <nick> [<reason>]
    Kick(String, String, Option<String>),

    /// Request or negotiate IRC capabilities.
    /// CAP <subcommand>
    Cap(String),

    /// Error command
    /// ERROR :Connection timeout  ; Server closing a client connection because it is unresponsive.
    Error(String),

    Unknown(String),
}
impl Command {
    pub fn as_bytes(&self) -> Vec<u8> {
        match &self {
            Command::PrivMsg(channel, message) => {
                format!("PRIVMSG {channel} :{message}\r\n").into_bytes()
            }
            Command::Join(channel) => format!("JOIN {channel}\r\n").into_bytes(),
            Command::Part(channel, Some(reason)) => {
                format!("PART {channel} :{reason}\r\n").into_bytes()
            }
            Command::Part(channel, None) => format!("PART {channel}\r\n").into_bytes(),
            Command::Notice(target, message) => {
                format!("NOTICE {target} :{message}\r\n").into_bytes()
            }
            Command::Topic(channel, topic) => format!("TOPIC {channel} :{topic}\r\n").into_bytes(),
            Command::Mode(target, mode) => format!("MODE {target} {mode}\r\n").into_bytes(),
            Command::Who(mask) => format!("WHO {mask}\r\n").into_bytes(),
            Command::List(Some(channel)) => format!("LIST {channel}\r\n").into_bytes(),
            Command::List(None) => "LIST\r\n".to_string().into_bytes(),
            Command::Invite(nick, channel) => format!("INVITE {nick} {channel}\r\n").into_bytes(),
            Command::Kick(channel, nick, Some(reason)) => {
                format!("KICK {channel} {nick} :{reason}\r\n").into_bytes()
            }
            Command::Kick(channel, nick, None) => format!("KICK {channel} {nick}\r\n").into_bytes(),
            Command::Nick(nickname) => format!("NICK {nickname}\r\n").into_bytes(),
            Command::Pass(pass) => format!("PASS {pass}\r\n").into_bytes(),
            Command::User(username, realname) => {
                format!("USER {username} 0 * {realname}\r\n").into_bytes()
            }
            Command::Ping(token) => format!("PING {token}\r\n").into_bytes(),
            Command::Pong(token) => format!("PONG {token}\r\n").into_bytes(),
            Command::Cap(cap) => format!("CAP {cap}\r\n").into_bytes(),
            Command::Quit(Some(reason)) => format!("QUIT {reason}\r\n").into_bytes(),
            Command::Quit(None) => "QUIT\r\n".to_string().into_bytes(),
            Command::Unknown(un) => format!("Unkwnon {un}\r\n").into_bytes(),
            Command::Error(un) => format!("Error {un}\r\n").into_bytes(),
        }
    }
}

pub struct CommandBuilder;

impl CommandBuilder {
    //USER alice 0 * :Alice Example
    fn user(parameters: &[&str], trailing: Option<&str>) -> Option<Command> {
        parameters.first().map(|target| {
            Command::User(target.to_string(), trailing.unwrap_or_default().to_string())
        })
    }

    //Command: PONG
    //Parameters: [<server>] <token>
    fn pong(parameters: &[&str]) -> Option<Command> {
        Some(Command::Pong(
            parameters.last().map(|v| v.to_string()).unwrap_or_default(),
        ))
    }

    fn nick(parameters: &[&str], trailing: Option<&str>) -> Option<Command> {
        if !parameters.is_empty() {
            Some(Command::Nick(
                parameters.last().map(|v| v.to_string()).unwrap_or_default(),
            ))
        } else {
            trailing.map(|trailing| Command::Nick(trailing.to_string()))
        }
    }

    fn join(parameters: &[&str], trailing: Option<&str>) -> Option<Command> {
        if !parameters.is_empty() {
            Some(Command::Join(
                parameters.last().map(|v| v.to_string()).unwrap_or_default(),
            ))
        } else {
            trailing.map(|trailing| Command::Join(trailing.to_string()))
        }
    }

    fn quit(trailing: Option<&str>) -> Option<Command> {
        Some(Command::Quit(trailing.map(|v| v.to_string())))
    }

    fn make_command_1<F>(parameters: &[&str], trailing: Option<&str>, ctor: F) -> Option<Command>
    where
        F: Fn(String) -> Command,
    {
        if !parameters.is_empty() {
            Some(ctor(
                parameters.last().map(|v| v.to_string()).unwrap_or_default(),
            ))
        } else {
            trailing.map(|trailing| ctor(trailing.to_string()))
        }
    }

    fn make_command_2<F>(parameters: &[&str], trailing: Option<&str>, ctor: F) -> Option<Command>
    where
        F: Fn(String, String) -> Command,
    {
        parameters
            .first()
            .map(|target| ctor(target.to_string(), trailing.unwrap_or_default().to_string()))
    }

    // PART <channel> [:reason]
    fn part(parameters: &[&str], trailing: Option<&str>) -> Option<Command> {
        if let Some(channel) = parameters.first() {
            let reason = trailing.map(|v| v.to_string());
            Some(Command::Part(channel.to_string(), reason))
        } else {
            None
        }
    }

    // NOTICE <target> :<message>
    fn notice(parameters: &[&str], trailing: Option<&str>) -> Option<Command> {
        parameters.first().map(|first| {
            Command::Notice(first.to_string(), trailing.unwrap_or_default().to_string())
        })
    }

    // TOPIC <channel> :<topic>
    fn topic(parameters: &[&str], trailing: Option<&str>) -> Option<Command> {
        parameters.first().map(|first| {
            Command::Topic(first.to_string(), trailing.unwrap_or_default().to_string())
        })
    }

    // MODE <target> <mode>
    fn mode(parameters: &[&str]) -> Option<Command> {
        if let [first, others @ ..] = parameters {
            Some(Command::Mode(first.to_string(), others.join(" ")))
        } else {
            None
        }
    }

    // WHO <mask>
    fn who(parameters: &[&str]) -> Option<Command> {
        parameters
            .first()
            .map(|mask| Command::Who(mask.to_string()))
    }

    // LIST [<channel>]
    fn list(parameters: &[&str]) -> Option<Command> {
        if let Some(channel) = parameters.first() {
            Some(Command::List(Some(channel.to_string())))
        } else {
            Some(Command::List(None))
        }
    }

    // INVITE <nick> <channel>
    fn invite(parameters: &[&str]) -> Option<Command> {
        if let [first, others] = parameters {
            Some(Command::Invite(first.to_string(), others.to_string()))
        } else {
            None
        }
    }

    // KICK <channel> <nick> [:reason]
    fn kick(parameters: &[&str], trailing: Option<&str>) -> Option<Command> {
        if let [channel, nick] = parameters {
            Some(Command::Kick(
                channel.to_string(),
                nick.to_string(),
                trailing.map(|v| v.to_string()),
            ))
        } else {
            None
        }
    }

    pub fn get_command(
        command_name: &str,
        parameters: &[&str],
        trailing: Option<&str>,
    ) -> Option<Command> {
        if let Some(command_name) = COMMAND_NAME.get(command_name) {
            match command_name {
                CommandName::Nick => CommandBuilder::nick(parameters, trailing),
                CommandName::Pass => {
                    CommandBuilder::make_command_1(parameters, trailing, Command::Pass)
                }
                CommandName::Quit => CommandBuilder::quit(trailing),
                CommandName::Ping => {
                    CommandBuilder::make_command_1(parameters, trailing, Command::Ping)
                }
                CommandName::Pong => CommandBuilder::pong(parameters),
                CommandName::User => CommandBuilder::user(parameters, trailing),
                CommandName::PrivMsg => {
                    CommandBuilder::make_command_2(parameters, trailing, Command::PrivMsg)
                }
                CommandName::Join => CommandBuilder::join(parameters, trailing),
                CommandName::Part => CommandBuilder::part(parameters, trailing),
                CommandName::Notice => CommandBuilder::notice(parameters, trailing),
                CommandName::Topic => CommandBuilder::topic(parameters, trailing),
                CommandName::Mode => CommandBuilder::mode(parameters),
                CommandName::Who => CommandBuilder::who(parameters),
                CommandName::List => CommandBuilder::list(parameters),
                CommandName::Invite => CommandBuilder::invite(parameters),
                CommandName::Kick => CommandBuilder::kick(parameters, trailing),
                CommandName::Error => {
                    CommandBuilder::make_command_1(parameters, trailing, Command::Ping)
                }
                CommandName::Cap => {
                    CommandBuilder::make_command_1(parameters, trailing, Command::Cap)
                }
            }
        } else {
            None
        }
    }
}
