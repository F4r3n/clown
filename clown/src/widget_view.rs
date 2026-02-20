use crate::event_handler::Event;
use crate::irc_view::session::Session;
use crate::message_event::MessageEvent;
use crate::message_queue::MessageQueue;
use crate::model::Model;
use ratatui::Frame;
pub trait WidgetView {
    fn view(&mut self, model: &mut Model, session: &mut Session, frame: &mut Frame<'_>);
    fn handle_event(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        event: &Event,
        messages: &mut MessageQueue,
    );
    fn need_redraw(&mut self, model: &mut Model) -> bool;
    fn update(
        &mut self,
        model: &mut Model,
        session: &mut Session,
        msg: MessageEvent,
        messages: &mut MessageQueue,
    );
}
