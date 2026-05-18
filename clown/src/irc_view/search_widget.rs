use crate::irc_view::input::text_input;
use crate::message_event::MessageEvent;
use crate::message_irc::log::message_logger::{LogReader, MessageLogger};
use crate::message_irc::message_content::{MessageContent, TimeFormat};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::Row;
use std::{ops::Range, path::PathBuf};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::debug;
#[derive(Debug, Default, PartialEq, Clone)]
pub struct QueryOption {
    pub server_id: Option<ServerID>,
    pub channel: Option<String>,
    pub from: Option<String>,
    pub start_time: Option<std::time::SystemTime>,
    pub end_time: Option<std::time::SystemTime>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Query {
    pub to_search: String,
    pub option: Box<QueryOption>,
}

use crate::{
    component::{Draw, EventHandler},
    state::server_id::ServerID,
};
//Do I search only in files? I flush messages into logs then search in it. Can do in a thread
// If the user disable the logs? Use a coroutine
// If Not a lot of live messages, can do it in main thread
// Do not store position, the display will do a lot of IO too long. We search the first 100 messages
// Own the struct, memory wise is < 1MB

struct SearchItemResult {
    highlights: Vec<Range<usize>>,
    message: MessageContent,
}

struct DisplayItem {
    search_item: SearchItemResult,
}

#[derive(Default)]
struct SearchResult {
    items: Vec<DisplayItem>,
    selected: usize,
    file_position: u64,
}

impl SearchResult {
    pub fn clear(&mut self) {
        self.items.clear();
        self.selected = 0;
        self.file_position = 0;
    }
}

struct AsyncSearch {
    query: Query,

    task: Option<JoinHandle<u64>>,
}

async fn async_search(
    log_path: PathBuf,
    query: Query,
    position_to_start: Option<std::num::NonZeroU64>,
    limit: Option<std::num::NonZero<usize>>,
    producer: mpsc::Sender<SearchItemResult>,
) -> u64 {
    debug!("Search In thread {}", query.to_search);

    let mut count: usize = 0;
    let mut last_offset = 0;
    let file_path = log_path;
    let reader = if let Some(start) = position_to_start {
        LogReader::try_from_path_with_start_pos(file_path.as_path(), start.get())
    } else {
        LogReader::try_from_path(file_path.as_path())
    };
    if let Ok(mut log_reader) = reader {
        for message in log_reader.iter() {
            if let Ok(message) = message {
                if let Some(content) = message.get_message().content() {
                    debug!("Search in message {}", content.clone());
                    let positions = content
                        .match_indices(&query.to_search)
                        .map(|v| Range {
                            start: v.0,
                            end: v.0 + query.to_search.len(),
                        })
                        .collect::<Vec<Range<usize>>>();
                    if positions.is_empty() {
                        continue;
                    }
                    count = count.saturating_add(1);
                    last_offset = message.get_offset();
                    debug!("Pushed In thread {}", query.to_search);

                    let _ = producer
                        .send(SearchItemResult {
                            highlights: positions,
                            message: MessageContent::message(
                                message.get_message().source().map(|v| v.to_string()),
                                content.to_string(),
                            )
                            .with_time(message.get_message().time),
                        })
                        .await;
                    if limit.is_some_and(|l| count >= l.into()) {
                        break;
                    }
                }
            }
        }
    }
    last_offset
}

#[derive(Default)]
pub struct SearchWidget {
    area: ratatui::prelude::Rect,
    result: SearchResult,
    scroll_offset: usize,
    max_visible_height: u16,
    searcher: Option<AsyncSearch>,
    receiver: Option<mpsc::Receiver<SearchItemResult>>,
    input: text_input::InputWidget,
    need_redraw: bool,
}

//What to see
// The Right panel is hidden during the search, no topic too
//
// 2 panels
// top pannel all the results, only result ellipsis around. Should be on one line better to read
// One line one result. If a result is found multiple times on the same line just display it on multiple lines
//
// bottom pannel if selected opens context, 2 messages above, 2 messages after, can be increased. Full line
// Not like the DiscussWidget (no vertical line in the middle)?
//
//
// Maybe 2 phase, first only the top panel
//
// Shortcut: TODO

struct TableDimension {
    time: u16,
    nickname: u16,
    separator: u16,
}

impl TableDimension {
    fn new(time: u16, nickname: u16, separator: u16) -> Self {
        Self {
            time,
            nickname,
            separator,
        }
    }

