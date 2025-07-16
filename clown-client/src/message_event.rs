use crate::irc_view::text_widget::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(MessageContent),
    UpdateUsers(Vec<String>),
    ReplaceUser(String /*old */, String /*new */),
    JoinUser(String),
    RemoveUser(String /*user */),
    SetTopic(String /*topic */),
    PullIRC,
    Connect,
    Join,
    Quit,
}
