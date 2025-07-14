use crate::text_widget::MessageContent;

#[derive(PartialEq)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(MessageContent),
    PullIRC,
    Connect,
    Join,
    Quit,
}
