use crate::irc_view::discuss::servers_messages::ServersMessages;
use crate::irc_view::session::Session;
use crate::model::Model;
pub struct Ctx {
    pub session: Session,
    pub model: Model,
    pub messages: ServersMessages,
}
