use crate::irc_view::message_content::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(String, MessageContent),
    HighlightUser(String),
    SelectChannel(String),
    UpdateUsers(Vec<String>),
    ReplaceUser(String /*old */, String /*new */),
    JoinUser(String),
    RemoveUser(String /*user */),
    SetTopic(String /*topic */),
    Hover(String /*Message or URL */, u16, u16),
    PullIRC,
    Connect,
    DisConnect,
    Tick,
    Quit(Option<String>),
}
