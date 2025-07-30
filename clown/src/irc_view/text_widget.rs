use std::{borrow::Cow, collections::HashMap};

use crate::MessageEvent;
use crate::component::Draw;
use crate::irc_view::color_user::nickname_color;
use chrono::{DateTime, Local, Timelike};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};
use textwrap::wrap;

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

#[derive(Debug)]
pub struct ChannelMessages {
    messages: HashMap<String, Vec<MessageContent>>,
}

impl ChannelMessages {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
        }
    }

    pub fn add_message(&mut self, channel: &str, in_message: &MessageContent) {
        self.messages
            .entry(channel.to_string())
            .or_insert_with(Vec::new)
            .push(in_message.clone());
    }

    pub fn get_number_messages(&self, channel: &str) -> Option<usize> {
        self.messages.get(channel).map(|v| v.len())
    }

    pub fn get_messages(&self, channel: &str) -> Option<&Vec<MessageContent>> {
        self.messages.get(channel)
    }
}

pub struct TextWidget {
    vertical_scroll_state: ScrollbarState,
    scroll_offset: usize,
    max_visible_height: usize,
    follow_last: bool,
    focus: bool,
    area: Rect,
    messages: ChannelMessages,
    current_channel: String,
}

const TIME_LENGTH: usize = 8;
const NICKNAME_LENGTH: usize = 10;
const SEPARATOR_LENGTH: usize = 2;

fn irc_to_color(code: &str) -> Color {
    match code {
        "00" | "0" => Color::White,
        "01" | "1" => Color::Black,
        "02" | "2" => Color::Blue,
        "03" | "3 " => Color::Green,
        "04" | "4" => Color::Red,
        "05" | "5" => Color::Rgb(127, 63, 0), // Brown (Maroon)
        "06" | "6" => Color::Magenta,
        "07" | "7" => Color::Rgb(252, 127, 0), // Orange
        "08" | "8" => Color::Yellow,
        "09" | "9" => Color::LightGreen,
        "10" => Color::Cyan,
        "11" => Color::LightCyan,
        "12" => Color::LightBlue,
        "13" => Color::Rgb(255, 0, 255), // Pink (Magenta)
        "14" => Color::Gray,
        "15" => Color::Rgb(210, 210, 210), // Light Grey
        _ => Color::default(),
    }
}

fn to_spans<'a>(content: &str, start_style: Option<Style>) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut setting_style = false;
    let mut style_buffer = String::new();
    let mut colors = vec![
        start_style.unwrap_or_default().fg.unwrap_or_default(),
        start_style.unwrap_or_default().bg.unwrap_or_default(),
    ];
    let mut index_color = 0;

    for c in content.chars() {
        if c == '\x03' {
            if !buffer.is_empty() {
                spans.push(
                    Span::from(buffer.clone()).style(Style::default().fg(colors[0]).bg(colors[1])),
                );
                buffer.clear();
            }
            setting_style = true;
            style_buffer.clear();
            index_color = 0;
        } else if setting_style && style_buffer.len() < 2 && c >= '0' && c <= '9' {
            style_buffer.push(c);
        } else if setting_style && c == ',' && index_color == 0 {
            colors[index_color] = irc_to_color(&style_buffer);
            index_color += 1;
            style_buffer.clear();
        } else {
            if setting_style {
                setting_style = false;
                colors[index_color] = irc_to_color(&style_buffer);
            }
            buffer.push(c);
        }
    }
    if !buffer.is_empty() {
        spans.push(Span::from(buffer.clone()).style(Style::default().fg(colors[0]).bg(colors[1])));
    }
    spans
}

impl Draw for TextWidget {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        let focused = self.focus;
        let focus_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let text_style = Style::default().fg(Color::White);

        // Set how many lines can be shown
        self.max_visible_height = area.height as usize;
        let max_scroll = self
            .get_number_messages()
            .saturating_sub(self.max_visible_height);
        let scroll = self.scroll_offset.min(max_scroll);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(30),   // Meta (time + source)
                Constraint::Length(1), // Scrollbar
            ])
            .split(area);
        let mut visible_rows = vec![];
        if let Some(messages) = self.messages.get_messages(&self.current_channel) {
            for line in messages.iter().skip(scroll).take(self.max_visible_height) {
                let content = line.content.clone();
                let content_width = layout[0]
                    .width
                    .saturating_sub(TIME_LENGTH as u16)
                    .saturating_sub(NICKNAME_LENGTH as u16)
                    .saturating_sub(SEPARATOR_LENGTH as u16)
                    .saturating_sub(4 /*Length separator between content */ as u16);

                let time_str = format!("{:>width$}", line.time_format(), width = TIME_LENGTH);
                let nickname_style = if let Some(source) = &line.source {
                    nickname_color(&source)
                } else {
                    Color::default()
                };
                let source_str = format!(
                    "{:<width$}",
                    line.source.as_ref().unwrap_or(&"".to_string()),
                    width = NICKNAME_LENGTH
                );
                let wrapped = wrap(&content, content_width as usize);
                let first_part = wrapped
                    .first()
                    .map(|v| to_spans(v, None))
                    .unwrap_or_default();
                let style_first_part = first_part.last().map(|v| v.style);

                visible_rows.push(Row::new(vec![
                    Cell::from(time_str),
                    Cell::from(source_str).style(nickname_style),
                    Cell::from("┃ ").style(focus_style),
                    Cell::from(Line::from(first_part)),
                ]));
                for w in wrapped.iter().skip(1) {
                    visible_rows.push(Row::new(vec![
                        Cell::from(format!("{:<width$}", " ", width = TIME_LENGTH)),
                        Cell::from(format!("{:<width$}", " ", width = NICKNAME_LENGTH))
                            .style(nickname_style),
                        Cell::from("┃ ").style(focus_style),
                        Cell::from(Line::from(to_spans(w, style_first_part))),
                    ]));
                }
            }
        }

        let table = Table::new(
            visible_rows,
            [
                Constraint::Length(TIME_LENGTH.saturating_add(1) as u16), // time
                Constraint::Length(NICKNAME_LENGTH.saturating_add(1) as u16), // nickname
                Constraint::Length(1),                                    // separator
                Constraint::Min(10),                                      // Content
            ],
        )
        .column_spacing(1)
        .style(text_style);

        self.vertical_scroll_state = ScrollbarState::new(self.get_number_messages())
            .position(self.scroll_offset + self.max_visible_height);

        frame.render_widget(table, layout[0]);
        if layout[1].width > 0 {
            frame.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::default().bg(Color::Cyan)),
                layout[1],
                &mut self.vertical_scroll_state,
            );
        }
    }
}

