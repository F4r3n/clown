use std::ops::Mul;

use crate::{component::Draw, message_event::MessageEvent};
use chrono::Local;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::{
    runtime::Handle,
    task::{JoinHandle, block_in_place},
};
use tracing::info;

use scraper::Html;
#[derive(Clone, serde::Serialize)]
struct MetaData {
    image_url: String,
    title: String,
    description: String,
    site: String,
    image_only: bool,
}

impl MetaData {
    pub fn new(in_html: Html) -> Self {
        let mut meta = MetaData {
            image_url: String::from(""),
            title: String::from(""),
            description: String::from(""),
            site: String::from(""),
            image_only: false,
        };
        use scraper::Selector;
        if let Ok(selector) = Selector::parse("head meta") {
            let s = in_html.select(&selector);
            for element in s {
                if let Some(property) = element.attr("property") {
                    if let Some(content) = element.attr("content") {
                        match property {
                            "og:title" => {
                                meta.title = String::from(content);
                            }
                            "og:image" => {
                                meta.image_url = String::from(content);
                            }
                            "og:description" => {
                                meta.description = String::from(content);
                            }
                            "og:site" => {
                                meta.site = String::from(content);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        meta
    }
}

async fn get_url_preview(endpoint: &str) -> Result<MetaData, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let headers = resp.headers();
    let mut has_meta = false;

    if let Some(content) = headers.get("content-type") {
        has_meta = !content
            .to_str()
            .map_err(|e| e.to_string())?
            .starts_with("image");
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let document = Html::parse_document(text.as_str());
    if has_meta {
        Ok(MetaData::new(document))
    } else {
        Ok(MetaData {
            image_url: String::from(endpoint),
            title: String::from(""),
            description: String::from(""),
            site: String::from(""),
            image_only: true,
        })
    }
}

pub struct WebsitePreview {
    url: String,
    handle: Option<JoinHandle<Result<MetaData, String>>>,

    metadata: Option<MetaData>, //handle: Option<JoinHandle<>,
}
impl WebsitePreview {
    pub fn from_url(url: &str) -> Self {
        Self {
            url: url.to_string(),
            handle: None,
            metadata: None,
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
            true
        }
    }
    fn get_metadata(&mut self) -> Option<MetaData> {
        if self.metadata.is_some() {
            return self.metadata.clone();
        }

        if !self.has_finished() {
            return self.metadata.clone();
        }
        if let Some(join_handle) = self.handle.take() {
            let data = block_in_place(|| {
                let handle = Handle::current();
                match handle.block_on(async { join_handle.await }) {
                    Ok(Ok(meta)) => Some(meta),
                    Ok(Err(err)) => {
                        eprintln!("Metadata fetch error: {err}");
                        None
                    }
                    Err(join_err) => {
                        eprintln!("Join error: {join_err}");
                        None
                    }
                }
            });
            self.metadata = data;
            self.metadata.clone()
        } else {
            None
        }
    }
}

pub struct ToolTipDiscussWidget {
    area: ratatui::prelude::Rect,
    preview: Option<WebsitePreview>,
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

    pub fn set_message(&mut self, message: &str) {
        //info!("New message");
        self.preview = Some(WebsitePreview::from_url(message));
        self.start_timer();
        if let Some(preview) = &mut self.preview {
            preview.fetch_preview();
        }
    }

    fn check_preview(&mut self) -> Option<MetaData> {
        if let Some(preview) = &mut self.preview {
            preview.get_metadata()
        } else {
            None
        }
    }
}

impl Draw for ToolTipDiscussWidget {
    fn render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect) {
        return;
        if !self.is_open() {
            return;
        }
        let area_to_render = ratatui::prelude::Rect {
            height: (area.height as f32).mul(0.2) as u16,
            width: (area.width as f32).mul(0.2) as u16,
            x: area.x,
            y: area.y,
        };

        self.area = area_to_render;
        let preview = self.check_preview();
        let overlay_style = Style::default().bg(Color::Rgb(0, 0, 0)).fg(Color::White);

        let tooltip = Paragraph::new("Tooltip")
            .style(overlay_style)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(tooltip, area_to_render);
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
            MessageEvent::Hover(content) => {
                if !self.is_open() {
                    self.set_message(content);
                }
                None
            }

            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        None
    }
}
