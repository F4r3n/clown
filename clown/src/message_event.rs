use crate::message_irc::message_content::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(Option<String>, MessageContent),
    PrivMsg(
        String, /*source */
        String, /*target*/
        String, /*content*/
    ),
    ActionMsg(
        String, /*source */
        String, /*target*/
        String, /*content*/
    ),
    SelectChannel(String),
    UpdateUsers(String /*channel*/, Vec<String> /*list users */),
    ReplaceUser(String /*old */, String /*new */),
    Join(
        String, /*channel*/
        String, /*user */
        bool,   /*main */
    ),
    JoinServer(String),
    SetTopic(
        Option<String>, /*source*/
        String,         /*channel */
        String,         /*topic */
    ),
    #[cfg(feature = "website-preview")]
    HoverURL(String /* URL */),
    #[allow(dead_code)]
    Hover(String), //currently not used, but the skeleton can be used anywhere
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
    QuitChannels(
        Vec<String>,    /*channel*/
        String,         /*user*/
        Option<String>, /*reason*/
    ),
    Bel,
}
