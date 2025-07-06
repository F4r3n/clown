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
        let mut outgoing = Outgoing::new();
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
