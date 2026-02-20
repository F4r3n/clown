use crate::message_event::MessageEvent;
use crate::message_irc::message_content::MessageContent;

use strum::{EnumIter, EnumMessage, IntoEnumIterator, IntoStaticStr};

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
    Unknown(Option<String>),
}

pub fn parse_command(in_content: &str) -> Option<ClientCommand> {
    if let Some(next) = in_content.trim().strip_prefix('/') {
        if let Some((command, args)) = next
            .find(' ')
            .map(|v| {
                Some((
                    &next[..v],
                    Some(&next[v.saturating_add(1).min(next.len() - 1)..]),
                ))
            })
            .unwrap_or(Some((next, None)))
        {
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
                _ => Some(ClientCommand::Unknown(Some(command.to_string()))),
            }
        } else {
            Some(ClientCommand::Unknown(None))
        }
    } else {
        None
    }
}

fn privmsg(message: &str) -> Option<ClientCommand> {
    if let Some((channel, content)) = message
        .find(' ')
        .map(|v| {
            Some((
                &message[..v],
                message.get(v.saturating_add(1).min(message.len() - 1)..),
            ))
        })
        .unwrap_or(Some((message, None)))
    {
        content.map(|v| ClientCommand::PrivMSG(channel.to_string(), v.to_string()))
    } else {
        None
    }
}

fn part(message: Option<&str>) -> ClientCommand {
    if let Some(message) = message {
        if let Some((channel, reason)) = message
            .find(' ')
            .map(|v| {
                Some((
                    &message[..v],
                    Some(&message[v.saturating_add(1).min(message.len() - 1)..]),
                ))
            })
            .unwrap_or(Some((message, None)))
        {
            ClientCommand::Part(Some(channel.to_string()), reason.map(|v| v.to_string()))
        } else {
            ClientCommand::Part(Some(message.to_string()), None)
        }
    } else {
        ClientCommand::Part(None, None)
    }
}

pub fn help() -> MessageEvent {
    let mut output: String = "List of commands, type /command:\n".into();
    for e in ClientCommand::iter() {
        output.push_str(
            format!(
                "Command {}: {}\n",
                e.get_message().unwrap_or_default(),
                e.get_detailed_message().unwrap_or_default()
            )
            .as_str(),
        );
    }
    MessageEvent::AddMessageView(None, None, MessageContent::new_info(output))
}
