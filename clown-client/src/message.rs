#[derive(PartialEq)]
pub enum Message {
    SendMessage(String),
    AddMessage(String),
    Connect,
    Quit,
}
