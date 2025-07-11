use tokio::sync::mpsc;

pub struct CommandReceiver {
    pub inner: mpsc::UnboundedReceiver<Command>,
}

#[derive(Debug)]
pub enum Command {
    NICK(String),
    PRIVMSG(String, String),
    PASS(String),
    USER(String, String),
    PING(String),
    PONG(String),
    QUIT(Option<String>),
    JOIN(String),
    PART(String, Option<String>),
    NOTICE(String, String),
    TOPIC(String, String),
    MODE(String, String),
    WHO(String),
    LIST(Option<String>),
    INVITE(String, String),
    KICK(String, String, Option<String>),
    CAP(String),
    WELCOME(String, String),
    ERROR(String), // The error message
                   // Add more as needed
}
impl Command {
    pub fn as_bytes(&self) -> Vec<u8> {
        match &self {
            Command::PRIVMSG(channel, message) => {
                format!("PRIVMSG {channel} :{message}\r\n").into_bytes()
            }
            Command::JOIN(channel) => format!("JOIN {channel}\r\n").into_bytes(),
            Command::PART(channel, Some(reason)) => {
                format!("PART {channel} :{reason}\r\n").into_bytes()
            }
            Command::PART(channel, None) => format!("PART {channel}\r\n").into_bytes(),
            Command::NOTICE(target, message) => {
                format!("NOTICE {target} :{message}\r\n").into_bytes()
            }
            Command::TOPIC(channel, topic) => format!("TOPIC {channel} :{topic}\r\n").into_bytes(),
            Command::MODE(target, mode) => format!("MODE {target} {mode}\r\n").into_bytes(),
            Command::WHO(mask) => format!("WHO {mask}\r\n").into_bytes(),
            Command::LIST(Some(channel)) => format!("LIST {channel}\r\n").into_bytes(),
            Command::LIST(None) => b"LIST\r\n".to_vec(),
            Command::INVITE(nick, channel) => format!("INVITE {nick} {channel}\r\n").into_bytes(),
            Command::KICK(channel, nick, Some(reason)) => {
                format!("KICK {channel} {nick} :{reason}\r\n").into_bytes()
            }
            Command::ERROR(msg) => format!("ERROR :{msg}\r\n").into_bytes(),
            Command::KICK(channel, nick, None) => format!("KICK {channel} {nick}\r\n").into_bytes(),
            // ... existing patterns
            _ => panic!("Not a supported command"),
        }
    }
}

pub struct CommandBuilder;

impl CommandBuilder {
    //USER alice 0 * :Alice Example
    fn user(parameters: Vec<&str>) -> Option<Command> {
        if let Some(target) = parameters.first() {
            let message_to_send = parameters[3..].join(" ");
            Some(Command::USER(target.to_string(), message_to_send))
        } else {
            None
        }
    }

    // ERROR :<message>
    fn error(parameters: Vec<&str>) -> Option<Command> {
        if !parameters.is_empty() {
            Some(Command::ERROR(parameters.join(" ")))
        } else {
            None
        }
    }
    //Command: PONG
    //Parameters: [<server>] <token>
    fn pong(parameters: Vec<&str>) -> Option<Command> {
        Some(Command::PONG(
            parameters.last().map(|v| v.to_string()).unwrap_or_default(),
        ))
    }

    fn quit(parameters: Vec<&str>) -> Option<Command> {
        Some(Command::QUIT(parameters.first().map(|v| v.to_string())))
    }

    fn make_command_1<F>(parameters: Vec<&str>, ctor: F) -> Option<Command>
    where
        F: Fn(String) -> Command,
    {
        parameters.first().map(|target| ctor(target.to_string()))
    }

    fn make_command_2<F>(parameters: Vec<&str>, ctor: F) -> Option<Command>
    where
        F: Fn(String, String) -> Command,
    {
        if let Some(target) = parameters.first() {
            let message_to_send = parameters[1..].join(" ");
            Some(ctor(target.to_string(), message_to_send))
        } else {
            None
        }
    }

