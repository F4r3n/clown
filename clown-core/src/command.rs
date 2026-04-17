use phf::phf_map;
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
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

#[derive(Debug, Clone)]
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
    pub async fn write<W>(&self, writer: &mut BufWriter<W>) -> Result<(), std::io::Error>
    where
        W: AsyncWrite + Unpin,
    {
        match self {
            Command::PrivMsg(channel, message) => {
                writer.write_all(b"PRIVMSG ").await?;
                writer.write_all(channel.as_bytes()).await?;
                writer.write_all(b" :").await?;
                writer.write_all(message.as_bytes()).await?;
            }
            Command::Join(channel) => {
                writer.write_all(b"JOIN ").await?;
                writer.write_all(channel.as_bytes()).await?;
            }
            Command::Part(channel, reason) => {
                writer.write_all(b"PART ").await?;
                writer.write_all(channel.as_bytes()).await?;
                if let Some(r) = reason {
                    writer.write_all(b" :").await?;
                    writer.write_all(r.as_bytes()).await?;
                }
            }
            Command::Notice(target, message) => {
                writer.write_all(b"NOTICE ").await?;
                writer.write_all(target.as_bytes()).await?;
                writer.write_all(b" :").await?;
                writer.write_all(message.as_bytes()).await?;
            }
            Command::Topic(channel, topic) => {
                writer.write_all(b"TOPIC ").await?;
                writer.write_all(channel.as_bytes()).await?;
                writer.write_all(b" :").await?;
                writer.write_all(topic.as_bytes()).await?;
            }
            Command::Mode(target, mode) => {
                writer.write_all(b"MODE ").await?;
                writer.write_all(target.as_bytes()).await?;
                writer.write_all(b" ").await?;
                writer.write_all(mode.as_bytes()).await?;
            }
            Command::Who(mask) => {
                writer.write_all(b"WHO ").await?;
                writer.write_all(mask.as_bytes()).await?;
            }
            Command::List(channel) => {
                writer.write_all(b"LIST").await?;
                if let Some(c) = channel {
                    writer.write_all(b" ").await?;
                    writer.write_all(c.as_bytes()).await?;
                }
            }
            Command::Invite(nick, channel) => {
                writer.write_all(b"INVITE ").await?;
                writer.write_all(nick.as_bytes()).await?;
                writer.write_all(b" ").await?;
                writer.write_all(channel.as_bytes()).await?;
            }
            Command::Kick(channel, nick, reason) => {
                writer.write_all(b"KICK ").await?;
                writer.write_all(channel.as_bytes()).await?;
                writer.write_all(b" ").await?;
                writer.write_all(nick.as_bytes()).await?;
                if let Some(r) = reason {
                    writer.write_all(b" :").await?;
                    writer.write_all(r.as_bytes()).await?;
                }
            }
            Command::Nick(nickname) => {
                writer.write_all(b"NICK ").await?;
                writer.write_all(nickname.as_bytes()).await?;
            }
            Command::Pass(pass) => {
                writer.write_all(b"PASS ").await?;
                writer.write_all(pass.as_bytes()).await?;
            }
            Command::User(username, realname) => {
                writer.write_all(b"USER ").await?;
                writer.write_all(username.as_bytes()).await?;
                writer.write_all(b" 0 * :").await?;
                writer.write_all(realname.as_bytes()).await?;
            }
            Command::Ping(token) => {
                writer.write_all(b"PING ").await?;
                writer.write_all(token.as_bytes()).await?;
            }
            Command::Pong(token) => {
                writer.write_all(b"PONG ").await?;
                writer.write_all(token.as_bytes()).await?;
            }
            Command::Cap(cap) => {
                writer.write_all(b"CAP ").await?;
                writer.write_all(cap.as_bytes()).await?;
            }
            Command::Quit(reason) => {
                writer.write_all(b"QUIT").await?;
                if let Some(r) = reason {
                    writer.write_all(b" :").await?;
                    writer.write_all(r.as_bytes()).await?;
                }
            }
            Command::Unknown(un) => {
                writer.write_all(un.as_bytes()).await?;
            }
            Command::Error(_) => {}
        }
        // Every IRC message ends with CRLF
        writer.write_all(b"\r\n").await
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