impl crate::component::EventHandler for TextWidget {
    fn has_focus(&self) -> bool {
        self.focus
    }

    fn get_area(&self) -> Rect {
        self.area
    }

    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::AddMessageView(channel, in_message) => {
                self.add_line(channel, in_message);
                None
            }
            MessageEvent::SelectChannel(channel) => {
                self.set_current_channel(channel);
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
                    self.scroll_offset = self.get_number_messages();
                    None
                }
                _ => None,
            }
        } else if let Some(mouse_event) = event.get_mouse() {
            match mouse_event.kind {
                crossterm::event::MouseEventKind::ScrollDown => {
                    self.scroll_down();
                    None
                }
                crossterm::event::MouseEventKind::ScrollUp => {
                    self.scroll_up();
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
    pub fn new(current_channel: &str) -> Self {
        Self {
            current_channel: current_channel.to_string(),
            messages: ChannelMessages::new(),
            focus: false,
            scroll_offset: 0,
            max_visible_height: 10,
            follow_last: true,
            vertical_scroll_state: ScrollbarState::default(),
            area: Rect::default(),
        }
    }

    pub fn set_current_channel(&mut self, channel: &str) {
        self.current_channel = channel.to_string();
        let max_scroll = self.get_max_scroll();
        self.scroll_offset = max_scroll;
        self.follow_last = true;
    }

    fn get_number_messages(&self) -> usize {
        self.messages
            .get_number_messages(&self.current_channel)
            .unwrap_or_default()
    }

    pub fn add_line(&mut self, channel: &str, in_message: &MessageContent) {
        self.messages.add_message(channel, in_message);

        if self.follow_last && channel.eq(&self.current_channel) {
            // Show last lines that fit the view
            self.scroll_offset = self
                .get_number_messages()
                .saturating_sub(self.max_visible_height);
        }
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.follow_last = false;
    }

    fn get_max_scroll(&self) -> usize {
        self.messages
            .get_number_messages(&self.current_channel)
            .unwrap_or_default()
            .saturating_sub(self.max_visible_height)
    }

    fn scroll_down(&mut self) {
        let max_scroll = self.get_max_scroll();
        self.scroll_offset = self.scroll_offset.saturating_add(1).min(max_scroll);
        self.follow_last = max_scroll.eq(&self.scroll_offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to extract text and colors from spans for assertion
    fn span_data<'a>(span: &'a Span) -> (&'a str, Color, Color) {
        // Assuming you have methods or public fields to get these:
        (
            &span.content,
            span.style.fg.unwrap_or_default(),
            span.style.bg.unwrap_or_default(),
        )
    }

    #[test]
    fn test_plain_text() {
        let input = "Hello world";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hello world");
        assert_eq!(fg, Color::default());
        assert_eq!(bg, Color::default());
    }

    #[test]
    fn test_single_color() {
        let input = "\x034Hello";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hello");
        assert_eq!(fg, Color::Red);
        assert_eq!(bg, Color::default());
    }

    #[test]
    fn test_fg_and_bg_color() {
        let input = "\x038,4Hi!";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hi!");
        assert_eq!(fg, Color::Yellow);
        assert_eq!(bg, Color::Red);
    }

    #[test]
    fn test_multispan_multicolor() {
        let input = "A\x034B\x037C";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 3);

        let (a, fg_a, _) = span_data(&spans[0]);
        let (b, fg_b, _) = span_data(&spans[1]);
        let (c, fg_c, _) = span_data(&spans[2]);
        assert_eq!(a, "A");
        assert_eq!(fg_a, Color::default());
        assert_eq!(b, "B");
        assert_eq!(fg_b, Color::Red);
        assert_eq!(c, "C");
        assert_eq!(fg_c, Color::Rgb(252, 127, 0)); // Orange
    }

    #[test]
    fn test_trailing_reset() {
        let input = "\x034Red\x03Normal";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 2);

        let (red, fg_red, _) = span_data(&spans[0]);
        let (normal, fg_normal, _) = span_data(&spans[1]);
        assert_eq!(red, "Red");
        assert_eq!(fg_red, Color::Red);
        assert_eq!(normal, "Normal");
        assert_eq!(fg_normal, Color::default());
    }

    // Add more tests as needed for edge cases, like incomplete codes, empty input, etc.
}
