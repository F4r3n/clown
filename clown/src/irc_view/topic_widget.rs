use crate::component::{Draw, EventHandler};
use crate::model::ServerID;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

pub struct TopicWidget {
    topic_collection: Vec<ahash::AHashMap<String, String>>,
    need_redraw: bool,
}

impl Draw for TopicWidget {
    fn render(
        &mut self,
        ctx: &mut crate::context::Ctx,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        if self.need_redraw {
            self.need_redraw = false;
        }
        let model = &ctx.session.model;
        if let Some(server_model) = model.get_current_server()
            && let Some(channel) = server_model.get_current_channel()
            && let Some(topic) = self.get_topic(server_model.get_server_id(), channel)
        {
            let text = Text::from(topic);
            let paragrapth = Paragraph::new(text);
            frame.render_widget(paragrapth, area);
        }
    }
}

impl TopicWidget {
    pub fn new() -> Self {
        Self {
            topic_collection: Vec::new(),
            need_redraw: true,
        }
    }

    fn get_topic(&self, server_id: ServerID, channel: &str) -> Option<&str> {
        if let Some(list_topics) = self.topic_collection.get(server_id.as_usize()) {
            list_topics.get(channel).map(|v| v.as_str())
        } else {
            None
        }
    }

    fn update_topic(&mut self, server_id: ServerID, channel: &str, content: String) {
        if let Some(server) = self.topic_collection.get_mut(server_id.as_usize()) {
            server.insert(channel.to_string(), content);
        }
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
        _ctx: &mut crate::context::Ctx,
        event: &MessageEvent,
    ) -> Option<MessageEvent> {
        match event {
            MessageEvent::SetTopic(server_id, _source, channel, topic) => {
                self.update_topic(*server_id, channel, topic.to_string());
                self.need_redraw = true;

                None
            }
            MessageEvent::JoinServer(server_id) => {
                self.topic_collection
                    .resize(server_id.as_usize().saturating_add(1), Default::default());
                None
            }
            MessageEvent::SelectChannel(_server_id, _channel) => {
                self.need_redraw = true;
                None
            }
            _ => None,
        }
    }
    fn handle_events(
        &mut self,
        _ctx: &mut crate::context::Ctx,
        _event: &crate::event_handler::Event,
    ) -> Option<MessageEvent> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_event::MessageEvent;

    #[test]
    fn test_new_topic_widget() {
        let widget = TopicWidget::new();
        assert!(widget.topic_collection.is_empty());
        assert!(widget.need_redraw());
    }

    #[test]
    fn test_join_server_resizes_collection() {
        let mut widget = TopicWidget::new();
        let server_id = ServerID::new(2);

        // Simulate joining a server with ID 2
        let event = MessageEvent::JoinServer(server_id);
        let mut ctx = mock_ctx();
        widget.handle_actions(&mut ctx, &event);

        // Vector should be resized to 3 to accommodate index 2
        assert_eq!(widget.topic_collection.len(), 3);
    }

    #[test]
    fn test_set_and_get_topic() {
        let mut widget = TopicWidget::new();
        let server_id = ServerID::new(0);
        let channel = "#rust";
        let topic = "Welcome to Rust IRC!";

        // Ensure the vector has space
        widget.topic_collection.push(ahash::AHashMap::new());

        // Update topic
        widget.update_topic(server_id, channel, topic.to_string());

        // Verify retrieval
        assert_eq!(widget.get_topic(server_id, channel), Some(topic));
        assert!(widget.need_redraw());
    }

    #[test]
    fn test_handle_set_topic_with_option_source() {
        let mut widget = TopicWidget::new();
        let server_id = ServerID::new(0);

        let mut ctx = mock_ctx();

        // 1. Initialize the server slot first
        widget.handle_actions(&mut ctx, &MessageEvent::JoinServer(server_id));

        // 2. Mock the SetTopic event with an Option source (Some)
        let event_with_source = MessageEvent::SetTopic(
            server_id,
            Some("NickServ".to_string()),
            "#rust".to_string(),
            "Crusty but trusty".to_string(),
        );

        widget.handle_actions(&mut ctx, &event_with_source);
        assert_eq!(
            widget.get_topic(server_id, "#rust"),
            Some("Crusty but trusty")
        );

        // 3. Mock the SetTopic event with None source
        let event_no_source = MessageEvent::SetTopic(
            server_id,
            None,
            "#rust".to_string(),
            "New Topic No Source".to_string(),
        );

        widget.handle_actions(&mut ctx, &event_no_source);
        assert_eq!(
            widget.get_topic(server_id, "#rust"),
            Some("New Topic No Source")
        );
    }

    #[test]
    fn test_get_topic_out_of_bounds() {
        let widget = TopicWidget::new();
        // Should return None safely even if server_id doesn't exist in the Vec
        assert_eq!(widget.get_topic(ServerID::new(99), "#anything"), None);
    }

    // Helper to satisfy the Draw/EventHandler traits in tests
    fn mock_ctx() -> crate::context::Ctx {
        crate::context::Ctx {
            model: crate::model::Model::new_empty_config(),
            session: crate::irc_view::session::Session::new(0),
            messages: crate::irc_view::discuss::servers_messages::ServersMessages::new(
                std::path::Path::new("").to_path_buf(),
            ),
        }
    }
}
