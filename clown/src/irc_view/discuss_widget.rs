use std::time::Duration;

use super::color_user::nickname_color;
use super::dimension_discuss::{NICKNAME_LENGTH, SEPARATOR_LENGTH, TIME_LENGTH};
use super::message_content::WordPos;
use crate::component::Draw;
use crate::{irc_view::message_content::MessageContent, message_event::MessageEvent};
use ahash::AHashMap;
use crossterm::event::KeyCode;
use crossterm::event::MouseButton;
use ratatui::widgets::Row;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};
use tracing::info;
#[derive(PartialEq, Eq, Clone)]
struct Range {
    line: usize,
    word_pos: WordPos,
}

#[derive(Debug, Default)]
pub struct ChannelMessages {
    messages: AHashMap<String, Vec<MessageContent>>,
}

impl ChannelMessages {
    pub fn add_message(&mut self, channel: &str, in_message: MessageContent) {
        self.messages
            .entry(channel.to_string())
            .or_default()
            .push(in_message);
    }

    pub fn get_messages(&self, channel: &str) -> Option<&Vec<MessageContent>> {
        self.messages.get(channel)
    }

    pub fn update_messages_width(&mut self, channel: &str, width: u16) {
        if let Some(v) = self.messages.get_mut(channel) {
            for m in v.iter_mut() {
                m.compute_cached_line_count(width as usize);
            }
        }
    }

    fn get_url_from_range(&self, channel: &str, range: &Range) -> Option<String> {
        self.messages
            .get(channel)
            .and_then(|messages| messages.get(range.line))
            .and_then(|message| message.get_url_from_pos(&range.word_pos))
            .map(|str| str.to_string())
    }

    fn get_word_pos(&self, channel: &str, index: usize, character_pos: usize) -> Option<Range> {
        self.messages
            .get(channel)
            .and_then(|messages| messages.get(index))
            .and_then(|message| message.get_word_pos(character_pos))
            .map(|w| Range {
                line: index,
                word_pos: w,
            })
    }
}
struct Hovered {
    time: std::time::Instant,
    range: Range,
}

impl Hovered {
    pub fn new(range: Range) -> Self {
        Self {
            range,
            time: std::time::Instant::now(),
        }
    }
}

pub struct DiscussWidget {
    vertical_scroll_state: ScrollbarState,
    scroll_offset: usize,
    max_visible_height: usize,
    follow_last: bool,
    area: Rect,
    content_width: usize,
    messages: ChannelMessages,
    current_channel: String,

    last_hovered: Option<Hovered>,
    color_map: AHashMap<String, ratatui::style::Color>,
    redraw: bool,
}

impl DiscussWidget {
    pub fn new(current_channel: &str) -> Self {
        Self {
            current_channel: current_channel.to_lowercase(),
            messages: ChannelMessages::default(),
            scroll_offset: 0,
            max_visible_height: 10,
            follow_last: true,
            vertical_scroll_state: ScrollbarState::default(),
            area: Rect::default(),
            content_width: 0,
            last_hovered: None,
            color_map: AHashMap::new(),
            redraw: true,
        }
    }

    pub fn set_current_channel(&mut self, channel: &str) {
        self.color_map
            .entry(channel.to_string())
            .or_insert(nickname_color(channel));
        self.current_channel = channel.to_lowercase();
        let max_scroll = self.get_max_scroll();
        self.scroll_offset = max_scroll;
        self.follow_last = true;
    }

