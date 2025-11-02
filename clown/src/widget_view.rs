use crate::MessageEvent;
use crate::MessageQueue;
use crate::event_handler::Event;
use crate::model::Model;
use ratatui::Frame;
pub trait WidgetView {
    fn view(&mut self, model: &mut Model, frame: &mut Frame<'_>);
    fn handle_event(&mut self, model: &mut Model, event: &Event, messages: &mut MessageQueue);

    fn update(&mut self, model: &mut Model, msg: MessageEvent, messages: &mut MessageQueue);
}
