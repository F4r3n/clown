use crate::event_handler::Event;
use crate::state::server_id::ServerID;
use clown_core::message::ServerMessage;

#[derive(Debug)]
pub enum WakeEvent {
    Event(Event),
    #[allow(clippy::upper_case_acronyms)]
    IRC((ServerID, ServerMessage)),
}
