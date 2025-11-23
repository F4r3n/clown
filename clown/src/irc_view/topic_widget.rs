use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::component::{Draw, EventHandler};

pub struct TopicWidget {
    topic: String,
}

impl Draw for TopicWidget {
    fn render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect) {
        let text = Text::from(self.topic.clone());
        let paragrapth = Paragraph::new(text);
        frame.render_widget(paragrapth, area);
    }
}

impl TopicWidget {
    pub fn new() -> Self {
        Self {
            topic: "".to_string(),
        }
    }

    fn set_topic(&mut self, content: &str) {
        self.topic = content.to_string();
    }
}

use crate::message_event::MessageEvent;
impl EventHandler for TopicWidget {
    fn get_area(&self) -> ratatui::prelude::Rect {
        Rect::default()
    }
    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::SetTopic(topic) => {
                self.set_topic(topic);
                None
            }
            _ => None,
        }
    }
    fn handle_events(&mut self, _event: &crate::event_handler::Event) -> Option<MessageEvent> {
        None
    }
}
