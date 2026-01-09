use ahash::AHashMap;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::component::{Draw, EventHandler};

pub struct TopicWidget {
    topic_collection: ahash::AHashMap<String, String>,
    current_channel: Option<String>,
    need_redraw: bool,
}

impl Draw for TopicWidget {
    fn render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect) {
        if self.need_redraw {
            self.need_redraw = false;
        }
        if let Some(channel) = self.current_channel.as_ref()
            && let Some(topic) = self.topic_collection.get(channel)
        {
            let text = Text::from(topic.clone());
            let paragrapth = Paragraph::new(text);
            frame.render_widget(paragrapth, area);
        }
    }
}

impl TopicWidget {
    pub fn new() -> Self {
        Self {
            topic_collection: AHashMap::new(),
            current_channel: None,
            need_redraw: true,
        }
    }

    fn update_topic(&mut self, channel: &str, content: &str) {
        self.topic_collection
            .insert(channel.to_string(), content.to_string());
        self.need_redraw = true;
    }

    fn set_channel(&mut self, channel: String) {
        self.current_channel = Some(channel);
        self.need_redraw = true;
    }
}

use crate::message_event::MessageEvent;
impl EventHandler for TopicWidget {
    fn need_redraw(&self) -> bool {
        self.need_redraw
    }

    fn get_area(&self) -> ratatui::prelude::Rect {
        Rect::default()
    }
    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::SetTopic(channel, topic) => {
                self.update_topic(channel, topic);
                self.need_redraw = true;
                None
            }
            MessageEvent::SelectChannel(channel) => {
                self.set_channel(channel.to_string());
                self.need_redraw = true;
                None
            }
            MessageEvent::Join(channel, _user) => {
                self.set_channel(channel.to_string());
                self.need_redraw = true;

                None
            }
            _ => None,
        }
    }
    fn handle_events(&mut self, _event: &crate::event_handler::Event) -> Option<MessageEvent> {
        None
    }
}
