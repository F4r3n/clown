use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{BufReader, BufWriter};
use tokio::task::JoinHandle;

use crate::conn::{self, Connection};
use crate::error::ClownError;
use crate::message::MessageReceiver;
use crate::outgoing::CommandSender;
use crate::outgoing::Outgoing;
use std::fs::File;

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
            message_receiver: Some(message_receiver),
            log: in_file.map(std::io::BufWriter::new),
        }
    }

    fn try_connect(&mut self) -> Result<(), ClownError> {
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

    async fn start<T>(mut self, stream: T) -> Result<(), ClownError>
    where
        T: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);
        self.try_connect()?;
        self.outgoing
            .process(self.log, reader, writer)
            .await
            .map_err(ClownError::IRCIOError)
    }

    pub fn spawn(
        self,
        connection_config: conn::ConnectionConfig,
    ) -> JoinHandle<Result<(), ClownError>> {
        tokio::spawn(async move {
            let conn = Connection::new(connection_config).connect().await?;
            self.start(conn).await
        })
    }

    pub async fn launch(self, connection_config: conn::ConnectionConfig) -> Result<(), ClownError> {
        let conn = Connection::new(connection_config).connect().await?;
        self.start(conn).await
    }

    pub fn command_sender(&self) -> CommandSender {
        self.sender.clone()
    }

    pub fn message_receiver(&mut self) -> Option<MessageReceiver> {
        self.message_receiver.take()
    }
}
