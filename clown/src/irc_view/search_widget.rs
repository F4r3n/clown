use crate::irc_view::discuss::message_collection::{Anchor, MessageCollection, MessageViewer};
use crate::irc_view::input::text_input;
use crate::log::message_logger::{LogReader, MessageLogger};
use crate::message_event::MessageEvent;
use clown_message_render::message_content::{MessageContent, TimeFormat};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use std::path::PathBuf;
use std::range::Range;
use tokio::{sync::mpsc, task::JoinHandle};
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
    message: MessageContent,
}

struct DisplayItem {
    search_item: SearchItemResult,
}

impl MessageCollection for Vec<DisplayItem> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    // Most recent first: must match the order of `iter`
    fn get(&self, index: usize) -> Option<&MessageContent> {
        Vec::len(self)
            .checked_sub(index.saturating_add(1))
            .and_then(|i| self.as_slice().get(i))
            .map(|v| &v.search_item.message)
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &MessageContent> {
        self.as_slice().iter().map(|v| &v.search_item.message)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
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
            if let Ok(message) = message
                && let Some(content) = message.get_message().content()
            {
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

                let mut content = content.to_string();

                for position in positions.iter().rev() {
                    content = clown_message_render::message_parser::insert_irc_color(
                        content,
                        position,
                        &ratatui::style::Style::default().fg(ratatui::style::Color::LightBlue),
                    );
                }

                let _ = producer
                    .send(SearchItemResult {
                        message: MessageContent::message(
                            message.get_message().source().map(|v| v.to_string()),
                            content,
                        )
                        .with_time(message.get_message().time),
                    })
                    .await;
                if limit.is_some_and(|l| count >= l.into()) {
                    break;
                }
            }
            tokio::task::yield_now().await;
        }
    }
    last_offset
}

pub struct SearchWidget {
    message_viewer: MessageViewer,

    result: SearchResult,
    need_redraw: bool,
    area: ratatui::prelude::Rect,

    searcher: Option<AsyncSearch>,
    receiver: Option<mpsc::Receiver<SearchItemResult>>,
    input: text_input::InputWidget,
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
        if let Some(layout_message) = main_layout.first() {
            self.message_viewer.render(
                &self.result.items,
                |s| ctx.model.get_color(s),
                true,
                frame,
                *layout_message,
            );
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
    pub fn new() -> Self {
        Self {
            message_viewer: MessageViewer::new(Some(TimeFormat::Day)).with_anchor(Anchor::Top),
            result: SearchResult::default(),
            need_redraw: true,
            area: ratatui::prelude::Rect::default(),
            searcher: None,
            receiver: None,
            input: text_input::InputWidget::default(),
        }
    }

    fn default_paragraph(&self) -> ratatui::widgets::Paragraph<'_> {
        ratatui::widgets::Paragraph::new(Line::from(vec![
            Span::from("> ")
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan)),
            Span::from(self.input.get_value()),
        ]))
    }

    // The viewer is anchored at the top (most recent result):
    // scrolling down moves away from the anchor, toward older results.
    fn scroll_up(&mut self) {
        self.message_viewer.scroll_toward_no_check(1);
    }

    fn page_up(&mut self) {
        self.message_viewer
            .scroll_toward_no_check(self.message_viewer.get_max_visible_height());
    }

    fn page_down(&mut self) {
        self.message_viewer.scroll_page_away(&self.result.items);
    }

    fn scroll_down(&mut self) {
        if self
            .message_viewer
            .possible_scroll_away(&self.result.items, 1)
            > 0
        {
            self.message_viewer.scroll_away_no_check(1);
        }
    }

    fn search(&mut self, query: Query, ctx: &mut crate::state::context::Ctx) {
        let (producer, receiver) = mpsc::channel(10);
        self.result.clear();
        self.message_viewer.reset_scroll();
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
                self.message_viewer.add_line_scroll();
            }
        }
        use futures::FutureExt;

        if let Some(searcher) = self.searcher.as_mut()
            && searcher.task.as_mut().is_some_and(|t| t.is_finished())
            && let Some(Ok(pos)) = searcher.task.take().and_then(|t| t.now_or_never())
        {
            self.result.file_position = pos;
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
                    KeyCode::PageUp => {
                        self.page_up();
                        None
                    }
                    KeyCode::PageDown => {
                        self.page_down();
                        None
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
        self.need_redraw || self.message_viewer.need_redraw()
    }
}
