use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::io::{BufReader, BufWriter};
use tokio::task::JoinHandle;

use crate::message::MessageReceiver;
use crate::outgoing::CommandSender;
use crate::outgoing::Outgoing;
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
}

impl Client {
    pub fn new(irc_config: IRCConfig) -> Self {
        let mut outgoing = Outgoing::default();
        let (sender, message_receiver) = outgoing.create_outgoing();
        Self {
            sender,
            irc_config,
            outgoing,
            state: Arc::new(State::new()),
            message_receiver: Some(message_receiver),
        }
    }

    fn try_connect(&mut self) -> anyhow::Result<()> {
        let mut command_sender = self.command_sender();

        command_sender.send(crate::command::Command::CAP())?;
        if let Some(password) = &self.irc_config.password {
            command_sender.send(crate::command::Command::PASS(password.clone()))?;
        }
        command_sender.send(crate::command::Command::NICK(
            self.irc_config.nickname.clone(),
        ))?;
        command_sender.send(crate::command::Command::USER(
            self.irc_config.username.clone(),
            self.irc_config.real_name.clone(),
        ))?;

        Ok(())
    }

    pub async fn start<T>(mut self, stream: T) -> anyhow::Result<()>
    where
        T: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);
        self.try_connect()?;
        self.outgoing
            .process(reader, writer, self.state.clone())
            .await
    }

    pub fn spawn<T>(self, stream: T) -> JoinHandle<anyhow::Result<()>>
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        tokio::spawn(async move { self.start(stream).await })
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

#[cfg(test)]
mod tests {

    use tokio::join;

    use crate::client;
    use crate::client::IRCConfig;
    use crate::conn::Connection;
    use crate::conn::ConnectionConfig;
    use crate::conn::test::Action;
    use crate::conn::test::StreamMock;

    #[tokio::test]
    async fn test_connect() -> anyhow::Result<()> {
        println!("TEST");
        let option = ConnectionConfig {
            address: "i.chevalier.io".into(),
            port: 6697,
        };
        let irc_config = IRCConfig {
            nickname: "farine".into(),
            password: Some("share-chan".into()),
            real_name: "farine".into(),
            username: "farine".into(),
            channel: "#rust-spam".into(),
        };

        let client = client::Client::new(irc_config);
        let stream = Connection::new(option).connect().await?;
        let state = client.state();
        let sender = client.command_sender();

        let handle = client.spawn(stream);
        // Send commands after delay
        /*tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(tx) = &sender.inner {
            tx.send(command::Command::nick())?;
        }*/

        join!(handle);
        Ok(())
    }
}
