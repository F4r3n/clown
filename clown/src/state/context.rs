use crate::irc_view::discuss::servers_messages::ServersMessages;
use super::model::Model;
use super::session::Session;

pub struct Ctx {
    pub session: Session,
    pub model: Model,
    pub messages: ServersMessages,
}
