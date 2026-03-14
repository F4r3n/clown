use crate::message_event::MessageEvent;

use strum::{EnumIter, EnumMessage, IntoEnumIterator, IntoStaticStr};

#[derive(IntoStaticStr, Debug, EnumIter, EnumMessage, Default)]
pub enum ConfigCommand {
    Add,
    Set,
    #[default]
    Get,
}

#[derive(IntoStaticStr, Debug, EnumIter, EnumMessage)]
pub enum ClientCommand {
    #[strum(
        message = "connect",
        detailed_message = "To connect to the server, if already connected does nothing"
    )]
    Connect,
    #[strum(message = "quit", detailed_message = "To quit the server and the app")]
    Quit(Option<String>),
    #[strum(message = "nick", detailed_message = "To change your nickname")]
    Nick(String),
    #[strum(message = "help", detailed_message = "To display the list of commands")]
    Help,
    #[strum(
        message = "spell",
        detailed_message = "To prepare the spellchecker for a specific language: fr, en"
    )]
    Spell(Option<String>),
    #[strum(message = "me", detailed_message = "To create an action")]
    Action(String),
    #[strum(message = "join", detailed_message = "To join a channel: {channel}")]
    Join(String),
    #[strum(
        message = "part",
        detailed_message = "To quit a channel: {channel} {reason}"
    )]
    Part(Option<String>, Option<String>),
    #[strum(
        message = "msg",
        detailed_message = "To send a message to a user or channel"
    )]
    PrivMSG(String, String),
    #[strum(
        message = "topic",
        detailed_message = "To set the topic of the channel"
    )]
    Topic(String),
    #[strum(message = "config", detailed_message = "To set/get the config")]
    Config(
        ConfigCommand,
        String,         /*Theme*/
        Option<String>, /*Value */
    ),
    #[strum(message = "close", detailed_message = "To close a buffer")]
    CloseBuffer(Option<String> /*buffer name, if not current*/),
    Unknown(Option<String>),
}

fn get_next_word(in_content: &str) -> Option<(&str, Option<&str>)> {
    let content = in_content.trim_ascii_start();
    if content.is_empty() {
        return None;
    }
    let result = content.find(|v: char| v.is_ascii_whitespace());
    if let Some(pos) = result {
        Some((&content[..pos], Some(content[pos..].trim_ascii_start())))
    } else {
        Some((content, None))
    }
}

