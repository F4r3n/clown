use ahash::AHashMap;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::component::{Draw, EventHandler};

#[derive(Hash, PartialEq, Eq)]
struct KeyServerChannel {
    server_id: usize,
    channel: String,
}

pub struct TopicWidget {
    topic_collection: ahash::AHashMap<KeyServerChannel, String>,
    need_redraw: bool,
}

impl Draw for TopicWidget {
    fn render(
        &mut self,
        irc_model: Option<&crate::irc_view::irc_model::IrcModel>,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        if self.need_redraw {
            self.need_redraw = false;
        }

        if let Some(model) = irc_model.as_ref()
            && let Some(server_model) = model.get_current_server()
            && let Some(channel) = server_model.get_current_channel()
        {
            if let Some(topic) = self.topic_collection.get(&KeyServerChannel {
                channel: channel.to_string(),
                server_id: server_model.get_server_id(),
            }) {
                let text = Text::from(topic.clone());
                let paragrapth = Paragraph::new(text);
                frame.render_widget(paragrapth, area);
            }
        }
    }
}

impl TopicWidget {
    pub fn new() -> Self {
        Self {
            topic_collection: AHashMap::new(),
            need_redraw: true,
        }
    }

    fn update_topic(&mut self, server_id: usize, channel: &str, content: &str) {
        self.topic_collection.insert(
            KeyServerChannel {
                channel: channel.to_string(),
                server_id,
            },
            content.to_string(),
        );
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
    fn handle_actions(
        &mut self,
        _irc_model: Option<&crate::irc_view::irc_model::IrcModel>,
        event: &MessageEvent,
    ) -> Option<MessageEvent> {
        match event {
            MessageEvent::SetTopic(server_id, _source, channel, topic) => {
                self.update_topic(*server_id, channel, topic);
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
