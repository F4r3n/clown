use std::time::Duration;

use super::dimension_discuss::{NICKNAME_LENGTH, SEPARATOR_LENGTH, TIME_LENGTH};
use crate::component::Draw;
use crate::message_irc::message_content::WordPos;
use crate::{message_event::MessageEvent, message_irc::message_content::MessageContent};
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

#[derive(PartialEq, Eq, Clone, Debug)]
struct Range {
    line: usize,
    word_pos: WordPos,
}

#[derive(Debug, Default)]
pub struct ServerMessages {
    messages: Vec<ChannelMessages>,
}

impl ServerMessages {
    pub fn new() -> Self {
        Self {
            messages: vec![ChannelMessages::default()],
        }
    }

    pub fn add_message(
        &mut self,
        server_id: Option<usize>,
        channel: &str,
        in_message: MessageContent,
    ) {
        if let Some(server_group) = self.add_server_group(server_id) {
            server_group.add_message(channel, in_message);
        }
    }

    fn server_id_position(server_id: Option<usize>) -> usize {
        server_id.map(|v| v.saturating_add(1)).unwrap_or(0)
    }

    fn add_server_group(&mut self, server_id: Option<usize>) -> Option<&mut ChannelMessages> {
        let new_length = Self::server_id_position(server_id).saturating_add(1);
        self.messages.resize_with(new_length, Default::default);

        self.get_server_group_mut(server_id)
    }

    fn get_server_group_mut(&mut self, server_id: Option<usize>) -> Option<&mut ChannelMessages> {
        self.messages.get_mut(Self::server_id_position(server_id))
    }

    fn get_server_group(&self, server_id: Option<usize>) -> Option<&ChannelMessages> {
        self.messages.get(Self::server_id_position(server_id))
    }

    fn rename(&mut self, server_id: Option<usize>, old: &str, new: &str) {
        if let Some(server_group) = self.get_server_group_mut(server_id) {
            server_group.rename(old, new);
        }
    }

