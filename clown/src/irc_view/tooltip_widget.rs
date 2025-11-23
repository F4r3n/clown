#[cfg(feature = "website-preview")]
use crate::irc_view::website_preview::WebsitePreview;
use crate::{component::Draw, message_event::MessageEvent};
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use std::ops::Mul;

struct MessagePreview {
    content: String,
}

impl MessagePreview {
    pub fn from_string(content: String) -> Self {
        Self { content }
    }
}

impl Draw for MessagePreview {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(ratatui::widgets::Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .style(Style::default().bg(Color::Black));
        let inner_area = block.inner(area);

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10), // Title
            ])
            .split(inner_area);
        let mut title = self.content.clone();

        if let Some(title_layout) = main_layout.first() {
            let width = title_layout.width as usize;
            if title.chars().count() > width {
                // Keep room for "..."
                title = title
                    .chars()
                    .take(width.saturating_sub(3))
                    .collect::<String>()
                    + "...";
            }
            let span = ratatui::text::Span::raw(title);
            frame.render_widget(span, *title_layout);
        }
    }
}

pub struct ToolTipDiscussWidget {
    area: ratatui::prelude::Rect,
    preview: Option<Box<dyn Draw>>,
    start_time: Option<chrono::DateTime<Local>>,
    end_time: Option<chrono::DateTime<Local>>,
}

impl ToolTipDiscussWidget {
    pub fn new() -> Self {
        Self {
            area: ratatui::prelude::Rect::default(),
            end_time: None,
            start_time: None,
            preview: None,
        }
    }

    fn is_open(&self) -> bool {
        if let Some(start_time) = self.start_time
            && let Some(end_time) = self.end_time
        {
            let current_time = chrono::Local::now();
            current_time >= start_time && current_time <= end_time
        } else {
            false
        }
    }

    pub fn start_timer(&mut self) {
        self.start_time = Some(Local::now());
        self.end_time = self
            .start_time
            .map(|time| time + chrono::Duration::seconds(5));
    }

    pub fn set_message(&mut self, preview: Box<dyn Draw>) {
        self.preview = Some(preview);
        self.start_timer();
    }
}

impl Draw for ToolTipDiscussWidget {
    fn render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect) {
        if !self.is_open() {
            return;
        }
        let area_to_render = ratatui::prelude::Rect {
            height: f32::from(area.height).mul(0.3) as u16,
            width: f32::from(area.width).mul(0.3) as u16,
            x: area.x,
            y: area.y,
        };

        self.area = area_to_render;
        if let Some(preview) = &mut self.preview {
            preview.render(frame, area_to_render);
        }
    }
}

impl crate::component::EventHandler for ToolTipDiscussWidget {
    fn get_area(&self) -> ratatui::prelude::Rect {
        self.area
    }

    fn handle_actions(
        &mut self,
        event: &crate::message_event::MessageEvent,
    ) -> Option<crate::message_event::MessageEvent> {
        match event {
            #[cfg(feature = "website-preview")]
            MessageEvent::HoverURL(content) => {
                if !self.is_open() {
                    let mut preview = WebsitePreview::from_url(content);
                    preview.fetch_preview();
                    self.set_message(Box::new(preview));
                }

                None
            }
            MessageEvent::Hover(content) => {
                if !self.is_open() {
                    self.set_message(Box::new(MessagePreview::from_string(content.clone())));
                }
                None
            }

            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        _event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        None
    }
}
