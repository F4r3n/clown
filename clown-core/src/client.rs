use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{BufReader, BufWriter};
use tokio::task::JoinHandle;

use crate::message::MessageReceiver;
use crate::outgoing::Outgoing;
use crate::outgoing::Sender;
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

pub struct Client {
    pub sender: Sender,
    pub state: Arc<State>,

    outgoing: Outgoing,
    message_receiver: MessageReceiver,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let mut outgoing = Outgoing::default();
        let (sender, message_receiver) = outgoing.create_outgoing();
        Self {
            sender,
            outgoing,
            state: Arc::new(State::new()),
            message_receiver,
        }
    }

    pub async fn start<T>(mut self, tls_stream: T) -> anyhow::Result<()>
    where
        T: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        let (reader, writer) = tokio::io::split(tls_stream);
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        self.outgoing
            .process(reader, writer, self.state.clone())
            .await
    }

    pub fn spawn<T>(self, tls_stream: T) -> JoinHandle<anyhow::Result<()>>
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        tokio::spawn(async move { self.start(tls_stream).await })
    }

    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }

    pub fn state(&self) -> Arc<State> {
        self.state.clone()
    }
}

#[cfg(test)]
mod tests {

    use tokio::join;

    use crate::client;
    use crate::conn::ConnectionConfig;
    use crate::conn::test::Action;
    use crate::conn::test::StreamMock;

    #[tokio::test]
    async fn test_connect() -> anyhow::Result<()> {
        let option = ConnectionConfig {
            address: "chat.freenode.net".into(),
            nickname: "farine".into(),
            password: None,
            port: 6697,
            real_name: "farine".into(),
            username: "farine".into(),
        };
        let client = client::Client::new();
        let stream_mock = StreamMock::new(vec![
            Action::Item("test\n".as_bytes().to_vec()),
            Action::Item("HELLO\n".as_bytes().to_vec()),
        ]);
        let state = client.state();
        let sender = client.sender();

        let handle = client.spawn(stream_mock);
        // Send commands after delay
        /*tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(tx) = &sender.inner {
            tx.send(command::Command::nick())?;
        }*/

        join!(handle);
        Ok(())
    }
}
