use crate::event_handler::Event;
use crate::message_event::MessageEvent;
use crate::message_queue::MessageQueue;
use crate::model::Model;
use ratatui::Frame;
pub trait WidgetView {
    fn view(&mut self, ctx: &mut crate::context::Ctx, frame: &mut Frame<'_>);
    fn handle_event(
        &mut self,
        ctx: &mut crate::context::Ctx,
        event: &Event,
        messages: &mut MessageQueue,
    );
    fn need_redraw(&mut self, model: &mut Model) -> bool;
    fn update(
        &mut self,
        ctx: &mut crate::context::Ctx,
        msg: MessageEvent,
        messages: &mut MessageQueue,
    );
}
