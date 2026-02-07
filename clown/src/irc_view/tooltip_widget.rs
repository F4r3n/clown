#[cfg(feature = "website-preview")]
use crate::irc_view::website_preview::WebsitePreview;
use crate::{component::Draw, irc_view::irc_model, message_event::MessageEvent};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use ratatui_image::picker::Picker;

pub trait DrawToolTip: Draw {
    fn need_redraw(&self) -> bool;
}

struct MessagePreview {
    content: String,
    need_redraw: bool,
}

impl MessagePreview {
    pub fn from_string(content: String) -> Self {
        Self {
            content,
            need_redraw: true,
        }
    }
}

impl Draw for MessagePreview {
    fn render(&mut self, _irc_model: &irc_model::IrcModel, frame: &mut Frame<'_>, area: Rect) {
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

            if self.need_redraw {
                self.need_redraw = false;
            }
        }
    }
}

impl DrawToolTip for MessagePreview {
    fn need_redraw(&self) -> bool {
        self.need_redraw
    }
}

pub struct ToolTipDiscussWidget {
    area: ratatui::prelude::Rect,
    preview: Option<Box<dyn DrawToolTip>>,
    start_time: Option<std::time::Instant>,
    picker: Option<Picker>,
    need_redraw: bool,
    is_opened: bool,
}

impl ToolTipDiscussWidget {
    pub fn new() -> Self {
        Self {
            area: ratatui::prelude::Rect::default(),
            start_time: None,
            preview: None,
            picker: Picker::from_query_stdio().ok(),
            need_redraw: true,
            is_opened: false,
        }
    }

    fn is_open(&self) -> bool {
        if let Some(start_time) = self.start_time {
            start_time.elapsed() < std::time::Duration::from_secs(5)
        } else {
            false
        }
    }

    pub fn start_timer(&mut self) {
        self.start_time = Some(std::time::Instant::now());
    }

    pub fn set_message(&mut self, preview: Box<dyn DrawToolTip>) {
        self.preview = Some(preview);
        self.start_timer();
    }
}

impl Draw for ToolTipDiscussWidget {
    fn render(
        &mut self,
        irc_model: &irc_model::IrcModel,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        if !self.is_open() {
            self.need_redraw = false;
            return;
        }

        let area_to_render = ratatui::prelude::Rect {
            height: std::cmp::min(area.height, 10),
            width: std::cmp::min(area.width, 30),
            x: area.x,
            y: area.y,
        };

        self.area = area_to_render;
        if let Some(preview) = &mut self.preview {
            self.is_opened = true;
            preview.render(irc_model, frame, area_to_render);
            self.need_redraw = preview.need_redraw();
        }
    }
}

impl crate::component::EventHandler for ToolTipDiscussWidget {
    fn get_area(&self) -> ratatui::prelude::Rect {
        self.area
    }
    fn need_redraw(&self) -> bool {
        self.need_redraw
    }
    fn handle_actions(
        &mut self,
        _irc_model: &irc_model::IrcModel,
        event: &crate::message_event::MessageEvent,
    ) -> Option<crate::message_event::MessageEvent> {
        match event {
            #[cfg(feature = "website-preview")]
            MessageEvent::HoverURL(content) => {
                if !self.is_open()
                    && let Some(picker) = self.picker.clone()
                {
                    let mut preview = WebsitePreview::from_url(content, picker);
                    preview.fetch_preview();
                    self.set_message(Box::new(preview));
                    self.need_redraw = true;
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
        if self.is_opened && !self.is_open() {
            self.need_redraw = true;
            self.is_opened = false;
        }
        None
    }
}
