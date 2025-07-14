use crate::irc_view::text_widget::MessageContent;

#[derive(PartialEq)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(MessageContent),
    UpdateUsers(Vec<String>),
    PullIRC,
    Connect,
    Join,
    Quit,
}
