use crate::irc_view::text_widget::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(String, Box<MessageContent>),
    HighlightUser(String),
    SelectChannel(String),
    UpdateUsers(Vec<String>),
    ReplaceUser(String /*old */, String /*new */),
    JoinUser(String),
    RemoveUser(String /*user */),
    SetTopic(String /*topic */),
    PullIRC,
    Connect,
    DisConnect,
    Quit,
}
