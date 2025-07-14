use crate::MessageEvent;
use crate::component::Draw;
use chrono::{DateTime, Local, Timelike};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Styled},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
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
        let title = "Message";

        // Adjust visible height and cache for scrolling logic
        self.max_visible_height = area.height.saturating_sub(2) as usize;

        let max_scroll = self.content.len().saturating_sub(self.max_visible_height);
        let scroll = self.scroll_offset.min(max_scroll);

        let time_width = 8;
        let source_width = 9;

        let visible_lines: Vec<Line> = self
            .content
            .iter()
            .skip(scroll)
            .take(self.max_visible_height)
            .map(|line| {
                let mut spans = vec![];

                // Pad time (right align)
                let time_str = format!("{:>width$}", line.time_format(), width = time_width);

                // Pad source (left align, or all spaces if None)
                let source_str = if let Some(source) = &line.source {
                    format!("{:<width$}", source, width = source_width)
                } else {
                    " ".repeat(source_width)
                };

                spans.push(Span::styled(time_str, text_style));
                spans.push(Span::raw(" ")); // Space after time
                spans.push(Span::styled(source_str, text_style));
                spans.push(Span::raw(" ")); // Space after source
                spans.push(Span::styled(&line.content, text_style));

                Line::from(spans)
            })
            .collect();

        let text = Text::from(visible_lines);

        let paragraph = Paragraph::new(text.set_style(Style::default()))
            .block(Block::bordered().title(title).border_style(border_style));

        // Split area for content and scrollbar
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        // Update scrollbar state
        self.vertical_scroll_state = ScrollbarState::new(self.content.len())
            .position(self.scroll_offset + self.max_visible_height);

        frame.render_widget(paragraph, layout[0]);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().bg(Color::Cyan)),
            layout[1],
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
