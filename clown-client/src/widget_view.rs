use crate::MessageEvent;
use crate::event_handler::Event;
use crate::model::Model;
use ratatui::Frame;
pub trait WidgetView {
    fn view(&mut self, model: &mut Model, frame: &mut Frame);
    fn handle_event(
        &mut self,
        model: &mut Model,
        event: &Event,
    ) -> color_eyre::Result<Option<MessageEvent>>;

    fn update(&mut self, model: &mut Model, msg: MessageEvent) -> Option<MessageEvent>;
}
