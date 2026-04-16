use crate::model::ServerID;

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageViewInfo(
        Option<ServerID>, /*server id */
        Option<String>,
        crate::message_irc::message_content::MessageKind,
        String,
    ),
    PrivMsg(
        ServerID, /*server id */
        String,   /*source */
        String,   /*target*/
        String,   /*content*/
    ),
    Notice(
        ServerID, /*server id */
        String,   /*source */
        String,   /*target*/
        String,   /*content*/
    ),
    ActionMsg(
        ServerID, /*server id */
        String,   /*source */
        String,   /*target*/
        String,   /*content*/
    ),
    SelectChannel(Option<ServerID> /*server id */, String),
    UpdateUsers(
        ServerID,    /*server id */
        String,      /*channel*/
        Vec<String>, /*list users */
    ),
    ReplaceUser(
        ServerID, /*server id */
        String,   /*old */
        String,   /*new */
    ),
    Join(
        ServerID, /*server id */
        String,   /*channel*/
        String,   /*user */
    ),
    JoinServer(ServerID /*server id */),
    SetTopic(
        ServerID,       /*server id */
        Option<String>, /*source*/
        String,         /*channel */
        String,         /*topic */
    ),
    HoverURL(String /* URL */),
    #[allow(dead_code)]
    Hover(String), //currently not used, but the skeleton can be used anywhere
    PullIRC,
    Connect(ServerID),
    DisConnect(ServerID),
    OpenWeb(String),
    SpellChecker(Option<String>),
    Part(
        ServerID, /*server id */
        String,   /*channel */
        String,   /*user */
    ),
    Quit(
        ServerID,       /*server id */
        String,         /*user*/
        Option<String>, /*reason*/
    ),
    QuitAll(Option<String> /*reason*/),
    SettingsDidChange,
    Bel,
    CloseBuffer(Option<ServerID> /*server id */, String /*name */),
}

impl MessageEvent {
    /// Helper to convert any compatible error into a MessageEvent
    pub fn from_error<E>(err: E) -> Self
    where
        E: Into<MessageEvent>,
    {
        err.into()
    }
}

impl From<anyhow::Error> for MessageEvent {
    fn from(value: anyhow::Error) -> Self {
        MessageEvent::AddMessageViewInfo(
            None,
            None,
            crate::message_irc::message_content::MessageKind::Error,
            format!("{}", value),
        )
    }
}