    pub fn has_messages(&self, server_id: Option<usize>, channel: &str) -> bool {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.has_messages(channel)
        } else {
            false
        }
    }

    pub fn get_messages(
        &self,
        server_id: Option<usize>,
        channel: &str,
    ) -> Option<&Vec<MessageContent>> {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.get_messages(channel)
        } else {
            None
        }
    }

    fn get_url_from_range(
        &self,
        server_id: Option<usize>,
        channel: &str,
        range: &Range,
    ) -> Option<String> {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.get_url_from_range(channel, range)
        } else {
            None
        }
    }

    fn get_word_pos(
        &self,
        server_id: Option<usize>,
        channel: &str,
        index: usize,
        character_pos: usize,
    ) -> Option<Range> {
        if let Some(server_group) = self.get_server_group(server_id) {
            server_group.get_word_pos(channel, index, character_pos)
        } else {
            None
        }
    }
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

    fn rename(&mut self, old: &str, new: &str) {
        if let Some(messages) = self.messages.remove(old) {
            self.messages.insert(new.to_string(), messages);
        }
    }

    pub fn has_messages(&self, channel: &str) -> bool {
        self.messages.get(channel).is_some_and(|c| !c.is_empty())
    }

    pub fn get_messages(&self, channel: &str) -> Option<&Vec<MessageContent>> {
        self.messages.get(channel)
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct DiscussWidget {
    vertical_scroll_state: ScrollbarState,
    scroll_offset: usize,
    max_visible_height: usize,
    follow_last: bool,
    area: Rect,
    content_width: usize,
    messages: ServerMessages,
    current_channel: String,
    current_server_id: Option<usize>,

    last_hovered: Option<Hovered>,
    redraw: bool,
}

impl DiscussWidget {
    pub fn new() -> Self {
        Self {
            current_channel: String::new(),
            current_server_id: None,
            messages: ServerMessages::new(),
            scroll_offset: 0,
            max_visible_height: 10,
            follow_last: true,
            vertical_scroll_state: ScrollbarState::default(),
            area: Rect::default(),
            content_width: 0,
            last_hovered: None,
            redraw: true,
        }
    }

    pub fn set_current_channel(&mut self, server_id: Option<usize>, channel: &str) {
        self.current_channel = channel.to_lowercase();
        self.current_server_id = server_id;
        self.scroll_offset = 0;
        self.follow_last = true;
    }

    fn get_range_from_mouse(&self, row: u16, col: u16) -> Option<Range> {
        let mouse_position = ratatui::prelude::Position::new(col, row);

        if self.area.contains(mouse_position) {
            if let Some((index, character)) = self.get_current_line_index_character(row, col) {
                self.messages.get_word_pos(
                    self.current_server_id,
                    &self.current_channel,
                    index,
                    character,
                )
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

        let mut visible_rows_total = 0;

        if let Some(messages) = self
            .messages
            .get_messages(self.current_server_id, &self.current_channel)
        {
            let message_count = messages.len();

            let target_visual_top = self.scroll_offset.saturating_add(self.max_visible_height);

            let mut start_message_index = 0;
            let mut rows_from_bottom = 0;
            let mut rows_to_skip_in_message = 0;

            for (i, line) in messages.iter().rev().enumerate() {
                let total_rows = line.wrapped_line_count(self.content_width);

                if rows_from_bottom + total_rows >= target_visual_top {
                    start_message_index = message_count.saturating_sub(1).saturating_sub(i);

                    rows_to_skip_in_message =
                        (rows_from_bottom + total_rows).saturating_sub(target_visual_top);
                    break;
                }

                rows_from_bottom += total_rows;
            }

            if let Some(slice) = messages.get(start_message_index..) {
                for (line_index, line) in slice.iter().enumerate() {
                    let total_line_rows = line.wrapped_line_count(self.content_width);

                    // Calculate available rows after skipping the top part (if any)
                    let rows_available = total_line_rows.saturating_sub(rows_to_skip_in_message);

                    // Cap strictly to the remaining height
                    let rows_remaining = self.max_visible_height.saturating_sub(visible_rows_total);
                    let rows_to_take = rows_remaining.min(rows_available);

                    if rows_to_take == 0 && rows_remaining == 0 {
                        break;
                    }

                    let mut rows = line.get_wrapped_line(self.content_width);
                    let mut char_skipped: usize = 0;
                    if let Some(rows) = rows.get(..rows_to_skip_in_message) {
                        char_skipped = rows.iter().map(|v| v.chars().count()).sum();
                    }
                    rows = rows.into_iter().skip(rows_to_skip_in_message).collect();
                    let rows_len = rows.len();

                    if index_y < visible_rows_total + rows_len {
                        let pointed_row = (index_y - visible_rows_total).min(rows_len - 1);
                        //it will be an approximation, because wrapping can remove spaces,
                        //  but sometimes does not remove characters
                        let char_position: usize = rows
                            .get(..pointed_row)
                            .map(|slice| slice.iter().map(|v| v.chars().count()).sum())
                            .unwrap_or(0);
                        if char_skipped + char_position + pos_x > line.get_message_width() {
                            return None;
                        }
                        return Some((
                            line_index + start_message_index,
                            char_skipped + char_position + pos_x,
                        ));
                    }
                    rows_to_skip_in_message = 0; // Reset for subsequent messages

                    visible_rows_total += rows_len;
                }
            }
        }

        None
    }

    fn collect_visible_rows<'a>(&'a mut self, model: Option<&crate::model::Model>) -> Vec<Row<'a>> {
        let mut visible_rows = Vec::new();
        if self.content_width == 0 {
            return visible_rows;
        }

        if let Some(messages) = self
            .messages
            .get_messages(self.current_server_id, &self.current_channel)
        {
            let message_count = messages.len();

            let target_visual_top = self.scroll_offset.saturating_add(self.max_visible_height);

            let mut start_message_index = 0;
            let mut rows_from_bottom = 0;
            let mut rows_to_skip_in_message = 0;

            for (i, line) in messages.iter().rev().enumerate() {
                let total_rows = line.wrapped_line_count(self.content_width);

                if rows_from_bottom + total_rows >= target_visual_top {
                    start_message_index = message_count.saturating_sub(1).saturating_sub(i);

                    rows_to_skip_in_message =
                        (rows_from_bottom + total_rows).saturating_sub(target_visual_top);
                    break;
                }

                rows_from_bottom += total_rows;
            }

            let mut visible_rows_total = 0;

            if let Some(slice) = messages.get(start_message_index..) {
                for line in slice {
                    let total_line_rows = line.wrapped_line_count(self.content_width);

                    // Calculate available rows after skipping the top part (if any)
                    let rows_available = total_line_rows.saturating_sub(rows_to_skip_in_message);

                    // Cap strictly to the remaining height
                    let rows_remaining = self.max_visible_height.saturating_sub(visible_rows_total);
                    let rows_to_take = rows_remaining.min(rows_available);

                    if rows_to_take == 0 && rows_remaining == 0 {
                        break;
                    }
                    let color = line
                        .get_source()
                        .and_then(|s| model.map(|v| v.get_color(s)));
                    let rows = line
                        .create_rows(
                            self.content_width as u16,
                            color,
                            TIME_LENGTH,
                            NICKNAME_LENGTH,
                        )
                        .skip(rows_to_skip_in_message)
                        .take(rows_to_take);

                    visible_rows_total += rows_to_take;
                    visible_rows.extend(rows);

                    rows_to_skip_in_message = 0; // Reset for subsequent messages

                    if visible_rows_total >= self.max_visible_height {
                        break;
                    }
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
            .get_messages(self.current_server_id, &self.current_channel)
            .map(|msgs| {
                msgs.iter()
                    .map(|m| m.wrapped_line_count(self.content_width))
                    .sum()
            })
            .unwrap_or(0)
    }

    fn get_fake_total_lines(&self) -> usize {
        if self.content_width == 0 {
            return 0;
        }

        self.messages
            .get_messages(self.current_server_id, &self.current_channel)
            .map(|msgs| {
                msgs.iter()
                    .map(|m| m.get_message_width().div_ceil(self.content_width))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn has_message(&self, server_id: Option<usize>, channel: &str) -> bool {
        let channel = channel.to_lowercase();
        self.messages.has_messages(server_id, &channel)
    }

    pub fn add_line(
        &mut self,
        server_id: Option<usize>,
        channel: &str,
        in_message: MessageContent,
    ) {
        let channel = channel.to_lowercase();

        self.messages.add_message(server_id, &channel, in_message);
        if channel.eq_ignore_ascii_case(&self.current_channel) {
            if self.follow_last {
                self.scroll_offset = 0;
            } else {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
        }

        self.redraw = true;
    }

    fn possible_scroll_up(&mut self, offset: usize) -> usize {
        if let Some(messages) = self
            .messages
            .get_messages(self.current_server_id, &self.current_channel)
        {
            let mut total_rows = 0;
            for line in messages.iter().rev() {
                total_rows += line.wrapped_line_count(self.content_width);
                if total_rows
                    >= self
                        .scroll_offset
                        .saturating_add(offset)
                        .saturating_add(self.max_visible_height)
                {
                    return offset;
                }
            }
            total_rows.saturating_sub(self.scroll_offset.saturating_add(self.max_visible_height))
        } else {
            0
        }
    }

    fn scroll_up(&mut self) {
        if self.possible_scroll_up(1) > 0 {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.follow_last = false;
            self.redraw = true;
        }
    }

    fn scroll_page_up(&mut self) {
        let offset = self.possible_scroll_up(self.max_visible_height);
        if offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_add(offset);
            self.follow_last = false;
            self.redraw = true;
        }
    }

    fn scroll_page_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(self.max_visible_height);
        self.follow_last = self.scroll_offset == 0;
        self.redraw = true;
    }

    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);

        self.follow_last = self.scroll_offset == 0;

        self.redraw = true;
    }

    fn should_highlight(&self, current_nickname: &str, in_content: &str) -> bool {
        let nick = current_nickname;

        if let Some(start_byte) = in_content.find(nick) {
            let end_byte = start_byte + nick.len();

            // char before
            let prev_char = in_content[..start_byte].chars().next_back();

            // char after
            let next_char = in_content[end_byte..].chars().next();

            let is_boundary = |c: char| c.is_ascii_punctuation() || c.is_ascii_whitespace();

            let start_ok = prev_char.is_none() || prev_char.is_some_and(is_boundary);
            let end_ok = next_char.is_none() || next_char.is_some_and(is_boundary);

            start_ok && end_ok
        } else {
            false
        }
    }
}

impl Draw for DiscussWidget {
    fn render(
        &mut self,
        model: &crate::model::Model,
        _irc_model: Option<&crate::irc_view::irc_model::IrcModel>,
        frame: &mut Frame<'_>,
        area: Rect,
    ) {
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

        self.content_width = content_width as usize;
        self.vertical_scroll_state = ScrollbarState::new(self.get_fake_total_lines()).position(
            self.get_fake_total_lines()
                .saturating_sub(self.scroll_offset),
        );
        let visible_rows = self.collect_visible_rows(Some(model));
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

    fn handle_actions(
        &mut self,
        irc_model: Option<&crate::irc_view::irc_model::IrcModel>,
        event: &MessageEvent,
    ) -> Option<MessageEvent> {
        match event {
            MessageEvent::AddMessageView(server_id, channel, in_message) => {
                if let Some(channel) = channel {
                    self.add_line(
                        server_id.or(self.current_server_id),
                        channel,
                        in_message.clone(),
                    );
                } else {
                    self.add_line(
                        server_id.or(self.current_server_id),
                        &self.current_channel.clone(),
                        in_message.clone(),
                    );
                }
                None
            }
            MessageEvent::SetTopic(server_id, source, channel, content) => {
                if let Some(source) = source {
                    let data = format!(
                        "{} has changed topic for {} to \"{}\"",
                        source, channel, content
                    );
                    self.add_line(Some(*server_id), channel, MessageContent::new_info(data));
                }

                None
            }
            MessageEvent::Quit(server_id, user, reason) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    for channel in irc_server.get_all_joined_channel(user) {
                        self.add_line(
                            Some(*server_id),
                            channel,
                            MessageContent::new_info(
                                reason
                                    .as_ref()
                                    .map(|v| format!("{} has quit: {}", user, v))
                                    .unwrap_or_else(|| format!("{} has quit", user)),
                            ),
                        );
                    }

                    if self.has_message(Some(*server_id), user) {
                        self.add_line(
                            Some(*server_id),
                            user,
                            MessageContent::new_info(
                                reason
                                    .as_ref()
                                    .map(|v| format!("{} has quit: {}", user, v))
                                    .unwrap_or_else(|| format!("{} has quit", user)),
                            ),
                        );
                    }
                }

                None
            }

            MessageEvent::Part(server_id, channel, user) => {
                self.add_line(
                    Some(*server_id),
                    channel,
                    MessageContent::new_info(format!("{} has quit", user)),
                );
                if self.has_message(Some(*server_id), user) {
                    self.add_line(
                        Some(*server_id),
                        channel,
                        MessageContent::new_info(format!("{} has quit", user)),
                    );
                }

                None
            }
            MessageEvent::PrivMsg(server_id, source, target, content) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                    && let Some(current_channel) = irc_server.get_current_channel()
                {
                    let target = irc_server.get_target(source, target);

                    if source.eq_ignore_ascii_case(irc_server.get_current_nick())
                        && !target.eq_ignore_ascii_case(current_channel)
                    {
                        self.add_line(
                            Some(*server_id),
                            current_channel,
                            MessageContent::new_privmsg(target.to_string(), content.clone()),
                        );
                    }

                    let is_highlight =
                        self.should_highlight(irc_server.get_current_nick(), content);

                    self.add_line(
                        Some(*server_id),
                        target,
                        if is_highlight {
                            MessageContent::new_highlight(Some(source.clone()), content.clone())
                        } else {
                            MessageContent::new_message(Some(source.clone()), content.clone())
                        },
                    );
                    if is_highlight {
                        Some(MessageEvent::Bel)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            MessageEvent::ActionMsg(server_id, source, target, content) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    let target = irc_server.get_target(source, target);

                    self.add_line(
                        Some(*server_id),
                        target,
                        MessageContent::new_action(source.clone(), content.clone()),
                    );
                }

                None
            }
            MessageEvent::JoinServer(server_id, server) => {
                self.add_line(
                    Some(*server_id),
                    server,
                    MessageContent::new_info(format!("{} has joined", server)),
                );
                None
            }
            MessageEvent::Join(server_id, channel, source) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    let main = irc_server.is_main_user(source);
                    self.add_line(
                        Some(*server_id),
                        channel,
                        if main {
                            MessageContent::new_info(format!("You joined the channel {}", channel))
                        } else {
                            MessageContent::new_info(format!("{} has joined", source))
                        },
                    );
                }
                None
            }
            MessageEvent::ReplaceUser(server_id, old, new) => {
                if let Some(irc_model) = irc_model
                    && let Some(irc_server) = irc_model.get_server(*server_id)
                {
                    for channel in irc_server.get_all_joined_channel(old) {
                        self.add_line(
                            Some(*server_id),
                            channel,
                            MessageContent::new_info(format!(
                                "{} has changed their nickname to {}",
                                &old, &new
                            )),
                        );
                    }

                    self.messages.rename(Some(*server_id), old, new);
                    if self.current_channel.eq_ignore_ascii_case(old) {
                        self.current_channel = new.to_ascii_lowercase();
                    }
                }
                None
            }
            MessageEvent::SelectChannel(server_id, channel) => {
                self.set_current_channel(*server_id, channel);
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
                                self.messages
                                    .get_url_from_range(
                                        self.current_server_id,
                                        &self.current_channel,
                                        &range,
                                    )
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
                            let mut should_clear = false;
                            let result = if let Some(last_hovered) = &mut self.last_hovered
                                && range.eq(&last_hovered.range)
                            {
                                if std::time::Instant::now().duration_since(last_hovered.time)
                                    > Duration::from_secs(2)
                                {
                                    if let Some(url) = self.messages.get_url_from_range(
                                        self.current_server_id,
                                        &self.current_channel,
                                        &last_hovered.range,
                                    ) {
                                        Some(MessageEvent::HoverURL(url))
                                    } else {
                                        //Not an url, no need to wait for next round
                                        should_clear = true;
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                self.last_hovered = Some(Hovered::new(range));
                                None
                            };

                            if should_clear {
                                self.last_hovered = None;
                            }

                            result
                        } else {
                            self.last_hovered = None;
                            None
                        }
                    }
                    _ => None,
                },
                crossterm::event::Event::Key(key_event) => match key_event.code {
                    KeyCode::PageUp => {
                        self.scroll_page_up();
                        None
                    }
                    KeyCode::PageDown => {
                        self.scroll_page_down();
                        None
                    }
                    KeyCode::Home => {
                        self.scroll_offset = self.get_total_lines();
                        None
                    }
                    KeyCode::End => {
                        self.scroll_offset = 0;
                        None
                    }
                    _ => None,
                },
                _ => None,
            }
        } else {
            match event {
                crate::event_handler::Event::Tick => {
                    let mut should_clear = false;
                    let result = if let Some(last_hovered) = &mut self.last_hovered {
                        if std::time::Instant::now().duration_since(last_hovered.time)
                            > Duration::from_secs(2)
                        {
                            if let Some(url) = self.messages.get_url_from_range(
                                self.current_server_id,
                                &self.current_channel,
                                &last_hovered.range,
                            ) {
                                Some(MessageEvent::HoverURL(url))
                            } else {
                                //Not an url, no need to wait for next round
                                should_clear = true;
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if should_clear {
                        self.last_hovered = None;
                    }

                    result
                }
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_SERVER_ID: usize = 0;
    #[test]
    fn test_find_index() {
        pub const TEXT_START: usize = TIME_LENGTH + NICKNAME_LENGTH + SEPARATOR_LENGTH;

        let mut discuss = DiscussWidget::new();
        discuss.set_current_channel(Some(TEST_SERVER_ID), "test");

        discuss.content_width = 4;
        discuss.max_visible_height = 4;
        discuss.area.width = (TEXT_START + discuss.content_width) as u16;
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "test",
            MessageContent::new_message(None, "HELLO".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "test",
            MessageContent::new_message(None, "HELLO".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "test",
            MessageContent::new_message(None, "HELLO".to_string()),
        );
        discuss.scroll_offset = 0;

        assert_eq!(discuss.scroll_offset, 0);
        let mouse_x = TEXT_START as u16;
        assert_eq!(
            discuss.get_current_line_index_character(0, mouse_x),
            Some((1, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(2, mouse_x),
            Some((2, 0))
        );
        assert_eq!(
            discuss.get_current_line_index_character(1, mouse_x),
            Some((1, 4))
        );

        assert_eq!(
            discuss.get_current_line_index_character(3, mouse_x),
            Some((2, 4))
        );
        assert_eq!(discuss.get_current_line_index_character(4, mouse_x), None);

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
        let mut discuss = DiscussWidget::new();
        discuss.content_width = 10;
        discuss.set_current_channel(Some(TEST_SERVER_ID), "");
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.scroll_offset = 0;
        assert_eq!(discuss.get_total_lines(), 9);
    }

    #[test]
    fn test_can_scroll_up() {
        let mut discuss = DiscussWidget::new();
        discuss.set_current_channel(Some(TEST_SERVER_ID), "");

        discuss.content_width = 5;
        discuss.max_visible_height = 3;

        discuss.add_line(
            Some(TEST_SERVER_ID),
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "",
            MessageContent::new_info("aa aaaa aaaaa aa aaa".to_string()),
        );
        discuss.scroll_offset = 0;
        assert_eq!(discuss.possible_scroll_up(10), 10);
        assert_eq!(discuss.possible_scroll_up(100), 12);
    }

    #[test]
    fn test_should_hightlight() {
        let current_nick = "nickname".to_string();

        let discuss = DiscussWidget::new();
        assert!(discuss.should_highlight(&current_nick, "my nickname is "));
        assert!(!discuss.should_highlight(&current_nick, "my nicknameis "));
        assert!(discuss.should_highlight(&current_nick, "nickname"));
        assert!(discuss.should_highlight(&current_nick, " nickname"));
        assert!(discuss.should_highlight(&current_nick, "nickname "));
        assert!(discuss.should_highlight(&current_nick, ",nickname "));
    }

    #[test]
    fn test_render_rows() {
        let mut discuss = DiscussWidget::new();
        discuss.set_current_channel(Some(TEST_SERVER_ID), "test");

        discuss.add_line(
            Some(TEST_SERVER_ID),
            "test",
            MessageContent::new_message(None, "HELLO".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "test",
            MessageContent::new_message(None, "HELLO".to_string()),
        );
        discuss.add_line(
            Some(TEST_SERVER_ID),
            "test",
            MessageContent::new_message(None, "HELLO".to_string()),
        );

        discuss.content_width = 10;
        assert_eq!(discuss.collect_visible_rows(None).len(), 3);

        discuss.content_width = 4;
        discuss.scroll_offset = 0;
        assert_eq!(discuss.collect_visible_rows(None).len(), 6);

        discuss.content_width = 4;
        discuss.scroll_offset = 0;
        discuss.max_visible_height = 2;
        let rows = discuss.collect_visible_rows(None);
        assert_eq!(rows.len(), 2);

        discuss.content_width = 4;
        discuss.scroll_offset = 1;
        discuss.max_visible_height = 2;
        let rows = discuss.collect_visible_rows(None);
        assert_eq!(rows.len(), 2);
    }
}
