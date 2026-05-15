use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthStr;

#[derive(Default, Debug, Clone)]
pub(crate) struct InputWidget {
    pub(super) value: String,
    pub(super) cursor_position: usize, // byte position in the string
    visual_scroll: usize,              // horizontal scroll offset (columns)
}

impl InputWidget {
    pub fn visual_cursor(&self) -> usize {
        UnicodeWidthStr::width(&self.value[..self.cursor_position])
    }

    pub fn get_value(&self) -> &str {
        &self.value
    }

    pub fn reset(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
        self.visual_scroll = 0;
    }

    pub fn reset_with(&mut self, content: String) {
        self.value = content;
        self.cursor_position = self.value.len();
        self.visual_scroll = 0;
    }

    /// Adjust horizontal scroll so the cursor stays visible within the given width.
    pub fn compute_visual_scroll(&mut self, width: usize) -> usize {
        let cursor_col = UnicodeWidthStr::width(&self.value[..self.cursor_position]);
        if cursor_col < self.visual_scroll {
            self.visual_scroll = cursor_col;
        } else if cursor_col >= self.visual_scroll + width {
            self.visual_scroll = cursor_col + 1 - width;
        }
        self.visual_scroll
    }

    fn append_char(&mut self, ch: char) {
        self.value.insert(self.cursor_position, ch);
        self.cursor_position += ch.len_utf8();
    }

    fn append_value(&mut self, content: String) {
        self.value.insert_str(self.cursor_position, &content);
        self.cursor_position += content.len();
    }

    fn delete_char_before_cursor(&mut self) {
        if self.cursor_position == 0 || self.cursor_position > self.value.len() {
            return;
        }

        if let Some((idx, ch)) = self.value[..self.cursor_position]
            .char_indices()
            .next_back()
        {
            self.cursor_position = idx;
            self.value.drain(idx..idx + ch.len_utf8());
        }
    }

