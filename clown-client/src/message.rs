#[derive(PartialEq)]
pub enum Message {
    SendMessage(String),
    AddMessageView(String),
    PullIRC,
    Connect,
    Quit,
}
