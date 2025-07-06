use clown_parser::Message;
use tokio::sync::mpsc;
pub struct MessageSender {
    pub inner: mpsc::UnboundedSender<Message>,
}

pub struct MessageReceiver {
    pub inner: mpsc::UnboundedReceiver<Message>,
}
