use crate::message_irc::message_content::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(Option<String>, MessageContent),
    HighlightUser(String),
    SelectChannel(String),
    UpdateUsers(String /*channel*/, Vec<String> /*list users */),
    ReplaceUser(String /*old */, String /*new */),
    Join(String /*channel*/, Option<String> /*user */),
    SetTopic(String /*channel */, String /*topic */),
    #[cfg(feature = "website-preview")]
    HoverURL(String /* URL */),
    Hover(String),
    PullIRC,
    Connect,
    DisConnect,
    OpenWeb(String),
    SpellChecker(Option<String>),
    Part(
        String, /*channel */
        String, /*user */
        bool,   /*main user*/
    ),
    Quit(String /*user*/, Option<String> /*reason*/),
}
