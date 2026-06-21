use super::irc_model::IrcModel;
use super::irc_model::IrcServerModel;
use super::model::IRCConnection;
use super::server_id::ServerID;
use anyhow::anyhow;
use clown_core::client::LoginConfig;
use clown_core::command::Command;
use clown_core::conn::ConnectionConfig;
use clown_core::message::ServerMessage;
use tokio::sync::mpsc;

pub struct SessionStatus<'a> {
    pub server_id: ServerID,
    pub channel: Option<std::borrow::Cow<'a, str>>,
    pub nickname: std::borrow::Cow<'a, str>,
}

impl SessionStatus<'_> {
    pub fn to_owned(&self) -> SessionStatus<'static> {
        SessionStatus {
            server_id: self.server_id,
            channel: self
                .channel
                .as_ref()
                .map(|s| std::borrow::Cow::Owned(s.to_string())),
            nickname: std::borrow::Cow::Owned(self.nickname.to_string()),
        }
    }
}

struct RetryState {
    pub max_retry: usize,
    pub counter: usize,
    pub time_before_next_retry: std::time::Duration,
}

impl RetryState {
    pub fn new() -> Self {
        Self {
            max_retry: 5,
            counter: 0,
            time_before_next_retry: std::time::Duration::from_secs(5),
        }
    }

    pub fn get_next_retry(&self) -> std::time::Duration {
        self.time_before_next_retry
            .mul_f32((self.counter + 1) as f32)
    }

    pub fn increment_retry(&mut self) -> bool {
        if self.counter < self.max_retry {
            self.counter += 1;
            true
        } else {
            false
        }
    }
}

struct ServerSlot {
    retry: RetryState,
    connection: Option<IRCConnection>,
}

impl ServerSlot {
    fn new() -> Self {
        Self {
            retry: RetryState::new(),
            connection: None,
        }
    }
}

pub struct Session {
    pub model: IrcModel,
    servers: Vec<ServerSlot>,
}

impl Session {
    pub fn new(in_length: usize) -> Self {
        Self {
            model: IrcModel::new(in_length),
            servers: std::iter::repeat_with(ServerSlot::new)
                .take(in_length)
                .collect(),
        }
    }

    pub fn reset_retry(&mut self, id: ServerID) {
        if let Some(server) = self.servers.get_mut(id.as_usize()) {
            server.retry = RetryState::new();
        }
    }

    pub fn get_mut_connection(&mut self, id: ServerID) -> Option<&mut IRCConnection> {
        self.servers
            .get_mut(id.as_usize())
            .and_then(|v| v.connection.as_mut())
    }

    pub fn get_connection(&self, id: ServerID) -> Option<&IRCConnection> {
        self.servers
            .get(id.as_usize())
            .and_then(|v| v.connection.as_ref())
    }

    pub fn send_command(&mut self, in_id: ServerID, in_command: Command) -> anyhow::Result<()> {
        if let Some(connection) = self.get_mut_connection(in_id) {
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
            && let Some(connection) = self.get_mut_connection(current_id)
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
        self.get_connection(in_id).is_some()
    }

    pub fn send_command_all_server(
        &mut self,
        in_command: Command,
    ) -> impl Iterator<Item = anyhow::Result<()>> {
        self.servers.iter_mut().filter_map(move |v| {
            v.connection.as_mut().map(|conn| {
                conn.command_sender
                    .send(in_command.clone())
                    .map_err(Into::into)
            })
        })
    }

    pub fn iter_valid_connection_id(&self) -> impl Iterator<Item = ServerID> {
        self.servers
            .iter()
            .enumerate()
            .filter(|(_, v)| v.connection.is_some())
            .map(|(i, _)| ServerID::new(i))
    }

    pub fn is_irc_finished(&self, in_id: ServerID) -> bool {
        if let Some(connection) = self.get_connection(in_id) {
            connection.task.is_finished()
        } else {
            true
        }
    }

    pub fn clear_connection(&mut self, in_id: ServerID) {
        if let Some(server_slot) = self.servers.get_mut(in_id.as_usize()) {
            server_slot.connection = None;
        }
    }

    pub fn get_duration_before_retry(&self, in_id: ServerID) -> Option<std::time::Duration> {
        self.servers
            .get(in_id.as_usize())
            .map(|s| s.retry.get_next_retry())
    }

    pub fn pull_all_server_message(&mut self) -> impl Iterator<Item = (ServerID, ServerMessage)> {
        self.servers.iter_mut().enumerate().flat_map(|(i, conn)| {
            conn.connection.iter_mut().flat_map(move |conn| {
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
        self.servers.iter_mut().enumerate().flat_map(|(i, slot)| {
            slot.connection.iter_mut().flat_map(move |conn| {
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
                None => anyhow::bail!("No channel to part"),
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
                channel,
                format!("\x01ACTION {}\x01", content),
            ))
        } else {
            anyhow::bail!("Not connected")
        }
    }

    pub fn get_current_status<'a>(&'a self) -> Option<SessionStatus<'a>> {
        self.get_current_irc_server_model().map(|v| SessionStatus {
            server_id: v.get_server_id(),
            channel: v.get_current_channel().map(std::borrow::Cow::Borrowed),
            nickname: std::borrow::Cow::Borrowed(v.get_current_nick()),
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

        let server = self
            .servers
            .get_mut(in_id.as_usize())
            .ok_or_else(|| anyhow!("Wrong ID {}", in_id))?;
        if !server.retry.increment_retry() {
            anyhow::bail!("No retries left");
        }

        server.connection = Some(IRCConnection {
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
