use crate::irc_view::irc_model::IrcServerModel;
use crate::model::ServerID;
use crate::{irc_view::irc_model::IrcModel, model::IRCConnection};
use anyhow::anyhow;
use clown_core::client::LoginConfig;
use clown_core::command::Command;
use clown_core::conn::ConnectionConfig;
use clown_core::message::ServerMessage;
use tokio::sync::mpsc;
pub struct SessionStatus<'a> {
    pub server_id: ServerID,
    pub channel: Option<&'a str>,
    pub nickname: &'a str,
}

pub struct Session {
    pub model: IrcModel,
    pub connections: Vec<Option<IRCConnection>>,
    pub retry: usize,
}

impl Session {
    pub fn new(in_length: usize) -> Self {
        Self {
            model: IrcModel::new(in_length),
            connections: std::iter::repeat_with(|| None).take(in_length).collect(),
            retry: 5,
        }
    }

    pub fn reset_retry(&mut self) {
        self.retry = 5;
    }

    pub fn send_command(&mut self, in_id: ServerID, in_command: Command) -> anyhow::Result<()> {
        if let Some(Some(connection)) = self.connections.get_mut(in_id.as_usize()) {
            connection
                .command_sender
                .send(in_command)
                .map_err(Into::into)
        } else {
            anyhow::bail!("connection {in_id} not found")
        }
    }

    pub fn send_command_current_server(&mut self, in_command: Command) -> anyhow::Result<()> {
        if let Some(current_id) = self.model.current_id
            && let Some(Some(connection)) = self.connections.get_mut(current_id.as_usize())
        {
            connection
                .command_sender
                .send(in_command)
                .map_err(Into::into)
        } else {
            anyhow::bail!("Not connected")
        }
    }

    pub fn is_connected(&self, in_id: ServerID) -> bool {
        if let Some(connection) = self.connections.get(in_id.as_usize()) {
            connection.is_some()
        } else {
            false
        }
    }

    pub fn send_command_all_server(&mut self, in_command: Command) {
        for connection in self.connections.iter_mut().flatten() {
            let _ = connection.command_sender.send(in_command.clone());
        }
    }

    pub fn iter_valid_connection_id(&self) -> impl Iterator<Item = ServerID> {
        self.connections
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_some())
            .map(|(i, _)| ServerID::new(i))
    }

    pub fn is_irc_finished(&self, in_id: ServerID) -> bool {
        if let Some(Some(connection)) = self.connections.get(in_id.as_usize()) {
            connection.task.is_finished()
        } else {
            true
        }
    }

    pub fn clear_connection(&mut self, in_id: ServerID) {
        if let Some(connection) = self.connections.get_mut(in_id.as_usize()) {
            *connection = None;
        }
        self.model.clear_server(in_id);
    }

    pub fn pull_all_server_message(&mut self) -> impl Iterator<Item = (ServerID, ServerMessage)> {
        self.connections
            .iter_mut()
            .enumerate()
            .flat_map(|(i, conn)| {
                conn.iter_mut().flat_map(move |conn| {
                    std::iter::from_fn(move || {
                        conn.message_reciever
                            .inner
                            .try_recv()
                            .ok()
                            .map(|msg| (ServerID::new(i), msg))
                    })
                })
            })
    }

    pub fn pull_all_server_error(&mut self) -> impl Iterator<Item = (ServerID, String)> {
        self.connections
            .iter_mut()
            .enumerate()
            .flat_map(|(i, conn)| {
                conn.iter_mut().flat_map(move |conn| {
                    std::iter::from_fn(move || {
                        conn.error_receiver
                            .try_recv()
                            .ok()
                            .map(|msg| (ServerID::new(i), msg))
                    })
                })
            })
    }

    pub fn send_command_topic(&mut self, topic: String) -> anyhow::Result<()> {
        if let Some(irc_model) = self.get_current_irc_server_model()
            && let Some(channel) = irc_model.get_current_channel()
        {
            self.send_command_current_server(Command::Topic(channel.to_string(), topic))
        } else {
            anyhow::bail!("Not connected")
        }
    }

    pub fn send_command_join(&mut self, server: String) -> anyhow::Result<()> {
        self.send_command_current_server(Command::Join(server))
    }

    pub fn send_command_part(
        &mut self,
        channel: Option<String>,
        reason: Option<String>,
    ) -> anyhow::Result<()> {
        let Some(irc_model) = self.get_current_irc_server_model() else {
            anyhow::bail!("Not connected");
        };

        let channel = match channel {
            Some(c) => c,
            None => match irc_model.get_current_channel() {
                Some(c) => c.to_string(),
                None => anyhow::bail!("No channel to part"), // no channel to part
            },
        };

        self.send_command_current_server(Command::Part(channel, reason))
    }

    pub fn send_command_action(&mut self, content: String) -> anyhow::Result<()> {
        if let Some(irc_model) = self.get_current_irc_server_model()
            && let Some(channel) = irc_model.get_current_channel()
        {
            let channel = channel.to_string();
            self.send_command_current_server(clown_core::command::Command::PrivMsg(
                channel.to_string(),
                format!("\x01ACTION {}\x01", content.clone()),
            ))
        } else {
            anyhow::bail!("Not connected")
        }
    }

    pub fn get_current_status<'a>(&'a self) -> Option<SessionStatus<'a>> {
        self.get_current_irc_server_model().map(|v| SessionStatus {
            server_id: v.get_server_id(),
            channel: v.get_current_channel(),
            nickname: v.get_current_nick(),
        })
    }

    pub fn init_irc_model(&mut self, stored_nick: String, in_id: ServerID, server_name: String) {
        self.model.init_server(in_id, server_name, stored_nick);
    }

    pub fn get_current_server_id(&self) -> Option<ServerID> {
        self.model.current_id
    }

    pub fn get_current_irc_server_model(&self) -> Option<&IrcServerModel> {
        self.model.get_current_server()
    }

    pub fn handle_action(&mut self, event: &crate::message_event::MessageEvent) {
        self.model.handle_action(event);
    }

    pub fn init_connection(
        &mut self,
        in_id: ServerID,
        connection_config: ConnectionConfig,
        login_config: LoginConfig,
    ) -> anyhow::Result<()> {
        if connection_config.address.is_empty() {
            anyhow::bail!("Connection address is empty");
        }

        let mut client = clown_core::client::Client::new(login_config);

        let receiver = client
            .message_receiver()
            .ok_or_else(|| anyhow!("Failed to get message receiver"))?;

        let command_sender = client.command_sender();

        let (error_sender, error_receiver) = mpsc::channel(10);

        if self.retry == 0 {
            anyhow::bail!("No retries left");
        }

        self.retry -= 1;

        let connection = self
            .connections
            .get_mut(in_id.as_usize())
            .ok_or_else(|| anyhow!("Wrong ID {}", in_id))?;

        *connection = Some(IRCConnection {
            command_sender,
            error_receiver,
            _error_sender: error_sender.clone(),
            message_reciever: receiver,
            task: tokio::spawn(async move {
                if let Err(err) = client.launch(&connection_config).await {
                    let _ = error_sender.send(format!("Connection error: {err}")).await;
                }
            }),
        });

        Ok(())
    }
}
