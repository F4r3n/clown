use std::io::Cursor;
use std::ops::Mul;

use crate::{component::Draw, message_event::MessageEvent};
use chrono::Local;
use image::DynamicImage;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};
use scraper::Html;
use std::sync::Arc;
use tokio::{
    runtime::Handle,
    task::{JoinHandle, block_in_place},
};

#[derive(Clone)]
struct MetaData {
    image_url: String,
    title: String,
    description: String,
    site: String,
    image: Option<Arc<DynamicImage>>,
}

impl MetaData {
    pub fn new(in_html: Html) -> Self {
        let mut meta = MetaData {
            image_url: String::from(""),
            title: String::from(""),
            description: String::from(""),
            site: String::from(""),
            image: None,
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

fn parse_html(text: &str, is_meta: bool) -> MetaData {
    if !is_meta {
        return MetaData {
            image_url: String::new(),
            title: String::new(),
            description: String::new(),
            site: String::new(),
            image: None,
        };
    }
    let document = Html::parse_document(text);
    MetaData::new(document)
}

async fn get_url_preview(endpoint: &str) -> Result<MetaData, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let headers = resp.headers();
    let has_meta = headers
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map_or(false, |ct| !ct.starts_with("image"));

    let text = resp.text().await.map_err(|e| e.to_string())?;
    let mut metadata = if has_meta {
        parse_html(&text, has_meta)
    } else {
        MetaData {
            image_url: String::from(endpoint),
            title: String::from(""),
            description: String::from(""),
            site: String::from(""),
            image: None,
        }
    };

    if let Some(bytes) = client
        .get(metadata.image_url.clone())
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .ok()
        .and_then(|bytes| Some(bytes))
    {
        metadata.image = Some(Arc::new(
            image::ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()
                .map_err(|e| e.to_string())?
                .decode()
                .map_err(|e| e.to_string())?,
        ));
    }

    Ok(metadata)
}

pub struct WebsitePreview {
    url: String,
    handle: Option<JoinHandle<Result<MetaData, String>>>,
    image: Option<StatefulProtocol>,
    picker: Option<Picker>,

    metadata: Option<MetaData>, //handle: Option<JoinHandle<>,
}
impl WebsitePreview {
    pub fn from_url(url: &str) -> Self {
        Self {
            url: url.to_string(),
            handle: None,
            metadata: None,
            image: None,
            picker: Picker::from_query_stdio().ok(),
        }
    }

    pub fn fetch_preview(&mut self) {
        let handle = Handle::current();
        let url = self.url.clone();
        //let url = "https://ogp.me/".to_string();
        self.handle = Some(handle.spawn(async move { get_url_preview(&url).await }));
    }

    pub fn has_finished(&self) -> bool {
        if let Some(handle) = &self.handle {
            handle.is_finished()
        } else {
            true
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
            if let Some(meta) = &mut self.metadata
                && let Some(dyn_image) = meta.image.take()
            {
                self.image = Some(in_picker.new_resize_protocol((*dyn_image).clone()));
            }

            self.metadata.clone()
        } else {
            None
        }
    }
}

impl Draw for WebsitePreview {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if let Some(picker) = self.picker.clone() {
            if let Some(meta) = self.get_metadata(&picker) {
                frame.render_widget(ratatui::widgets::Clear, area);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
                    .style(Style::default().bg(Color::Black));

                let inner_area = block.inner(area);
                frame.render_widget(block, area);

                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),       // Title
                        Constraint::Percentage(100), // Image
                    ])
                    .split(inner_area);

                let span = ratatui::text::Span::raw(meta.title);
                frame.render_widget(span, main_layout[0]);

                if let Some(image_protocol) = &mut self.image {
                    frame.render_stateful_widget(
                        StatefulImage::new(),
                        main_layout[1],
                        image_protocol,
                    );
                }
            }
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
}

impl Draw for ToolTipDiscussWidget {
    fn render(&mut self, frame: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect) {
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
        _event: &crate::event_handler::Event,
    ) -> Option<crate::message_event::MessageEvent> {
        None
    }
}
