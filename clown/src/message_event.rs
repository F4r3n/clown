#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageViewInfo(
        Option<usize>, /*server id */
        Option<String>,
        crate::message_irc::message_content::MessageKind,
        String,
    ),
    PrivMsg(
        usize,  /*server id */
        String, /*source */
        String, /*target*/
        String, /*content*/
    ),
    Notice(
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
    JoinServer(usize /*server id */),
    SetTopic(
        usize,          /*server id */
        Option<String>, /*source*/
        String,         /*channel */
        String,         /*topic */
    ),
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
    QuitAll(Option<String> /*reason*/),
    SettingsDidChange,
    Bel,
    CloseBuffer(Option<usize> /*server id */, String /*name */),
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
