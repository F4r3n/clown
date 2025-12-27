use crate::irc_view::message_content::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(Option<String>, MessageContent),
    HighlightUser(String),
    SelectChannel(String),
    UpdateUsers(String /*channel*/, Vec<String> /*list users */),
    ReplaceUser(String /*old */, String /*new */),
    JoinUser(String /*channel*/, String /*nickname*/),
    JoinChannel(String /*channel*/),
    RemoveUser(Option<String> /*channel*/, String /*user */),
    SetTopic(String /*channel */, String /*topic */),
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
