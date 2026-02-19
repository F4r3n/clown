use crate::irc_view::irc_model::IrcServerModel;
use crate::message_event::MessageEvent;
use crate::{irc_view::irc_model::IrcModel, model::IRCConnection};
use clown_core::client::LoginConfig;
use clown_core::command::Command;
use clown_core::conn::ConnectionConfig;
use clown_core::message::ServerMessage;
use tokio::sync::mpsc;

pub struct SessionStatus<'a> {
    pub server_id: usize,
    pub channel: Option<&'a str>,
    pub nickname: &'a str,
}

pub struct IrcSession {
    pub model: IrcModel,
    pub connections: Vec<Option<IRCConnection>>,
    pub retry: usize,
}

impl IrcSession {
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

    pub fn get_current_nick(&self) -> Option<&str> {
        self.get_current_irc_server_model()
            .map(|v| v.get_current_nick())
    }

    pub fn send_command(&mut self, in_id: usize, in_command: Command) {
        if let Some(Some(connection)) = self.connections.get_mut(in_id) {
            let _ = connection.command_sender.send(in_command);
        }
    }

    pub fn send_command_current_server(&mut self, in_command: Command) {
        if let Some(current_id) = self.model.current_id
            && let Some(Some(connection)) = self.connections.get_mut(current_id)
        {
            let _ = connection.command_sender.send(in_command);
        }
    }

    pub fn is_irc_finished(&self, in_id: usize) -> bool {
        if let Some(Some(connection)) = self.connections.get(in_id) {
            connection.task.is_finished()
        } else {
            true
        }
    }

    pub fn clear_connection(&mut self, in_id: usize) {
        if let Some(connection) = self.connections.get_mut(in_id) {
            *connection = None;
        }
        self.model.clear_server(in_id);
    }

    pub fn pull_server_message(&mut self, in_id: usize) -> Option<ServerMessage> {
        if let Some(Some(connection)) = self.connections.get_mut(in_id) {
            connection.message_reciever.inner.try_recv().ok()
        } else {
            None
        }
    }

    pub fn pull_all_server_message(&mut self) -> impl Iterator<Item = (usize, ServerMessage)> + '_ {
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
                            .map(|msg| (i, msg))
                    })
                })
            })
    }

    pub fn pull_all_server_error(&mut self) -> impl Iterator<Item = (usize, String)> {
        self.connections
            .iter_mut()
            .enumerate()
            .flat_map(|(i, conn)| {
                conn.iter_mut().flat_map(move |conn| {
                    std::iter::from_fn(move || {
                        conn.error_receiver.try_recv().ok().map(|msg| (i, msg))
                    })
                })
            })
    }

    pub fn is_connected(&self, in_id: usize) -> bool {
        if let Some(connection) = self.connections.get(in_id) {
            connection.is_some()
        } else {
            false
        }
    }

    pub fn all_connected_servers(&self) -> impl Iterator<Item = usize> {
        self.connections
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.is_some().then_some(i))
    }

    pub fn send_command_topic(&mut self, topic: String) {
        if let Some(irc_model) = self.get_current_irc_server_model()
            && let Some(channel) = irc_model.get_current_channel()
        {
            self.send_command_current_server(Command::Topic(channel.to_string(), topic));
        }
    }

    pub fn send_command_join(&mut self, server: String) {
        self.send_command_current_server(Command::Join(server));
    }

    pub fn send_command_part(&mut self, channel: Option<String>, reason: Option<String>) {
        let Some(irc_model) = self.get_current_irc_server_model() else {
            return;
        };

        let channel = match channel {
            Some(c) => c,
            None => match irc_model.get_current_channel() {
                Some(c) => c.to_string(),
                None => return, // no channel to part
            },
        };

        self.send_command_current_server(Command::Part(channel, reason));
    }

    pub fn send_command_action(&mut self, content: String) {
        if let Some(irc_model) = self.get_current_irc_server_model()
            && let Some(channel) = irc_model.get_current_channel()
        {
            let channel = channel.to_string();
            self.send_command_current_server(clown_core::command::Command::PrivMsg(
                channel.to_string(),
                format!("\x01ACTION {}\x01", content.clone()),
            ));
        }
    }

    pub fn get_current_status<'a>(&'a self) -> Option<SessionStatus<'a>> {
        if let Some(irc_model) = self.get_current_irc_server_model() {
            Some(SessionStatus {
                server_id: irc_model.get_server_id(),
                channel: irc_model.get_current_channel(),
                nickname: irc_model.get_current_nick(),
            })
        } else {
            None
        }
    }

    pub fn init_irc_model(&mut self, stored_nick: String, in_id: usize) {
        self.model.init_server(in_id, stored_nick);
    }

    pub fn get_current_server_id(&self) -> Option<usize> {
        self.model.current_id
    }

    pub fn get_current_irc_server_model(&self) -> Option<&IrcServerModel> {
        self.model.get_current_server()
    }

    pub fn handle_action(&mut self, msg: &MessageEvent) {
        self.model.handle_action(msg);
    }

    pub fn init_connection(
        &mut self,
        in_id: usize,
        connection_config: ConnectionConfig,
        login_config: LoginConfig,
    ) {
        if !connection_config.address.is_empty() {
            let mut client = clown_core::client::Client::new(login_config);
            if let Some(reciever) = client.message_receiver() {
                let command_sender = client.command_sender();

                let (error_sender, error_receiver) = mpsc::channel(10);
                //TODO: retry per connection
                if self.retry > 0 {
                    self.retry -= 1;
                    self.connections[in_id] = Some(IRCConnection {
                        command_sender,
                        error_receiver,
                        _error_sender: error_sender.clone(),
                        message_reciever: reciever,
                        task: tokio::spawn(async move {
                            if let Err(err) = client.launch(&connection_config).await {
                                let _ = error_sender.send(format!("Connection error: {err}")).await;
                            }
                        }),
                    });
                }
            }
        }
    }
}