    fn get_range_from_mouse(&self, row: u16, col: u16) -> Option<Range> {
        let mouse_position = ratatui::prelude::Position::new(col, row);

        if self.area.contains(mouse_position) {
            if let Some((index, character)) = self.get_current_line_index_character(row, col) {
                self.messages
                    .get_word_pos(&self.current_channel, index, character)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_current_line_index_character(
        &self,
        mouse_pos_y: u16,
        mouse_pos_x: u16,
    ) -> Option<(usize, usize)> {
        let index_y = mouse_pos_y.saturating_sub(self.area.y) as usize;
        let mouse_pos_x = mouse_pos_x.saturating_sub(self.area.x) as usize;

        let text_start = self.area.width as usize - self.content_width;
        if mouse_pos_x < text_start {
            return None;
        }

        let pos_x = mouse_pos_x - text_start;
        if pos_x > self.content_width {
            return None;
        }

        let mut wrapped_rows_seen = 0;
        let mut visible_rows_total = 0;

        if let Some(messages) = self.messages.get_messages(&self.current_channel) {
            for (line_index, line) in messages.iter().enumerate() {
                let total_rows = line.get_cached_line_count(self.content_width).unwrap_or(1);

                if wrapped_rows_seen + total_rows <= self.scroll_offset {
                    wrapped_rows_seen += total_rows;
                    continue;
                }

                let mut rows = line.get_wrapped_line(self.content_width);
                let mut char_skipped: usize = 0;
                if self.scroll_offset > wrapped_rows_seen {
                    let skip = self.scroll_offset - wrapped_rows_seen;
                    if let Some(rows) = rows.get(..skip) {
                        char_skipped = rows.iter().map(|v| v.chars().count()).sum();
                    }
                    rows = rows.into_iter().skip(skip).collect();
                }

                if index_y < visible_rows_total + rows.len() {
                    let pointed_row = (index_y - visible_rows_total).min(rows.len() - 1);
                    //it will be an approximation, because wrapping can remove spaces,
                    //  but sometimes does not remove characters
                    let char_position: usize = rows
                        .get(..pointed_row)
                        .map(|slice| slice.iter().map(|v| v.chars().count()).sum())
                        .unwrap_or(0);
                    if char_skipped + char_position + pos_x > line.get_message_width() {
                        return None;
                    }
                    return Some((line_index, char_skipped + char_position + pos_x));
                }

                visible_rows_total += rows.len();
                wrapped_rows_seen += total_rows;
            }
        }

        None
    }

    fn collect_visible_rows<'a>(&'a mut self) -> Vec<Row<'a>> {
        let mut visible_rows = Vec::new();
        if self.content_width == 0 {
            return visible_rows;
        }
        let mut wrapped_rows_seen = 0; // counts all rows, even skipped
        let mut visible_rows_total = 0; // counts only rendered rows

        if let Some(messages) = self.messages.get_messages(&self.current_channel) {
            for line in messages {
                //Approximation of the start, same as total_lines
                let total_rows = line.get_cached_line_count(self.content_width).unwrap_or(1);
                // Skip rows above scroll
                if wrapped_rows_seen + total_rows <= self.scroll_offset {
                    wrapped_rows_seen += total_rows;
                    continue;
                }

                // Create all wrapped rows for this message
                let mut rows = line
                    .create_rows(
                        self.content_width as u16,
                        line.get_source().and_then(|s| self.color_map.get(s)),
                    )
                    .collect::<Vec<Row<'a>>>();
                let total_rows = rows.len();
                // Skip inside this message if scroll_offset lands inside it
                if self.scroll_offset > wrapped_rows_seen {
                    let skip = self.scroll_offset - wrapped_rows_seen;
                    rows = rows.into_iter().skip(skip).collect();
                }

                // Truncate if screen full
                let remaining = self.max_visible_height - visible_rows_total;
                if rows.len() > remaining {
                    rows.truncate(remaining);
                }

                visible_rows_total += rows.len();
                wrapped_rows_seen += total_rows;
                visible_rows.extend(rows);

                if visible_rows_total >= self.max_visible_height {
                    break;
                }
            }
        }

        visible_rows
    }

    fn get_total_lines(&self) -> usize {
        if self.content_width == 0 {
            return 0;
        }

        self.messages
            .get_messages(&self.current_channel)
            .map(|msgs| {
                msgs.iter()
                    .map(|m| m.get_cached_line_count(self.content_width).unwrap_or(1))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn add_line(&mut self, channel: &str, in_message: MessageContent) {
        if let Some(source) = in_message.get_source() {
            self.color_map
                .entry(source.to_string())
                .or_insert(nickname_color(source));
        }
        let channel = channel.to_lowercase();
        let mut message = in_message;
        message.compute_cached_line_count(self.content_width);
        //tracing::debug!("Message {:?}", &message);

        self.messages.add_message(&channel, message);

        if self.follow_last && channel.eq(&self.current_channel) {
            // Show last lines that fit the view
            self.scroll_offset = self.get_max_scroll();
        }
        self.redraw = true;
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.follow_last = false;
        self.redraw = true;
    }

    fn get_max_scroll(&self) -> usize {
        self.get_total_lines()
            .saturating_sub(self.max_visible_height)
    }

    fn scroll_boundary(&mut self) {
        let max_scroll = self.get_max_scroll();
        self.scroll_offset = self.scroll_offset.min(max_scroll);
        if self.follow_last {
            self.scroll_offset = max_scroll;
        }
    }

    fn scroll_down(&mut self) {
        let max_scroll = self.get_max_scroll();
        tracing::debug!("total lines {}", self.get_total_lines());

        self.scroll_offset = self.scroll_offset.saturating_add(1).min(max_scroll);

        self.follow_last = max_scroll.eq(&self.scroll_offset);
        tracing::debug!("Scroll offset {}", self.scroll_offset);

        self.redraw = true;
    }
}

impl Draw for DiscussWidget {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if self.redraw {
            self.redraw = false;
        }

        self.area = area;

        let text_style = Style::default().fg(Color::White);

        // Set how many lines can be shown
        self.max_visible_height = area.height as usize;

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(30),   // Meta (time + source)
                Constraint::Length(1), // Scrollbar
            ])
            .split(area);

        let content_width = layout
            .first()
            .map(|rect| {
                rect.width
                    .saturating_sub(TIME_LENGTH as u16)
                    .saturating_sub(NICKNAME_LENGTH as u16)
                    .saturating_sub(SEPARATOR_LENGTH as u16)
                    .saturating_sub(4_u16)
            })
            .unwrap_or(0);
        if (content_width as usize) != self.content_width {
            self.messages
                .update_messages_width(&self.current_channel, content_width);
        }
        self.content_width = content_width as usize;
        self.scroll_boundary();
        self.vertical_scroll_state = ScrollbarState::new(self.get_total_lines())
            .position(self.scroll_offset + self.max_visible_height);
        let visible_rows = self.collect_visible_rows();
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

        if let Some(layout) = layout.first() {
            frame.render_widget(table, *layout)
        }
        if let Some(layout_1) = layout.get(1)
            && layout_1.width > 0
        {
            frame.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::default().bg(Color::Cyan)),
                *layout_1,
                &mut self.vertical_scroll_state,
            );
        }
    }
}

