use clown_core::client::Client;
use tokio::sync::mpsc;

use crate::{
    irc_view::message_content::MessageContent,
    message_event::MessageEvent,
    model::{IRCConnection, Model},
};

use strum::{EnumIter, EnumMessage, IntoEnumIterator, IntoStaticStr};

#[derive(IntoStaticStr, Debug, EnumIter, EnumMessage)]
pub enum ClientCommand {
    #[strum(
        message = "Connect",
        detailed_message = "To connect to the server, if already connected does nothing"
    )]
    Connect,
    #[strum(message = "Quit", detailed_message = "To quit the server and the app")]
    Quit(Option<String>),
    #[strum(message = "Nick", detailed_message = "To change your nickname")]
    Nick(String),
    #[strum(message = "Help", detailed_message = "To display the list of commands")]
    Help,
    #[strum(
        message = "Spell",
        detailed_message = "To prepare the spellchecker for a specific language: fr, en"
    )]
    Spell(Option<String>),
    #[strum(message = "me", detailed_message = "To create an action")]
    Action(String),
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
                "nick" => args.map(|v| ClientCommand::Nick(v.to_string())),
                "help" => Some(ClientCommand::Help),
                "spell" => Some(ClientCommand::Spell(args.map(|v| v.to_string()))),
                "me" => args.map(|v| ClientCommand::Action(v.to_string())),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
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
    MessageEvent::AddMessageView(None, MessageContent::new_info(output))
}

pub fn connect_irc(model: &mut Model) -> Option<MessageEvent> {
    if !model.is_irc_finished() {
        return Some(MessageEvent::AddMessageView(
            None,
            MessageContent::new_error("Already connected".to_string()),
        ));
    }
    if let Some(connection_config) = model.get_connection_config() {
        let login_config = &model.get_login_config();

        let mut client = Client::new(login_config);
        if let Some(reciever) = client.message_receiver() {
            let command_sender = client.command_sender();

            let (error_sender, error_receiver) = mpsc::unbounded_channel();
            if model.retry > 0 {
                model.retry -= 1;
                model.irc_connection = Some(IRCConnection {
                    command_sender,
                    error_receiver,
                    _error_sender: error_sender.clone(),
                    message_reciever: reciever,
                    task: tokio::spawn(async move {
                        if let Err(err) = client.launch(&connection_config).await {
                            let _ = error_sender.send(format!("Connection error: {err}"));
                        }
                    }),
                });
            }
        }
    }

    None
}
