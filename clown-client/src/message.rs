#[derive(PartialEq)]
pub enum Message {
    SendMessage(String),
    AddMessageView(String),
    PullIRC,
    Connect,
    Join,
    Quit,
}