    fn get_width(&self) -> u16 {
        self.time
            .saturating_add(self.nickname)
            .saturating_add(self.separator)
    }
}

impl Draw for SearchWidget {
    fn render(
        &mut self,
        ctx: &mut crate::state::context::Ctx,
        frame: &mut ratatui::prelude::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        self.need_redraw = false;
        self.area = area;
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(100), // Messages area
                Constraint::Length(2),       // Input area
            ])
            .split(frame.area());
        if let Some(layout_message) = main_layout.get(0) {
            let time_format = TimeFormat::Day;
            const NICKNAME_LENGTH: u16 = 10;
            const SEPARATOR_LENGTH: u16 = 2;
            let table_dimension =
                TableDimension::new(time_format.length(), NICKNAME_LENGTH, SEPARATOR_LENGTH);

            self.max_visible_height = layout_message.height;

            let content_width = self
                .area
                .width
                .saturating_sub(table_dimension.get_width())
                .saturating_sub(4_u16);

            let visible_rows = self.collect_visible_rows(
                &time_format,
                content_width,
                &table_dimension,
                &ctx.model,
            );

            //collect visible rows
            let table = ratatui::widgets::Table::new(
                visible_rows,
                [
                    Constraint::Length(time_format.length().saturating_add(1)), // time
                    Constraint::Length(NICKNAME_LENGTH.saturating_add(1)),      // nickname
                    Constraint::Length(1),                                      // separator
                    Constraint::Min(10),                                        // Content
                ],
            )
            .column_spacing(1);

            frame.render_widget(table, *layout_message)
        }
        if let Some(layout_input) = main_layout.get(1) {
            let width = layout_input.width.max(3) - 3;
            let scroll = self.input.compute_visual_scroll(width as usize);
            let input = self.default_paragraph();

            frame.render_widget(input.scroll((0, scroll as u16)), *layout_input);

            let x = self.input.visual_cursor().max(scroll) - scroll + 2;
            frame.set_cursor_position((layout_input.x + x as u16, layout_input.y))
        }
    }
}

