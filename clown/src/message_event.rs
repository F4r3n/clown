use crate::state::irc_name::IrcName;
use crate::state::server_id::ServerID;
#[derive(PartialEq, Debug)]
pub enum ConfigEvent {
    AddColor(Vec<String> /*usernames*/),
}

#[derive(PartialEq, Debug)]
pub enum MessageEvent {
    MessageInput(String),
    AddMessageViewInfo(
        Option<ServerID>, /*server id */
        Option<String>,
        clown_message_render::message_content::MessageKind,
        String,
    ),
    PrivMsg(
        ServerID, /*server id */
        IrcName,  /*source */
        IrcName,  /*target*/
        String,   /*content*/
    ),
    Notice(
        ServerID, /*server id */
        IrcName,  /*source */
        IrcName,  /*target*/
        String,   /*content*/
    ),
    ActionMsg(
        ServerID, /*server id */
        IrcName,  /*source */
        IrcName,  /*target*/
        String,   /*content*/
    ),
    SelectChannel(Option<ServerID> /*server id */, IrcName),
    UpdateUsers(
        ServerID,     /*server id */
        IrcName,      /*channel*/
        Vec<IrcName>, /*list users */
    ),
    ReplaceUser(
        ServerID, /*server id */
        IrcName,  /*old */
        IrcName,  /*new */
    ),
    Join(
        ServerID, /*server id */
        IrcName,  /*channel*/
        IrcName,  /*user */
    ),
    JoinServer(ServerID /*server id */),
    SetTopic(
        ServerID,       /*server id */
        Option<String>, /*source*/
        IrcName,        /*channel */
        String,         /*topic */
    ),
    HoverURL(String /* URL */),
    #[allow(dead_code)]
    Hover(String), //currently not used, but the skeleton can be used anywhere
    Connect(ServerID),
    DisConnect(ServerID),
    OpenWeb(String),
    SpellChecker(Option<String>),
    Part(
        ServerID, /*server id */
        IrcName,  /*channel */
        IrcName,  /*user */
    ),
    Quit(
        ServerID,       /*server id */
        IrcName,        /*user*/
        Option<String>, /*reason*/
    ),
    QuitAll(Option<String> /*reason*/),
    SettingsDidChange,
    Bel,
    CloseBuffer(Option<ServerID> /*server id */, IrcName /*name */),
    Search(super::irc_view::search_widget::Query),
    SearchEnd(),
    Config(ConfigEvent),
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
            clown_message_render::message_content::MessageKind::Error,
            format!("{}", value),
        )
    }
}
