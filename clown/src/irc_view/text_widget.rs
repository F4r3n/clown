use std::collections::HashMap;

use crate::component::Draw;
use crate::irc_view::dimension_discuss::{NICKNAME_LENGTH, SEPARATOR_LENGTH, TIME_LENGTH};
use crate::{MessageEvent, irc_view::message_content::MessageContent};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

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
        let content_width = layout[0]
            .width
            .saturating_sub(TIME_LENGTH as u16)
            .saturating_sub(NICKNAME_LENGTH as u16)
            .saturating_sub(SEPARATOR_LENGTH as u16)
            .saturating_sub(4 /*Length separator between content */ as u16);

        let mut visible_rows = vec![];
        if let Some(messages) = self.messages.get_messages(&self.current_channel) {
            for line in messages.iter().skip(scroll).take(self.max_visible_height) {
                visible_rows.append(&mut line.create_rows(content_width, &focus_style));
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
