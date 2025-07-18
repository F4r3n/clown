use crate::component::Draw;
use crate::{MessageEvent, component::EventHandler};
use chrono::{DateTime, Local, Timelike};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Styled},
    widgets::{
        Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    },
};

#[derive(PartialEq, Debug, Clone)]
pub struct MessageContent {
    time: std::time::SystemTime, /*Generated time */
    source: Option<String>,      /*Source*/
    content: String,             /*Content */
}

impl MessageContent {
    pub fn new(source: Option<String>, content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source,
            content,
        }
    }

    fn time_format(&self) -> String {
        let datetime: DateTime<Local> = self.time.into();

        // Format as HH:MM:SS
        let formatted_time = format!(
            "{:02}:{:02}:{:02}",
            datetime.hour(),
            datetime.minute(),
            datetime.second()
        );
        formatted_time
    }
}

pub struct TextWidget {
    vertical_scroll_state: ScrollbarState,
    content: Vec<MessageContent>,
    scroll_offset: usize,
    max_visible_height: usize,
    follow_last: bool,
    focus: bool,
}

impl Draw for TextWidget {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focus;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let text_style = Style::default().fg(Color::White);

        // Set how many lines can be shown
        self.max_visible_height = area.height.saturating_sub(2) as usize;
        let max_scroll = self.content.len().saturating_sub(self.max_visible_height);
        let scroll = self.scroll_offset.min(max_scroll);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20), // Meta (time + source)
                Constraint::Length(1),  // Separator "┃"
                Constraint::Min(10),    // Message
                Constraint::Length(1),  // Scrollbar
            ])
            .split(area);

        let visible_rows: Vec<Row> = self
            .content
            .iter()
            .skip(scroll)
            .take(self.max_visible_height)
            .map(|line| {
                let time_str = format!("{:>8}", line.time_format());
                let source_str = line.source.clone().unwrap_or_default();
                let meta = format!("{time_str:<8} {source_str:<10}");
                Row::new(vec![Cell::from(meta), Cell::from(line.content.clone())])
            })
            .collect();

        let mut table = Table::new(
            visible_rows,
            [
                Constraint::Length(20), // Meta
                Constraint::Min(10),    // Content
            ],
        )
        .column_spacing(1)
        .style(text_style);
        if self.has_focus() {
            table = table
                .block(Block::bordered().title("Messages"))
                .set_style(border_style);
        }

        let height = layout[1].height as usize;
        let binding = "┃\n".repeat(height);
        let vertical_line = binding.trim_end(); // trim to avoid extra line
        let line_paragraph = Paragraph::new(vertical_line)
            .alignment(Alignment::Center) // optional
            .style(Style::default().fg(Color::DarkGray));

        self.vertical_scroll_state = ScrollbarState::new(self.content.len())
            .position(self.scroll_offset + self.max_visible_height);

        frame.render_widget(
            table,
            Rect {
                x: layout[0].x,
                y: layout[0].y,
                width: layout[0].width + layout[2].width, // table covers meta + content
                height: layout[0].height,
            },
        );
        frame.render_widget(line_paragraph, layout[1]);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().bg(Color::Cyan)),
            layout[3],
            &mut self.vertical_scroll_state,
        );
    }
}

impl crate::component::EventHandler for TextWidget {
    fn has_focus(&self) -> bool {
        self.focus
    }

    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::AddMessageView(content) => {
                self.add_line(content.clone());
                None
            }
            _ => None,
        }
    }

    fn set_focus(&mut self, focused: bool) {
        self.focus = focused;
    }
    fn handle_events(&mut self, event: &crate::event_handler::Event) -> Option<MessageEvent> {
        if let Some(key) = event.get_key() {
            match key.code {
                KeyCode::Up => {
                    self.scroll_up();
                    None
                }
                KeyCode::PageUp => {
                    for _ in 0..5 {
                        self.scroll_up();
                    }
                    None
                }
                KeyCode::Down => {
                    self.scroll_down();
                    None
                }
                KeyCode::PageDown => {
                    for _ in 0..5 {
                        self.scroll_down();
                    }
                    None
                }
                KeyCode::Home => {
                    self.scroll_offset = 0;
                    None
                }
                KeyCode::End => {
                    self.scroll_offset = self.content.len();
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

impl TextWidget {
    pub fn new(content: Vec<MessageContent>) -> Self {
        Self {
            content,
            focus: false,
            scroll_offset: 0,
            max_visible_height: 10,
            follow_last: true,
            vertical_scroll_state: ScrollbarState::default(),
        }
    }

    pub fn add_line(&mut self, line: MessageContent) {
        self.content.push(line);
        if self.follow_last {
            // Show last lines that fit the view
            self.scroll_offset = self.content.len().saturating_sub(self.max_visible_height);
        }
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.follow_last = false;
    }

    fn scroll_down(&mut self) {
        let max_scroll = self.content.len().saturating_sub(self.max_visible_height);
        self.scroll_offset = self.scroll_offset.saturating_add(1).min(max_scroll);
        self.follow_last = max_scroll.eq(&self.scroll_offset);
    }
}
