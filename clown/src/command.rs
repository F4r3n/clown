use clown_core::client::Client;

use crate::{irc_view::message_content::MessageContent, message_event::MessageEvent, model::Model};

use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

#[derive(IntoStaticStr, Debug, EnumIter)]
pub enum ClientCommand {
    Connect,
    Quit,
    Nick(String),
    Help,
}

pub fn parse_command(in_content: &str) -> Option<ClientCommand> {
    if let Some(next) = in_content.trim().to_ascii_lowercase().strip_prefix('/') {
        let mut splits = next.split_ascii_whitespace();
        if let Some(command) = splits.next() {
            match command {
                "connect" => Some(ClientCommand::Connect),
                "quit" => Some(ClientCommand::Quit),
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
        output += e.into();
        output += "\n";
    }
    MessageEvent::AddMessageView(
        channel.to_string(),
        Box::new(MessageContent::new_info(output.as_str())),
    )
}

pub fn connect_irc(model: &mut Model) -> Option<MessageEvent> {
    if let Some(task) = &model.task {
        if !task.is_finished() {
            return Some(MessageEvent::AddMessageView(
                model.current_channel.clone(),
                Box::new(MessageContent::new_error("Already connected".to_string())),
            ));
        }
    }
    let connection_config = model.config.connection_config.clone();
    let login_config = &model.config.login_config;

    let mut client = Client::new(login_config);
    let reciever = client.message_receiver();
    let command_sender = client.command_sender();

    model.command_sender = Some(command_sender);
    model.message_reciever = reciever;

    model.task = Some(tokio::spawn(async move {
        client.launch(&connection_config).await.map_err(Into::into)
    }));
    None
}
