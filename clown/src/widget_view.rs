use crate::message_event::MessageEvent;
use crate::message_queue::MessageQueue;
use crate::state::model::Model;
use crate::wake_event::WakeEvent;
use ratatui::Frame;
pub trait WidgetView {
    fn view(&mut self, ctx: &mut crate::state::context::Ctx, frame: &mut Frame<'_>);
    fn handle_event(
        &mut self,
        ctx: &mut crate::state::context::Ctx,
        event: &WakeEvent,
        messages: &mut MessageQueue,
    );
    fn need_redraw(&mut self, model: &mut Model) -> bool;
    fn update(
        &mut self,
        ctx: &mut crate::state::context::Ctx,
        msg: MessageEvent,
        messages: &mut MessageQueue,
    );
}
