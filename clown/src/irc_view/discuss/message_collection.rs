use super::dimension_discuss::{META_LENGTH, NICKNAME_LENGTH, SEPARATOR_LENGTH, TIME_LENGTH};
use super::servers_messages::{Messages, WordRange};

use clown_message_render::message_content::{MessageContent, TimeFormat};
use clown_message_render::textwrapper::wrap_content;

use ratatui::widgets::Row;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

#[derive(Debug)]
pub struct WidgetBox {
    pub size: u16,
    pub padding: u16,
}

impl WidgetBox {
    fn get_full_size(&self) -> u16 {
        self.size.saturating_add(self.padding)
    }

    fn is_empty(&self) -> bool {
        self.get_full_size() == 0
    }
}

pub trait MessageCollection {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Option<&MessageContent>;
    fn iter(&self) -> impl DoubleEndedIterator<Item = &MessageContent>;
    fn is_empty(&self) -> bool;
}

impl MessageCollection for Messages {
    fn len(&self) -> usize {
        Messages::len(self)
    }

    fn get(&self, index: usize) -> Option<&MessageContent> {
        Messages::get(self, index)
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &MessageContent> {
        Messages::iter(self)
    }

    fn is_empty(&self) -> bool {
        Messages::is_empty(self)
    }
}

impl MessageCollection for Vec<MessageContent> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn get(&self, index: usize) -> Option<&MessageContent> {
        self.as_slice().get(index)
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &MessageContent> {
        self.as_slice().iter()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct ScrollbarCache {
    pub start_message_index: usize,
    pub rows_to_skip_in_message: usize,

    pub cache_width: usize,
    pub scroll_offset: usize,
    pub max_visible_height: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    #[default]
    Bottom,
    Top,
}

#[derive(Debug)]
pub struct MessageViewer {
    time_format: Option<TimeFormat>,
    anchor: Anchor,
    vertical_scroll_state: ScrollbarState,
    scroll_offset: usize,
    max_visible_height: usize,
    follow_last: bool,
    area: Rect,
    content_width: usize,
    redraw: bool,
    scroll_cache: ScrollbarCache,
}

impl Default for MessageViewer {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MessageViewer {
    pub fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
        self.follow_last = true;

        self.invalidate_cache();
    }

    pub fn invalidate_cache(&mut self) {
        self.scroll_cache = ScrollbarCache::default();
    }

    pub fn new(time_format: Option<TimeFormat>) -> Self {
        Self {
            time_format,
            anchor: Anchor::Bottom,
            scroll_offset: 0,
            max_visible_height: 10,
            follow_last: true,
            vertical_scroll_state: ScrollbarState::default(),
            area: Rect::default(),
            content_width: 0,
            redraw: true,
            scroll_cache: ScrollbarCache::default(),
        }
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn is_scroll_cache_valid(&self) -> bool {
        self.content_width > 0
            && self.scroll_cache.cache_width == self.content_width
            && self.scroll_cache.scroll_offset == self.scroll_offset
            && self.scroll_cache.max_visible_height == self.max_visible_height
    }

    #[cfg(any(test, feature = "bench"))]
    #[allow(dead_code)]
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
        self.follow_last = offset == 0;
        self.redraw = true;
    }

    #[cfg(test)]
    pub fn get_scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn get_range_from_mouse<M: MessageCollection>(
        &self,
        messages: &M,
        row: u16,
        col: u16,
    ) -> Option<WordRange> {
        let mouse_position = ratatui::prelude::Position::new(col, row);

        if self.area.contains(mouse_position) {
            if let Some((index, character)) =
                self.get_current_line_index_character(messages, row, col)
            {
                messages.get(index).and_then(|v| {
                    v.get_word_pos(character).map(|v| WordRange {
                        line: index,
                        word_pos: v,
                    })
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_current_line_index_character<M: MessageCollection>(
        &self,
        messages: &M,
        mouse_pos_y: u16,
        mouse_pos_x: u16,
    ) -> Option<(usize, usize)> {
        let index_y = mouse_pos_y.saturating_sub(self.area.y) as usize;
        let mouse_pos_x = mouse_pos_x.saturating_sub(self.area.x) as usize;

        let text_start = (self.area.width as usize).saturating_sub(self.content_width);
        if mouse_pos_x < text_start {
            return None;
        }

        let pos_x = mouse_pos_x - text_start;
        if pos_x >= self.content_width {
            return None;
        }

        let mut visible_rows_total = 0;

        let (start_message_index, mut rows_to_skip_in_message) = self.find_viewport_start(messages);

        for (line_index, line) in messages.iter().enumerate().skip(start_message_index) {
            let total_line_rows = line.wrapped_line_count(self.content_width);

            // Calculate available rows after skipping the top part (if any)
            let rows_available = total_line_rows.saturating_sub(rows_to_skip_in_message);

            // Cap strictly to the remaining height
            let rows_remaining = self.max_visible_height.saturating_sub(visible_rows_total);
            let rows_to_take = rows_remaining.min(rows_available);

            if rows_to_take == 0 && rows_remaining == 0 {
                break;
            }

            let stripped = line.stripped_formatting();
            let mut rows = wrap_content(&stripped, self.content_width);
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
                return Some((line_index, char_skipped + char_position + pos_x));
            }
            rows_to_skip_in_message = 0; // Reset for subsequent messages

            visible_rows_total += rows_len;
        }

        None
    }

    fn find_viewport_start<M: MessageCollection>(&self, collection: &M) -> (usize, usize) {
        if self.is_scroll_cache_valid() {
            return (
                self.scroll_cache.start_message_index,
                self.scroll_cache.rows_to_skip_in_message,
            );
        }

        if self.anchor == Anchor::Top {
            let mut rows_from_top = 0;
            for (i, line) in collection.iter().enumerate() {
                let total_rows = line.wrapped_line_count(self.content_width);
                if rows_from_top + total_rows > self.scroll_offset {
                    return (i, self.scroll_offset - rows_from_top);
                }
                rows_from_top += total_rows;
            }
            return (collection.len(), 0);
        }

        let message_count = collection.len();
        let target_visual_top = self.scroll_offset.saturating_add(self.max_visible_height);

        let mut start_message_index = 0;
        let mut rows_from_bottom = 0;
        let mut rows_to_skip_in_message = 0;

        for (i, line) in collection.iter().rev().enumerate() {
            let total_rows = line.wrapped_line_count(self.content_width);
            if rows_from_bottom + total_rows >= target_visual_top {
                start_message_index = message_count.saturating_sub(1).saturating_sub(i);
                rows_to_skip_in_message =
                    (rows_from_bottom + total_rows).saturating_sub(target_visual_top);
                break;
            }
            rows_from_bottom += total_rows;
        }

        (start_message_index, rows_to_skip_in_message)
    }

    pub fn collect_visible_rows<'a, M: MessageCollection, F>(
        &'a self,
        collection: &'a M,
        get_color: F,
        display_time: bool,
    ) -> Vec<Row<'a>>
    where
        F: Fn(&str) -> Color,
    {
        let mut visible_rows = Vec::new();
        if self.content_width == 0 {
            return visible_rows;
        }

        let (start_message_index, mut rows_to_skip_in_message) =
            self.find_viewport_start(collection);

        let mut visible_rows_total = 0;

        for line in collection.iter().skip(start_message_index) {
            let total_line_rows = line.wrapped_line_count(self.content_width);

            // Calculate available rows after skipping the top part (if any)
            let rows_available = total_line_rows.saturating_sub(rows_to_skip_in_message);

            // Cap strictly to the remaining height
            let rows_remaining = self.max_visible_height.saturating_sub(visible_rows_total);
            let rows_to_take = rows_remaining.min(rows_available);

            if rows_to_take == 0 && rows_remaining == 0 {
                break;
            }
            let color = line.get_source().map(&get_color);
            let time_format = self.time_format.as_ref().unwrap_or_else(|| {
                if line.is_log() {
                    &clown_message_render::message_content::TimeFormat::Day
                } else {
                    &clown_message_render::message_content::TimeFormat::Hour
                }
            });

            let rows = line.create_rows(
                self.content_width as u16,
                color,
                display_time.then_some(time_format),
                NICKNAME_LENGTH,
                rows_to_skip_in_message,
                rows_to_take,
            );

            visible_rows_total += rows_to_take;
            visible_rows.extend(rows);

            rows_to_skip_in_message = 0; // Reset for subsequent messages

            if visible_rows_total >= self.max_visible_height {
                break;
            }
        }

        visible_rows
    }

    #[cfg(any(test, feature = "bench"))]
    #[allow(dead_code)]
    pub fn total_lines<M: MessageCollection>(&self, messages: &M) -> usize {
        self.get_fake_total_lines(messages)
    }

    //Does not properly get the number of lines
    //but because its called each rendering time, better be fast
    pub fn get_fake_total_lines<M: MessageCollection>(&self, messages: &M) -> usize {
        if self.content_width == 0 {
            return 0;
        }

        if self.content_width == 1 {
            return messages
                .iter()
                .map(|m| m.get_message_width())
                .sum::<usize>();
        }
        //need divisor >=2
        let div = crate::math::div::DivCeilBy::new(self.content_width);
        messages
            .iter()
            .map(|m| div.apply(m.get_message_width()))
            .sum::<usize>()
    }

    pub fn add_line_scroll(&mut self) {
        if self.follow_last {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
        self.scroll_cache = ScrollbarCache::default();
    }

    pub fn ask_redraw(&mut self) {
        self.redraw = true;
    }

    fn update_scroll_cache<M: MessageCollection>(&mut self, messages: &M) {
        if !self.is_scroll_cache_valid() {
            let (start_message_index, rows_to_skip_in_message) = self.find_viewport_start(messages);
            self.scroll_cache = ScrollbarCache {
                start_message_index,
                rows_to_skip_in_message,
                cache_width: self.content_width,
                max_visible_height: self.max_visible_height,
                scroll_offset: self.scroll_offset,
            };
        }
    }

    // "away" / "toward" are relative to the anchor:
    // with `Anchor::Bottom` away is up, with `Anchor::Top` away is down.
    pub fn possible_scroll_away<M: MessageCollection>(
        &mut self,
        messages: &M,
        offset: usize,
    ) -> usize {
        let current_visible = self.scroll_offset.saturating_add(self.max_visible_height);
        let mut total_rows = 0;
        for line in messages.iter().rev() {
            total_rows += line.wrapped_line_count(self.content_width);
            if total_rows >= offset.saturating_add(current_visible) {
                return offset;
            }
        }
        total_rows.saturating_sub(current_visible)
    }

    pub fn scroll_away_no_check(&mut self, offset: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(offset);
        self.follow_last = false;
        self.redraw = true;
    }

    pub fn scroll_toward_no_check(&mut self, offset: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(offset);
        self.follow_last = self.scroll_offset == 0;
        self.redraw = true;
    }

    pub fn scroll_page_away<M: MessageCollection>(&mut self, messages: &M) -> usize {
        let offset = self.possible_scroll_away(messages, self.max_visible_height);
        if offset > 0 {
            self.scroll_away_no_check(offset);
        }
        offset
    }

    pub fn get_max_visible_height(&self) -> usize {
        self.max_visible_height
    }

    fn compute_time_size(&self, display_time: bool, row: u16) -> WidgetBox {
        let time_size = if display_time && row.saturating_div(3) > META_LENGTH {
            Some(TIME_LENGTH)
        } else {
            None
        };
        let time_padding = time_size.map(|_| 1).unwrap_or(0);
        WidgetBox {
            padding: time_padding,
            size: time_size.unwrap_or(0),
        }
    }

    pub fn render<M: MessageCollection, F>(
        &mut self,
        messages: &M,
        get_color: F,
        display_time: bool,
        frame: &mut Frame<'_>,
        area: Rect,
    ) where
        F: Fn(&str) -> Color,
    {
        if self.redraw {
            self.redraw = false;
        }

        self.area = area;
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),    // Meta (time + source)
                Constraint::Length(1), // Scrollbar
            ])
            .split(area);
        let row = layout.first().map(|v| v.width).unwrap_or(0);
        // Set how many lines can be shown
        self.max_visible_height = area.height as usize;
        let time_box = self.compute_time_size(display_time, row);
        let constraints = [
            Constraint::Length(time_box.get_full_size()), // time
            Constraint::Max(NICKNAME_LENGTH.saturating_add(1)), // nickname
            Constraint::Length(SEPARATOR_LENGTH),         // separator
            Constraint::Min(10),                          // Content
        ];
        let mut row_area = self.area;
        row_area.width = row;
        let fake_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(row_area);

        self.content_width = fake_layout.get(3).map(|v| v.width).unwrap_or(0) as usize;

        let number_lines = self.get_fake_total_lines(messages);
        self.update_scroll_cache(messages);

        let visible_rows = self.collect_visible_rows(messages, get_color, !time_box.is_empty());

        let visible_length = visible_rows.len();
        {
            let text_style = Style::default().fg(Color::White);
            //TODO: replace render row by own renderer
            // remove the rows allocation
            // no more need of constraint
            let table = Table::new(visible_rows, constraints)
                .column_spacing(0)
                .style(text_style);

            if let Some(layout) = layout.first() {
                frame.render_widget(table, *layout)
            }
        }

        self.vertical_scroll_state = ScrollbarState::new(number_lines)
            .viewport_content_length(visible_length)
            .position(match self.anchor {
                Anchor::Bottom => number_lines.saturating_sub(self.scroll_offset),
                Anchor::Top => self.scroll_offset,
            });
        if let Some(layout_1) = layout.get(1)
            && layout_1.width > 0
        {
            frame.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::default().fg(if self.follow_last {
                        Color::White
                    } else {
                        Color::LightRed
                    })),
                *layout_1,
                &mut self.vertical_scroll_state,
            );
        }
    }

    pub fn get_area(&self) -> Rect {
        self.area
    }
    pub fn need_redraw(&self) -> bool {
        self.redraw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_messages() -> Vec<MessageContent> {
        (0..3)
            .map(|_| MessageContent::message(None, "HELLO".to_string()))
            .collect()
    }

    fn no_color(_: &str) -> Color {
        Color::White
    }

    #[test]
    fn test_find_index() {
        pub const TEXT_START: usize = (TIME_LENGTH + NICKNAME_LENGTH + SEPARATOR_LENGTH) as usize;

        let messages = hello_messages();
        let mut viewer = MessageViewer::new(None);

        viewer.content_width = 4;
        viewer.max_visible_height = 4;
        viewer.area.width = (TEXT_START + viewer.content_width) as u16;
        viewer.scroll_offset = 0;

        let mouse_x = TEXT_START as u16;
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 0, mouse_x),
            Some((1, 0))
        );
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 2, mouse_x),
            Some((2, 0))
        );
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 1, mouse_x),
            Some((1, 4))
        );

        assert_eq!(
            viewer.get_current_line_index_character(&messages, 3, mouse_x),
            Some((2, 4))
        );
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 4, mouse_x),
            None
        );

        viewer.scroll_offset = 1;
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 0, mouse_x),
            Some((0, 4))
        );
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 1, mouse_x),
            Some((1, 0))
        );
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 2, mouse_x),
            Some((1, 4))
        );

        assert_eq!(
            viewer.get_current_line_index_character(&messages, 3, mouse_x),
            Some((2, 0))
        );
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 4, mouse_x),
            Some((2, 4))
        );
        assert_eq!(
            viewer.get_current_line_index_character(&messages, 0, (TEXT_START + 100) as u16),
            None
        );
    }

    #[test]
    fn test_can_scroll_up() {
        let messages: Vec<MessageContent> = (0..3)
            .map(|_| MessageContent::info("aa aaaa aaaaa aa aaa".to_string()))
            .collect();
        let mut viewer = MessageViewer::new(None);

        viewer.content_width = 5;
        viewer.max_visible_height = 3;
        viewer.scroll_offset = 0;
        assert_eq!(viewer.possible_scroll_away(&messages, 10), 10);
        assert_eq!(viewer.possible_scroll_away(&messages, 100), 12);
    }

    #[test]
    fn test_render_rows() {
        let messages = hello_messages();
        let mut viewer = MessageViewer::new(None);

        viewer.content_width = 10;
        assert_eq!(
            viewer.collect_visible_rows(&messages, no_color, true).len(),
            3,
        );

        viewer.content_width = 4;
        viewer.scroll_offset = 0;
        assert_eq!(
            viewer.collect_visible_rows(&messages, no_color, true).len(),
            6
        );

        viewer.content_width = 4;
        viewer.scroll_offset = 0;
        viewer.max_visible_height = 2;
        let rows = viewer.collect_visible_rows(&messages, no_color, true);
        assert_eq!(rows.len(), 2);

        viewer.content_width = 4;
        viewer.scroll_offset = 1;
        viewer.max_visible_height = 2;
        let rows = viewer.collect_visible_rows(&messages, no_color, true);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_fake_nb_rows() {
        let messages = hello_messages();
        let mut viewer = MessageViewer::new(None);

        viewer.content_width = 10;
        assert_eq!(viewer.get_fake_total_lines(&messages), 3);
    }
}
