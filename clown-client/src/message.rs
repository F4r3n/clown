#[derive(PartialEq)]
pub enum Message {
    SendMessage(String),
    AddMessage(String),
    Quit,
}
