use crate::event_handler::Event;
use crate::message_event::MessageEvent;
use crate::message_queue::MessageQueue;
use crate::model::Model;
use ratatui::Frame;
pub trait WidgetView {
    fn view(&mut self, model: &mut Model, frame: &mut Frame<'_>);
    fn handle_event(&mut self, model: &mut Model, event: &Event, messages: &mut MessageQueue);
    fn update(&mut self, model: &mut Model, msg: MessageEvent, messages: &mut MessageQueue);
}
