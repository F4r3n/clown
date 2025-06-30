use tokio::sync::mpsc;

pub struct CommandReceiver {
    pub inner: mpsc::UnboundedReceiver<String>,
}