impl SearchWidget {
    fn default_paragraph(&self) -> ratatui::widgets::Paragraph<'_> {
        ratatui::widgets::Paragraph::new(Line::from(vec![
            Span::from("> ")
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)),
            Span::from(self.input.get_value()),
        ]))
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.need_redraw = true;
    }

    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        self.need_redraw = true;
    }

    fn collect_visible_rows(
        &self,
        time_format: &TimeFormat,
        content_width: u16,
        table_dimension: &TableDimension,
        model: &crate::state::model::Model,
    ) -> Vec<Row<'_>> {
        let mut visible_rows = Vec::new();

        if content_width == 0 {
            return visible_rows;
        }

        let max_visible_height = (self.max_visible_height as usize).saturating_sub(2);

        let mut rows_to_skip_in_message = self.scroll_offset;
        let mut visible_rows_total = 0;

        for item in &self.result.items {
            let message = &item.search_item.message;
            let total_line_rows = message.wrapped_line_count(content_width as usize);

            if rows_to_skip_in_message >= total_line_rows {
                rows_to_skip_in_message -= total_line_rows;
                continue;
            }

            let rows_available = total_line_rows - rows_to_skip_in_message;
            let rows_remaining = max_visible_height.saturating_sub(visible_rows_total);
            let rows_to_take = rows_remaining.min(rows_available);

            if rows_to_take == 0 {
                break;
            }
            let color = message.get_source().map(|s| model.get_color(s));

            let rows = message
                .create_rows(
                    content_width,
                    color,
                    Some(time_format),
                    table_dimension.nickname,
                )
                .skip(rows_to_skip_in_message)
                .take(rows_to_take);

            visible_rows_total += rows_to_take;
            visible_rows.extend(rows);

            rows_to_skip_in_message = 0;

            if visible_rows_total >= max_visible_height {
                break;
            }
        }

        visible_rows
    }

    fn search(&mut self, query: Query, ctx: &mut crate::state::context::Ctx) {
        let (producer, receiver) = mpsc::channel(10);
        self.result.clear();
        self.receiver = Some(receiver);
        let server_address = query
            .option
            .server_id
            .and_then(|v| ctx.model.get_address(v));

        if let Some(server_address) = server_address {
            let log_dir = crate::project_path::ProjectPath::log_dir().unwrap_or(
                std::env::current_dir().unwrap_or(std::path::Path::new("").to_path_buf()),
            );
            let log_path = log_dir.join(MessageLogger::compute_filename(
                server_address,
                query.option.channel.as_deref(),
            ));
            self.searcher = Some(AsyncSearch {
                query: query.clone(),
                task: Some(tokio::spawn(async move {
                    async_search(log_path, query, None, None, producer).await
                })),
            })
        }
    }

    fn handle_tick(&mut self) {
        if let Some(receiver) = self.receiver.as_mut() {
            while let Ok(result) = receiver.try_recv() {
                self.need_redraw = true;
                self.result.items.push(DisplayItem {
                    search_item: result,
                });
            }
        }

        if let Some(searcher) = self.searcher.as_ref()
            && searcher.task.as_ref().is_some_and(|t| t.is_finished())
        {
            if let Some(searcher) = self.searcher.as_mut() {
                use futures::FutureExt;
                if let Some(Ok(pos)) = searcher.task.take().and_then(|t| t.now_or_never()) {
                    self.result.file_position = pos;
                }
            }
        }
    }
}

impl EventHandler for SearchWidget {
    fn get_area(&self) -> ratatui::prelude::Rect {
        self.area
    }

    fn handle_actions(
        &mut self,
        ctx: &mut crate::state::context::Ctx,
        event: &MessageEvent,
    ) -> Option<crate::message_event::MessageEvent> {
        match event {
            MessageEvent::Search(query) => {
                self.search(query.clone(), ctx);
                None
            }
            _ => None,
        }
    }

    fn handle_events(
        &mut self,
        _ctx: &mut crate::state::context::Ctx,
        event: &crate::event_handler::Event,
    ) -> Option<MessageEvent> {
        match event {
            crate::event_handler::Event::Tick => {
                self.handle_tick();
                None
            }
            crate::event_handler::Event::Crossterm(crossterm_event) => match crossterm_event {
                crossterm::event::Event::Key(key) => match key.code {
                    KeyCode::Esc => Some(MessageEvent::SearchEnd()),
                    KeyCode::Enter => {
                        if let Some(searcher) = self.searcher.as_ref() {
                            let mut previous_query = searcher.query.clone();
                            previous_query.to_search = self.input.get_value().to_string();
                            Some(MessageEvent::Search(previous_query))
                        } else {
                            None
                        }
                    }

                    _ => {
                        self.input.handle_key_events(key);
                        self.need_redraw = true;
                        None
                    }
                },
                crossterm::event::Event::Resize(_x, _y) => {
                    self.need_redraw = true;
                    None
                }
                crossterm::event::Event::Mouse(mouse_event) => match mouse_event.kind {
                    crossterm::event::MouseEventKind::ScrollDown => {
                        self.scroll_down();
                        None
                    }
                    crossterm::event::MouseEventKind::ScrollUp => {
                        self.scroll_up();
                        None
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn need_redraw(&self) -> bool {
        self.need_redraw
    }
}