    fn delete_char_after_cursor(&mut self) {
        if self.cursor_position < self.value.len() {
            self.value.remove(self.cursor_position);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position == 0 || self.cursor_position > self.value.len() {
            return;
        }

        if let Some((idx, _ch)) = self.value[..self.cursor_position]
            .char_indices()
            .next_back()
        {
            self.cursor_position = idx;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position >= self.value.len() {
            return;
        }

        if let Some((idx, ch)) = self.value[self.cursor_position..].char_indices().next() {
            self.cursor_position += idx + ch.len_utf8();
        }
    }

    pub(super) fn insert_completion(&mut self, start: usize, word: String) {
        let count = self.value[start..]
            .chars()
            .zip(word.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a.len_utf8())
            .sum();

        if start + count < self.cursor_position {
            self.value.drain(start + count..self.cursor_position);
        }
        self.value.insert_str(start + count, &word[count..]);
        self.cursor_position = self.value[..start + count + word[count..].len()].len();
    }

    pub(super) fn find_previous_break(&self, skip_spaces: bool) -> Option<usize> {
        if self.cursor_position == 0 || self.cursor_position > self.value.len() {
            return None;
        }
        let mut cursor_pos = self.cursor_position;
        if skip_spaces {
            for c in self.value[..self.cursor_position].chars().rev() {
                if c.is_whitespace() {
                    cursor_pos = cursor_pos.saturating_sub(c.len_utf8());
                } else {
                    break;
                }
            }
        }

        self.value[..cursor_pos]
            .char_indices()
            .rfind(|&(_, ch)| ch.is_whitespace())
            .map(|v| v.0.saturating_add(1))
    }

    fn delete_previous_word(&mut self) {
        if let Some(cursor_pos) = self.find_previous_break(true) {
            self.value.drain((cursor_pos)..self.cursor_position);
            self.cursor_position = cursor_pos;
        } else {
            self.value.drain(0..self.cursor_position);
            self.cursor_position = 0;
        }
    }

    fn move_cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    pub fn handle_paste(&mut self, content: String) {
        self.append_value(content);
    }

    pub fn handle_key_events(&mut self, key_event: &KeyEvent) {
        if key_event.is_press() || key_event.is_repeat() {
            match key_event.code {
                KeyCode::Char(ch) => {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        if ch == 'w' {
                            self.delete_previous_word();
                        } else if ch == 'h' {
                            self.delete_char_before_cursor();
                        }
                    } else {
                        self.append_char(ch)
                    }
                }
                KeyCode::Backspace => {
                    self.delete_char_before_cursor();
                }
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::End => self.move_cursor_end(),
                KeyCode::Home => self.move_cursor_home(),
                KeyCode::Delete => self.delete_char_after_cursor(),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode) -> crossterm::event::KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_ctrl_w() -> crossterm::event::KeyEvent {
        KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_append_char_and_cursor_movement() {
        let mut w = InputWidget::default();

        w.handle_key_events(&make_key(KeyCode::Char('a')));
        w.handle_key_events(&make_key(KeyCode::Char('b')));
        w.handle_key_events(&make_key(KeyCode::Char('c')));

        assert_eq!(w.get_value(), "abc");
        assert_eq!(w.cursor_position, 3);
    }

    #[test]
    fn test_move_cursor_left_right() {
        let mut w = InputWidget::default();
        w.value = "abc".to_string();
        w.cursor_position = 3;

        w.handle_key_events(&make_key(KeyCode::Left));
        assert_eq!(w.cursor_position, 2);

        w.handle_key_events(&make_key(KeyCode::Left));
        assert_eq!(w.cursor_position, 1);

        w.handle_key_events(&make_key(KeyCode::Right));
        assert_eq!(w.cursor_position, 2);
    }

    #[test]
    fn test_delete_char_before_cursor() {
        let mut w = InputWidget::default();
        w.value = "abc".to_string();
        w.cursor_position = 3;

        w.handle_key_events(&make_key(KeyCode::Backspace));
        assert_eq!(w.get_value(), "ab");
        assert_eq!(w.cursor_position, 2);

        w.handle_key_events(&make_key(KeyCode::Backspace));
        assert_eq!(w.get_value(), "a");
        assert_eq!(w.cursor_position, 1);
    }

    #[test]
    fn test_delete_char_after_cursor() {
        let mut w = InputWidget::default();
        w.value = "abc".to_string();
        w.cursor_position = 2;

        w.handle_key_events(&make_key(KeyCode::Delete));
        assert_eq!(w.get_value(), "ab");
        assert_eq!(w.cursor_position, 2);

        w.value = "a".to_string();
        w.cursor_position = 0;
        w.handle_key_events(&make_key(KeyCode::Delete));
        assert_eq!(w.get_value(), "");
        assert_eq!(w.cursor_position, 0);
    }

    #[test]
    fn test_insert_completion() {
        let mut w = InputWidget::default();

        w.value = "Hello my na".to_string();
        w.cursor_position = 10;

        w.insert_completion(9, "name".to_string());
        assert_eq!(w.value, "Hello my name".to_string());
        assert_eq!(w.cursor_position, 13);

        w.value = "Hello my na".to_string();
        w.cursor_position = 6;
        w.insert_completion(6, "myyo".to_string());
        assert_eq!(w.value, "Hello myyo na".to_string());
        assert_eq!(w.cursor_position, 10);

        w.value = "Hello myyo na".to_string();
        w.cursor_position = 10;
        w.insert_completion(6, "my".to_string());
        assert_eq!(w.value, "Hello my na".to_string());
        assert_eq!(w.cursor_position, 8);

        w.value = "à n".to_string();
        let start = w.value.len();
        w.cursor_position = start;
        w.insert_completion(start - 1, "name".to_string());
        assert_eq!(w.value, "à name".to_string());
        assert_eq!(w.cursor_position, start + 3);
    }

    #[test]
    fn test_find_previous_break() {
        let mut w = InputWidget {
            value: "Hello my na".to_string(),
            cursor_position: 10,
            ..Default::default()
        };

        assert_eq!(
            w.find_previous_break(true).and_then(|v| w.value.get(..v)),
            Some("Hello my ")
        );

        w.value = "Hello my na ".to_string();
        w.cursor_position = 12;
        assert_eq!(
            w.find_previous_break(true).and_then(|v| w.value.get(..v)),
            Some("Hello my ")
        );

        assert_eq!(
            w.find_previous_break(false).and_then(|v| w.value.get(..v)),
            Some("Hello my na ")
        );
    }

    #[test]
    fn test_delete_previous_word_ctrl_w() {
        let mut w = InputWidget::default();
        w.value = "hello world".to_string();
        w.cursor_position = w.value.len();

        w.handle_key_events(&make_ctrl_w());
        assert_eq!(w.cursor_position, 6);
        assert_eq!(w.value, "hello ");

        w.value = "hello world   ".to_string();
        w.cursor_position = w.value.len();
        w.handle_key_events(&make_ctrl_w());
        assert_eq!(w.cursor_position, 6);
        assert_eq!(w.value, "hello ");

        w.handle_key_events(&make_ctrl_w());
        assert_eq!(w.cursor_position, 0);
        assert_eq!(w.value, "");

        w.value = "hello world   ".to_string();
        w.cursor_position = 6;
        w.handle_key_events(&make_ctrl_w());
        assert_eq!(w.cursor_position, 0);
        assert_eq!(w.value, "world   ");
    }

    #[test]
    fn test_move_home_end() {
        let mut w = InputWidget::default();
        w.value = "abcdef".to_string();
        w.cursor_position = 3;

        w.handle_key_events(&make_key(KeyCode::Home));
        assert_eq!(w.cursor_position, 0);

        w.handle_key_events(&make_key(KeyCode::End));
        assert_eq!(w.cursor_position, 6);
    }

    #[test]
    fn test_unicode_width_cursor() {
        let mut w = InputWidget::default();
        w.value = "aé你".to_string();

        w.cursor_position = "aé".len();
        assert_eq!(
            w.visual_cursor(),
            unicode_width::UnicodeWidthStr::width("aé")
        );
    }

    #[test]
    fn test_visual_scroll_keeps_cursor_visible() {
        let mut w = InputWidget::default();
        w.value = "abcdefghijkl".to_string();
        w.cursor_position = 10;

        let new_scroll = w.compute_visual_scroll(5);

        assert_eq!(new_scroll, 6);
    }

    #[test]
    fn test_reset() {
        let mut w = InputWidget::default();
        w.value = "abc".to_string();
        w.cursor_position = 3;
        w.visual_scroll = 5;

        w.reset();

        assert_eq!(w.get_value(), "");
        assert_eq!(w.cursor_position, 0);
        assert_eq!(w.visual_scroll, 0);
    }
}
