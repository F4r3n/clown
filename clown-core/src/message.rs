use clown_parser::IRCMessage;
use tokio::sync::mpsc;
pub struct MessageSender {
    pub inner: mpsc::UnboundedSender<String>,
}

pub struct MessageReceiver {
    pub inner: mpsc::UnboundedReceiver<String>,
}