impl crate::component::EventHandler for DiscussWidget {
    fn get_area(&self) -> Rect {
        self.area
    }
    fn need_redraw(&self) -> bool {
        self.redraw
    }

    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::AddMessageView(channel, in_message) => {
                if let Some(channel) = channel {
                    self.add_line(channel, in_message.clone());
                } else {
                    self.add_line(&self.current_channel.clone(), in_message.clone());
                }
                None
            }
            MessageEvent::SelectChannel(channel) => {
                self.set_current_channel(channel);
                None
            }
            _ => None,
        }
    }

    fn handle_events(&mut self, event: &crate::event_handler::Event) -> Option<MessageEvent> {
        if let crate::event_handler::Event::Crossterm(cross) = &event {
            match cross {
                crossterm::event::Event::Resize(_x, _y) => {
                    self.redraw = true;
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
                    crossterm::event::MouseEventKind::Down(button) => {
                        if button == MouseButton::Left {
                            if let Some(range) =
                                self.get_range_from_mouse(mouse_event.row, mouse_event.column)
                            {
                                info!(
                                    "Try open url {:?}",
                                    self.messages
                                        .get_url_from_range(&self.current_channel, &range)
                                );

                                self.messages
                                    .get_url_from_range(&self.current_channel, &range)
                                    .map(MessageEvent::OpenWeb)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    #[cfg(feature = "website-preview")]
                    crossterm::event::MouseEventKind::Moved => {
                        if let Some(range) =
                            self.get_range_from_mouse(mouse_event.row, mouse_event.column)
                        {
                            if let Some(last_hovered) = &self.last_hovered
                                && range.eq(&last_hovered.range)
                            {
                                if std::time::Instant::now().duration_since(last_hovered.time)
                                    > Duration::from_secs(2)
                                {
                                    info!(
                                        "Hovering url {:?}",
                                        self.messages
                                            .get_url_from_range(&self.current_channel, &range)
                                    );
                                    self.messages
                                        .get_url_from_range(
                                            &self.current_channel,
                                            &last_hovered.range,
                                        )
                                        .map(MessageEvent::HoverURL)
                                } else {
                                    None
                                }
                            } else {
                                self.last_hovered = Some(Hovered::new(range));
                                None
                            }
                        } else {
                            self.last_hovered = None;
                            None
                        }
                    }
                    _ => None,
                },
                crossterm::event::Event::Key(key_event) => match key_event.code {
                    KeyCode::PageUp => {
                        for _ in 0..5 {
                            self.scroll_up();
                        }
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
                        self.scroll_offset = self.get_total_lines();
                        None
                    }
                    _ => None,
                },
                _ => None,
            }
        } else {
            match event {
                crate::event_handler::Event::Tick => {
                    if let Some(last_hovered) = &self.last_hovered {
                        if std::time::Instant::now().duration_since(last_hovered.time)
                            > Duration::from_secs(2)
                        {
                            self.messages
                                .get_url_from_range(&self.current_channel, &last_hovered.range)
                                .map(MessageEvent::HoverURL)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_index() {
        pub const TEXT_START: usize = TIME_LENGTH + NICKNAME_LENGTH + SEPARATOR_LENGTH;

        let mut discuss = DiscussWidget::new("test");
        discuss.content_width = 4;
        discuss.area.width = (TEXT_START + discuss.content_width) as u16;
        discuss.add_line(
            "test",
            MessageContent::new_message(None, "HELLO".to_string(), "hey".to_string()),
        );
        discuss.add_line(
            "test",
            MessageContent::new_message(None, "HELLO".to_string(), "hey".to_string()),
        );
        discuss.add_line(
            "test",
            MessageContent::new_message(None, "HELLO".to_string(), "hey".to_string()),
        );

        assert_eq!(discuss.scroll_offset, 0);
        let mouse_x = TEXT_START as u16;
        assert_eq!(
            discuss.get_current_line_index_character(0, mouse_x),
            Some((0, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(2, mouse_x),
            Some((1, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(1, mouse_x),
            Some((0, 4))
        );

        assert_eq!(
            discuss.get_current_line_index_character(3, mouse_x),
            Some((1, 4))
        );
        assert_eq!(
            discuss.get_current_line_index_character(4, mouse_x),
            Some((2, 0))
        );

        discuss.scroll_offset = 1;
        discuss.area.width = (TEXT_START + discuss.content_width) as u16;
        assert_eq!(
            discuss.get_current_line_index_character(0, mouse_x),
            Some((0, 4))
        );
        assert_eq!(
            discuss.get_current_line_index_character(1, mouse_x),
            Some((1, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(2, mouse_x),
            Some((1, 4))
        );

        assert_eq!(
            discuss.get_current_line_index_character(3, mouse_x),
            Some((2, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(4, mouse_x),
            Some((2, 4))
        );
        assert_eq!(
            discuss.get_current_line_index_character(0, (TEXT_START + 100) as u16),
            None
        );
    }

    #[test]
    fn test_total_lines() {
        let mut discuss = DiscussWidget::new("");
        discuss.content_width = 10;

        discuss.add_line(
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.add_line(
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.add_line(
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.scroll_offset = 0;
        assert_eq!(discuss.get_total_lines(), 9);
    }

    #[test]
    fn test_render_rows() {
        let mut discuss = DiscussWidget::new("test");

        discuss.add_line(
            "test",
            MessageContent::new_message(None, "HELLO".to_string(), "hey".to_string()),
        );
        discuss.add_line(
            "test",
            MessageContent::new_message(None, "HELLO".to_string(), "hey".to_string()),
        );
        discuss.add_line(
            "test",
            MessageContent::new_message(None, "HELLO".to_string(), "hey".to_string()),
        );

        discuss.content_width = 10;
        discuss.messages.update_messages_width("test", 10);
        assert_eq!(discuss.collect_visible_rows().len(), 3);

        discuss.content_width = 4;
        discuss.scroll_offset = 0;
        discuss
            .messages
            .update_messages_width("test", discuss.content_width as u16);
        assert_eq!(discuss.collect_visible_rows().len(), 6);

        discuss.content_width = 4;
        discuss.scroll_offset = 0;
        discuss.max_visible_height = 2;
        discuss
            .messages
            .update_messages_width("test", discuss.content_width as u16);
        let rows = discuss.collect_visible_rows();
        assert_eq!(rows.len(), 2);

        discuss.content_width = 4;
        discuss.scroll_offset = 1;
        discuss.max_visible_height = 2;
        discuss
            .messages
            .update_messages_width("test", discuss.content_width as u16);
        let rows = discuss.collect_visible_rows();
        assert_eq!(rows.len(), 2);
    }
}
