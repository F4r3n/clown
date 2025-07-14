#[derive(PartialEq)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(String),
    PullIRC,
    Connect,
    Join,
    Quit,
}
