use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{BufReader, BufWriter};
use tokio::task::JoinHandle;

use crate::conn::{self, Connection};
use crate::message::MessageReceiver;
use crate::outgoing::CommandSender;
use crate::outgoing::Outgoing;
use std::fs::File;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone)]
pub struct State {
    list_users: Arc<RwLock<Vec<String>>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            list_users: Arc::new(RwLock::new(vec![])),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct IRCConfig {
    pub nickname: String,
    pub real_name: String,
    pub username: String,
    pub password: Option<String>,
    pub channel: String,
}

pub struct Client {
    sender: CommandSender,
    state: Arc<State>,

    irc_config: IRCConfig,
    outgoing: Outgoing,
    message_receiver: Option<MessageReceiver>,
    log: Option<std::io::BufWriter<File>>,
}

impl Client {
    pub fn new(irc_config: IRCConfig, in_file: Option<std::fs::File>) -> Self {
        let mut outgoing = Outgoing::default();
        let (sender, message_receiver) = outgoing.create_outgoing();
        Self {
            sender,
            irc_config,
            outgoing,
            state: Arc::new(State::new()),
            message_receiver: Some(message_receiver),
            log: in_file.map(std::io::BufWriter::new),
        }
    }

    fn try_connect(&mut self) -> anyhow::Result<()> {
        let mut command_sender = self.command_sender();

        //command_sender.send(crate::command::Command::CAP("LS 302".to_string()))?;
        if let Some(password) = &self.irc_config.password {
            command_sender.send(crate::command::Command::Pass(password.clone()))?;
        }
        command_sender.send(crate::command::Command::Nick(
            self.irc_config.nickname.clone(),
        ))?;
        command_sender.send(crate::command::Command::User(
            self.irc_config.username.clone(),
            self.irc_config.real_name.clone(),
        ))?;

        Ok(())
    }

    async fn start<T>(mut self, stream: T) -> anyhow::Result<()>
    where
        T: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);
        self.try_connect()?;
        self.outgoing
            .process(self.log, reader, writer, self.state.clone())
            .await
    }

    pub fn spawn(
        self,
        connection_config: conn::ConnectionConfig,
    ) -> JoinHandle<anyhow::Result<()>> {
        tokio::spawn(async move {
            let conn = Connection::new(connection_config).connect().await?;
            self.start(conn).await
        })
    }

    pub fn command_sender(&self) -> CommandSender {
        self.sender.clone()
    }

    pub fn state(&self) -> Arc<State> {
        self.state.clone()
    }

    pub fn message_receiver(&mut self) -> Option<MessageReceiver> {
        self.message_receiver.take()
    }
}