    // PART <channel> [:reason]
    fn part(parameters: Vec<&str>) -> Option<Command> {
        if let Some(channel) = parameters.first() {
            let reason = if parameters.len() > 1 {
                Some(parameters[1..].join(" "))
            } else {
                None
            };
            Some(Command::PART(channel.to_string(), reason))
        } else {
            None
        }
    }

    // NOTICE <target> :<message>
    fn notice(parameters: Vec<&str>) -> Option<Command> {
        if parameters.len() >= 2 {
            Some(Command::NOTICE(
                parameters[0].to_string(),
                parameters[1..].join(" "),
            ))
        } else {
            None
        }
    }

    // TOPIC <channel> :<topic>
    fn topic(parameters: Vec<&str>) -> Option<Command> {
        if parameters.len() >= 2 {
            Some(Command::TOPIC(
                parameters[0].to_string(),
                parameters[1..].join(" "),
            ))
        } else {
            None
        }
    }

    // MODE <target> <mode>
    fn mode(parameters: Vec<&str>) -> Option<Command> {
        if parameters.len() >= 2 {
            Some(Command::MODE(
                parameters[0].to_string(),
                parameters[1..].join(" "),
            ))
        } else {
            None
        }
    }

    // WHO <mask>
    fn who(parameters: Vec<&str>) -> Option<Command> {
        parameters
            .first()
            .map(|mask| Command::WHO(mask.to_string()))
    }

    // LIST [<channel>]
    fn list(parameters: Vec<&str>) -> Option<Command> {
        if let Some(channel) = parameters.first() {
            Some(Command::LIST(Some(channel.to_string())))
        } else {
            Some(Command::LIST(None))
        }
    }

    // INVITE <nick> <channel>
    fn invite(parameters: Vec<&str>) -> Option<Command> {
        if parameters.len() >= 2 {
            Some(Command::INVITE(
                parameters[0].to_string(),
                parameters[1].to_string(),
            ))
        } else {
            None
        }
    }

    // KICK <channel> <nick> [:reason]
    fn kick(parameters: Vec<&str>) -> Option<Command> {
        if parameters.len() >= 2 {
            let channel = parameters[0].to_string();
            let nick = parameters[1].to_string();
            let reason = if parameters.len() > 2 {
                Some(parameters[2..].join(" "))
            } else {
                None
            };
            Some(Command::KICK(channel, nick, reason))
        } else {
            None
        }
    }

    pub fn get_command(command_name: &str, trailing: Vec<&str>) -> Option<Command> {
        match command_name {
            "NICK" => CommandBuilder::make_command_1(trailing, Command::NICK),
            "PASS" => CommandBuilder::make_command_1(trailing, Command::PASS),
            "QUIT" => CommandBuilder::quit(trailing),
            "PING" => CommandBuilder::make_command_1(trailing, Command::PING),
            "PONG" => CommandBuilder::pong(trailing),
            "USER" => CommandBuilder::user(trailing),
            "PRIVMSG" => CommandBuilder::make_command_2(trailing, Command::PRIVMSG),
            "JOIN" => CommandBuilder::make_command_1(trailing, Command::JOIN),
            "PART" => CommandBuilder::part(trailing),
            "NOTICE" => CommandBuilder::notice(trailing),
            "TOPIC" => CommandBuilder::topic(trailing),
            "MODE" => CommandBuilder::mode(trailing),
            "WHO" => CommandBuilder::who(trailing),
            "LIST" => CommandBuilder::list(trailing),
            "INVITE" => CommandBuilder::invite(trailing),
            "KICK" => CommandBuilder::kick(trailing),
            "ERROR" => CommandBuilder::error(trailing),
            "CAP" => CommandBuilder::make_command_1(trailing, Command::CAP),
            "001" => CommandBuilder::make_command_2(trailing, Command::WELCOME),
            _ => None,
        }
    }
}