pub fn parse_command(in_content: &str) -> Option<ClientCommand> {
    if let Some(next) = in_content.trim_ascii_start().strip_prefix('/') {
        if let Some((command, args)) = get_next_word(next) {
            match command.to_lowercase().as_str() {
                "connect" => Some(ClientCommand::Connect),
                "quit" => Some(ClientCommand::Quit(args.map(|v| v.to_string()))),
                "topic" => Some(args.map_or(ClientCommand::Unknown(None), |v| {
                    ClientCommand::Topic(v.to_string())
                })),
                "nick" => Some(args.map_or(ClientCommand::Unknown(None), |v| {
                    ClientCommand::Nick(v.to_string())
                })),
                "help" => Some(ClientCommand::Help),
                "spell" => Some(ClientCommand::Spell(args.map(|v| v.to_string()))),
                "me" => Some(args.map_or(ClientCommand::Unknown(None), |v| {
                    ClientCommand::Action(v.to_string())
                })),
                "join" => Some(args.map_or(ClientCommand::Unknown(None), |v| {
                    ClientCommand::Join(v.to_string())
                })),
                "part" => Some(part(args)),
                "msg" => args.map_or(Some(ClientCommand::Unknown(None)), |v| {
                    privmsg(v).or(Some(ClientCommand::Unknown(None)))
                }),
                "config" => args.map_or(Some(ClientCommand::Unknown(None)), |v| {
                    config_command(v).or(Some(ClientCommand::Unknown(None)))
                }),
                "close" => Some(ClientCommand::CloseBuffer(args.map(|v| v.to_string()))),
                _ => Some(ClientCommand::Unknown(Some(command.to_string()))),
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn config_command(message: &str) -> Option<ClientCommand> {
    if let Some((config_type, content)) = get_next_word(message) {
        let config_command_type = match config_type {
            "get" => ConfigCommand::Get,
            "set" => ConfigCommand::Set,
            "add" => ConfigCommand::Add,
            _ => ConfigCommand::Get,
        };

        if let Some(content) = content
            && let Some((theme, value)) = get_next_word(content)
        {
            Some(ClientCommand::Config(
                config_command_type,
                theme.to_string(),
                value.map(|v| v.to_string()),
            ))
        } else {
            None
        }
    } else {
        None
    }
}

fn privmsg(message: &str) -> Option<ClientCommand> {
    if let Some((channel, content)) = get_next_word(message) {
        content.map(|v| ClientCommand::PrivMSG(channel.to_string(), v.to_string()))
    } else {
        None
    }
}

fn part(message: Option<&str>) -> ClientCommand {
    if let Some(message) = message {
        if let Some((channel, reason)) = get_next_word(message) {
            ClientCommand::Part(Some(channel.to_string()), reason.map(|v| v.to_string()))
        } else {
            ClientCommand::Part(Some(message.to_string()), None)
        }
    } else {
        ClientCommand::Part(None, None)
    }
}
pub fn help() -> MessageEvent {
    use std::fmt::Write;

    let mut output = String::from("List of commands, type /command:");

    for e in ClientCommand::iter() {
        if let Some(message) = e.get_message()
            && let Err(e) = write!(
                &mut output,
                "\nCommand {}: {}",
                message,
                e.get_detailed_message().unwrap_or_default()
            )
        {
            tracing::error!("Cannot write help command: {}", e)
        }
    }

    MessageEvent::AddMessageViewInfo(
        None,
        None,
        crate::message_irc::message_content::MessageKind::Info,
        output,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn next_word() {
        let (word, rest) = get_next_word("/connect").unwrap();
        assert_eq!(word, "/connect");
        assert_eq!(rest, None);

        let (word, rest) = get_next_word("/connect test hey").unwrap();
        assert_eq!(word, "/connect");
        assert_eq!(rest, Some("test hey"));
    }

    #[test]
    fn parse_connect() {
        let cmd = parse_command("/connect");
        assert!(matches!(cmd, Some(ClientCommand::Connect)));
    }

    #[test]
    fn parse_help() {
        let cmd = parse_command("/help");
        assert!(matches!(cmd, Some(ClientCommand::Help)));
    }

    #[test]
    fn parse_quit_without_reason() {
        let cmd = parse_command("/quit");
        assert!(matches!(cmd, Some(ClientCommand::Quit(None))));
    }

    #[test]
    fn parse_quit_with_reason() {
        let cmd = parse_command("/quit bye");
        assert!(matches!(cmd, Some(ClientCommand::Quit(Some(ref s))) if s == "bye"));
    }

    #[test]
    fn parse_nick() {
        let cmd = parse_command("/nick bob");
        assert!(matches!(cmd, Some(ClientCommand::Nick(ref s)) if s == "bob"));
    }

    #[test]
    fn parse_join() {
        let cmd = parse_command("/join #rust");
        assert!(matches!(cmd, Some(ClientCommand::Join(ref s)) if s == "#rust"));
    }

    #[test]
    fn parse_me_action() {
        let cmd = parse_command("/me waves");
        assert!(matches!(cmd, Some(ClientCommand::Action(ref s)) if s == "waves"));
    }

    #[test]
    fn parse_privmsg() {
        let cmd = parse_command("/msg #rust hello");
        assert!(matches!(
            cmd,
            Some(ClientCommand::PrivMSG(ref chan, ref msg))
            if chan == "#rust" && msg == "hello"
        ));
    }

    #[test]
    fn parse_topic() {
        let cmd = parse_command("/topic new topic");
        assert!(matches!(
            cmd,
            Some(ClientCommand::Topic(ref s)) if s == "new topic"
        ));
    }

    #[test]
    fn parse_part_without_reason() {
        let cmd = parse_command("/part #rust");
        assert!(matches!(
            cmd,
            Some(ClientCommand::Part(Some(ref chan), None))
            if chan == "#rust"
        ));
    }

    #[test]
    fn parse_part_with_reason() {
        let cmd = parse_command("/part #rust bye");
        assert!(matches!(
            cmd,
            Some(ClientCommand::Part(Some(ref chan), Some(ref reason)))
            if chan == "#rust" && reason == "bye"
        ));
    }

    #[test]
    fn parse_spell() {
        let cmd = parse_command("/spell fr");
        assert!(matches!(
            cmd,
            Some(ClientCommand::Spell(Some(ref s))) if s == "fr"
        ));
    }

    #[test]
    fn parse_config_get() {
        let cmd = parse_command("/config get theme");
        assert!(matches!(
            cmd,
            Some(ClientCommand::Config(ConfigCommand::Get, ref theme, None))
            if theme == "theme"
        ));
    }

    #[test]
    fn parse_config_set() {
        let cmd = parse_command("/config set theme dark");
        assert!(matches!(
            cmd,
            Some(ClientCommand::Config(ConfigCommand::Set, ref theme, Some(ref value)))
            if theme == "theme" && value == "dark"
        ));
    }

    #[test]
    fn parse_unknown_command() {
        let cmd = parse_command("/foobar");
        assert!(matches!(
            cmd,
            Some(ClientCommand::Unknown(Some(ref s))) if s == "foobar"
        ));
    }

    #[test]
    fn parse_non_command() {
        let cmd = parse_command("hello world");
        assert!(cmd.is_none());
    }
}
