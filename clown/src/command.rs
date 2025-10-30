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
    Quit(String),
    #[strum(message = "Nick", detailed_message = "To change your nickname")]
    Nick(String),
    #[strum(message = "Help", detailed_message = "To display the list of commands")]
    Help,
}

pub fn parse_command(in_content: &str) -> Option<ClientCommand> {
    if let Some(next) = in_content.trim().to_ascii_lowercase().strip_prefix('/') {
        let mut splits = next.split_ascii_whitespace();
        if let Some(command) = splits.next() {
            match command {
                "connect" => Some(ClientCommand::Connect),
                "quit" => Some(ClientCommand::Quit(splits.collect())),
                "nick" => Some(ClientCommand::Nick(splits.collect())),
                "help" => Some(ClientCommand::Help),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn help(channel: &str) -> MessageEvent {
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
    MessageEvent::AddMessageView(
        channel.to_string(),
        MessageContent::new_info(output.as_str()),
    )
}

pub fn connect_irc(model: &mut Model) -> Option<MessageEvent> {
    if !model.is_irc_finished() {
        return Some(MessageEvent::AddMessageView(
            model.current_channel.clone(),
            MessageContent::new_error("Already connected".to_string()),
        ));
    }
    let connection_config = model.config.connection_config.clone();
    let login_config = &model.config.login_config;

    let mut client = Client::new(login_config);
    if let Some(reciever) = client.message_receiver() {
        let command_sender = client.command_sender();

        let (error_sender, error_receiver) = mpsc::unbounded_channel();

        model.irc_connection = Some(IRCConnection {
            command_sender: command_sender,
            error_receiver: error_receiver,
            _error_sender: error_sender.clone(),
            message_reciever: reciever,
            task: tokio::spawn(async move {
                if let Err(err) = client.launch(&connection_config).await {
                    let _ = error_sender.send(format!("Connection error: {err}"));
                }
            }),
        });
    }

    None
}
