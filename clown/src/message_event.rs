use crate::irc_view::message_content::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(Option<String>, MessageContent),
    HighlightUser(String),
    SelectChannel(String),
    UpdateUsers(Vec<String>),
    ReplaceUser(String /*old */, String /*new */),
    JoinUser(String),
    RemoveUser(String /*user */),
    SetTopic(String /*topic */),
    #[cfg(feature = "website-preview")]
    HoverURL(String /* URL */),
    Hover(String),
    PullIRC,
    Connect,
    DisConnect,
    OpenWeb(String),
    SpellChecker(Option<String>),
    Quit(Option<String>),
}
