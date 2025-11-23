use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{BufReader, BufWriter};

use crate::conn::{self, Connection};
use crate::error::ClownError;
use crate::message::MessageReceiver;
use crate::outgoing::CommandSender;
use crate::outgoing::Outgoing;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct LoginConfig {
    pub nickname: String,
    pub real_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub channel: String,
}

pub struct Client {
    sender: CommandSender,

    login_config: LoginConfig,
    outgoing: Outgoing,
    message_receiver: Option<MessageReceiver>,
}

impl Client {
    pub fn new(login_config: &LoginConfig) -> Self {
        let mut outgoing = Outgoing::default();
        let (sender, message_receiver) = outgoing.create_outgoing();
        Self {
            sender,
            login_config: login_config.clone(),
            outgoing,
            message_receiver: Some(message_receiver),
        }
    }

    fn try_connect(&mut self) -> Result<(), ClownError> {
        let mut command_sender = self.command_sender();

        //command_sender.send(crate::command::Command::CAP("LS 302".to_string()))?;
        if let Some(password) = &self.login_config.password {
            command_sender.send(crate::command::Command::Pass(password.clone()))?;
        }
        command_sender.send(crate::command::Command::Nick(
            self.login_config.nickname.clone(),
        ))?;
        command_sender.send(crate::command::Command::User(
            self.login_config
                .username
                .clone()
                .unwrap_or(self.login_config.nickname.clone()),
            self.login_config
                .real_name
                .clone()
                .unwrap_or(self.login_config.nickname.clone())
                .clone(),
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
            .process(reader, writer)
            .await
            .map_err(ClownError::IRCIOError)
    }

    pub async fn launch(
        self,
        connection_config: &conn::ConnectionConfig,
    ) -> Result<(), ClownError> {
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
