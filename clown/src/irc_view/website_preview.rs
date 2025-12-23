use {
    clown_website_preview::{MetaData, get_url_preview},
    ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol},
    tokio::{
        runtime::Handle,
        task::{JoinHandle, block_in_place},
    },
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};

use crate::component::Draw;

pub struct WebsitePreview {
    url: String,
    handle: Option<JoinHandle<Result<MetaData, String>>>,
    image: Option<StatefulProtocol>,
    picker: Picker,

    metadata: Option<MetaData>, //handle: Option<JoinHandle<>,
}

impl WebsitePreview {
    pub fn from_url(url: &str, picker: Picker) -> Self {
        Self {
            url: url.to_string(),
            handle: None,
            metadata: None,
            image: None,
            picker,
        }
    }

    pub fn fetch_preview(&mut self) {
        let handle = Handle::current();
        let url = self.url.clone();
        self.handle = Some(handle.spawn(async move { get_url_preview(&url).await }));
    }

    pub fn has_finished(&self) -> bool {
        if let Some(handle) = &self.handle {
            handle.is_finished()
        } else {
            false
        }
    }
    fn get_metadata(&mut self, in_picker: &Picker) -> Option<MetaData> {
        if self.metadata.is_some() {
            return self.metadata.clone();
        }

        if !self.has_finished() {
            return self.metadata.clone();
        }
        if let Some(join_handle) = self.handle.take() {
            let data = block_in_place(|| {
                let handle = Handle::current();
                match handle.block_on(join_handle) {
                    Ok(Ok(meta)) => Some(meta),
                    Ok(Err(_)) => None,
                    Err(_) => None,
                }
            });
            self.metadata = data;
            if let Some(meta) = &mut self.metadata
                && let Some(dyn_image) = meta.take_image()
            {
                self.image = Some(in_picker.new_resize_protocol((*dyn_image).clone()));
            }

            self.metadata.clone()
        } else {
            None
        }
    }
}

#[cfg(feature = "website-preview")]
impl Draw for WebsitePreview {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if let Some(meta) = self.get_metadata(&self.picker.clone()) {
            frame.render_widget(ratatui::widgets::Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
                .style(Style::default().bg(Color::Black));

            let inner_area = block.inner(area);
            frame.render_widget(block, area);
            if !meta.get_title().is_empty() {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),       // Title
                        Constraint::Percentage(100), // Image
                    ])
                    .split(inner_area);
                let mut title = meta.get_title().to_string();

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

                if let Some(image_layout) = main_layout.get(1)
                    && let Some(image_protocol) = &mut self.image
                {
                    frame.render_stateful_widget(
                        StatefulImage::new(),
                        *image_layout,
                        image_protocol,
                    );
                }
            } else if let Some(image_protocol) = &mut self.image {
                frame.render_stateful_widget(StatefulImage::new(), inner_area, image_protocol);
            }
        }
    }
}
