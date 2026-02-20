use crate::message_irc::message_content::MessageContent;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageView(Option<String>, MessageContent),
    PrivMsg(
        usize,  /*server id */
        String, /*source */
        String, /*target*/
        String, /*content*/
    ),
    ActionMsg(
        usize,  /*server id */
        String, /*source */
        String, /*target*/
        String, /*content*/
    ),
    SelectChannel(Option<usize> /*server id */, String),
    UpdateUsers(
        usize,       /*server id */
        String,      /*channel*/
        Vec<String>, /*list users */
    ),
    ReplaceUser(
        usize,  /*server id */
        String, /*old */
        String, /*new */
    ),
    Join(
        usize,  /*server id */
        String, /*channel*/
        String, /*user */
    ),
    JoinServer(usize /*server id */, String),
    SetTopic(
        usize,          /*server id */
        Option<String>, /*source*/
        String,         /*channel */
        String,         /*topic */
    ),
    #[cfg(feature = "website-preview")]
    HoverURL(String /* URL */),
    #[allow(dead_code)]
    Hover(String), //currently not used, but the skeleton can be used anywhere
    PullIRC,
    Connect(usize),
    DisConnect(usize),
    OpenWeb(String),
    SpellChecker(Option<String>),
    Part(
        usize,  /*server id */
        String, /*channel */
        String, /*user */
    ),
    Quit(
        usize,          /*server id */
        String,         /*user*/
        Option<String>, /*reason*/
    ),
    Bel,
}
