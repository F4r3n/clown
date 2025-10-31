use crate::component::Draw;
use crate::irc_view::dimension_discuss::{NICKNAME_LENGTH, SEPARATOR_LENGTH, TIME_LENGTH};
use crate::{MessageEvent, irc_view::message_content::MessageContent};
use ahash::AHashMap;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

#[derive(Debug)]
pub struct ChannelMessages {
    messages: AHashMap<String, Vec<MessageContent>>,
}

impl ChannelMessages {
    pub fn new() -> Self {
        Self {
            messages: AHashMap::new(),
        }
    }

    pub fn add_message(&mut self, channel: &str, in_message: &MessageContent) {
        self.messages
            .entry(channel.to_string())
            .or_default()
            .push(in_message.clone());
    }

    pub fn get_number_messages(&self, channel: &str) -> Option<usize> {
        self.messages.get(channel).map(|v| v.len())
    }

    pub fn get_messages(&self, channel: &str) -> Option<&Vec<MessageContent>> {
        self.messages.get(channel)
    }

    pub fn get_url(&self, channel: &str, index: usize, character_pos: usize) -> Option<String> {
        self.messages
            .get(channel)
            .and_then(|messages| messages.get(index))
            .and_then(|message| message.get_url(character_pos))
            .map(|str| str.to_string())
    }
}

pub struct DiscussWidget {
    vertical_scroll_state: ScrollbarState,
    scroll_offset: usize,
    max_visible_height: usize,
    follow_last: bool,
    focus: bool,
    area: Rect,
    content_width: usize,
    messages: ChannelMessages,
    current_channel: String,
}

impl Draw for DiscussWidget {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
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
        self.content_width = content_width as usize;

        let mut counter: i64 = self.max_visible_height as i64;
        let mut visible_rows = vec![];
        if let Some(messages) = self.messages.get_messages(&self.current_channel) {
            for line in messages.iter().skip(scroll).take(self.max_visible_height) {
                let mut new_rows = line.create_rows(content_width, &focus_style);
                counter -= new_rows.len() as i64;
                if counter < 0 {
                    for i in 0..counter.abs() {
                        if let Some(row) = new_rows.get(i as usize) {
                            visible_rows.push(row.clone());
                        }
                    }
                    break;
                } else {
                    visible_rows.append(&mut new_rows);
                }
                if counter == 0 {
                    break;
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
                crossterm::event::MouseEventKind::Moved => {
                    let mouse_position =
                        ratatui::prelude::Position::new(mouse_event.column, mouse_event.row);

                    if self.area.contains(mouse_position) {
                        if let Some((index, character)) = self
                            .get_current_line_index_character(mouse_event.row, mouse_event.column)
                        {
                            self.messages
                                .get_url(&self.current_channel, index, character)
                                .map(MessageEvent::Hover)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

impl DiscussWidget {
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
            content_width: 0,
        }
    }

    pub fn set_current_channel(&mut self, channel: &str) {
        self.current_channel = channel.to_string();
        let max_scroll = self.get_max_scroll();
        self.scroll_offset = max_scroll;
        self.follow_last = true;
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

        let pos_x = mouse_pos_x.saturating_sub(text_start);

        if pos_x > self.content_width {
            return None;
        }

        if let Some(messages) = self.messages.get_messages(&self.current_channel) {
            let mut wrapped_line_index = 0;

            for (msg_index, line) in messages
                .iter()
                .skip(self.scroll_offset)
                .take(self.max_visible_height)
                .enumerate()
            {
                let msg_len = line.get_message_length();
                let wrapped_lines = msg_len.div_ceil(self.content_width);

                if index_y < wrapped_line_index + wrapped_lines {
                    let line_in_msg = index_y - wrapped_line_index;
                    let local_x = pos_x.min(self.content_width - 1);
                    let char_index = line_in_msg * self.content_width + local_x;
                    if char_index >= msg_len {
                        return None;
                    }
                    return Some((msg_index + self.scroll_offset, char_index));
                }

                wrapped_line_index += wrapped_lines;
            }
        }

        None
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

    #[test]
    fn test_find_index() {
        pub const TEXT_START: usize = TIME_LENGTH + NICKNAME_LENGTH + SEPARATOR_LENGTH;

        let mut discuss = DiscussWidget::new("test");
        discuss.content_width = 4;
        discuss.area.width = (TEXT_START + discuss.content_width) as u16;
        discuss.add_line("test", &MessageContent::new_message(None, "HELLO", "hey"));
        discuss.add_line("test", &MessageContent::new_message(None, "HELLO", "hey"));
        discuss.add_line("test", &MessageContent::new_message(None, "HELLO", "hey"));

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
            discuss.get_current_line_index_character(2, mouse_x),
            Some((2, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(1, mouse_x),
            Some((1, 4))
        );
        assert_eq!(
            discuss.get_current_line_index_character(0, mouse_x),
            Some((1, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(3, mouse_x),
            Some((2, 4))
        );
        assert_eq!(discuss.get_current_line_index_character(4, mouse_x), None);
        assert_eq!(
            discuss.get_current_line_index_character(0, (TEXT_START + 100) as u16),
            None
        );
    }
}
