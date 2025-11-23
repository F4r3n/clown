use crate::irc_view::spell_checker::SpellChecker;
use crate::message_event::MessageEvent;
use crate::{component::Draw, event_handler::Event};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct CInput {
    input: InputWidget,
    /// Current input mode
    area: Rect,

    spell_checker: Option<SpellChecker>,
    spellchecker_task: Option<crate::async_task::AsyncTask<SpellChecker>>,
}

impl Draw for CInput {
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect) {
        self.area = area;
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = self.input.set_visual_scroll(width as usize);

        let input = Paragraph::new(Line::from(vec![
            Span::from(">").style(Style::default().fg(ratatui::style::Color::Cyan)),
            Span::from(self.input.value()),
        ]))
        .scroll((0, scroll as u16));
        frame.render_widget(input, area);

        // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
        let x = self.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y))
    }
}

impl crate::component::EventHandler for CInput {
    fn get_area(&self) -> Rect {
        self.area
    }

    fn handle_actions(&mut self, event: &MessageEvent) -> Option<MessageEvent> {
        match event {
            MessageEvent::InitSpellChecker(language) => {
                self.spellchecker_task = Some(crate::async_task::AsyncTask {
                    handle: Some(SpellChecker::async_build(&language)),
                    result: None,
                });
            }
            _ => {}
        }

        None
    }

    fn handle_events(&mut self, event: &crate::event_handler::Event) -> Option<MessageEvent> {
        match event {
            crate::event_handler::Event::Crossterm(event) => {
                if let Some(key_event) = event.as_key_event() {
                    match key_event.code {
                        KeyCode::Enter => {
                            let m = self.get_current_input().to_string();
                            self.reset_value();
                            if !m.is_empty() {
                                Some(MessageEvent::MessageInput(m))
                            } else {
                                None
                            }
                        }
                        _ => {
                            self.input.handle_key_events(&key_event);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            crate::event_handler::Event::Tick => {
                self.handle_spellchecker();
                None
            }
            _ => None,
        }
    }
}

impl CInput {
    pub fn new() -> Self {
        Self {
            input: InputWidget::default(),
            area: Rect::default(),
            spell_checker: None,
            spellchecker_task: None,
        }
    }

    pub fn get_current_input(&self) -> &str {
        self.input.value()
    }

    pub fn reset_value(&mut self) {
        self.input.reset();
    }

    fn handle_spellchecker(&mut self) {
        if self.spellchecker_task.as_mut().is_some_and(|v| v.poll())
            && let Some(spell_task) = self.spellchecker_task.take()
        {
            self.spell_checker = spell_task.take_result();
        }
    }
}

#[derive(Default, Debug, Clone)]
struct InputWidget {
    value: String,
    cursor_position: usize, // byte position in the string
    visual_scroll: usize,   // horizontal scroll offset (columns)
}

impl InputWidget {
    pub fn visual_cursor(&self) -> usize {
        UnicodeWidthStr::width(&self.value[..self.cursor_position])
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn reset(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
        self.visual_scroll = 0;
    }

    /// Adjust horizontal scroll so the cursor stays visible within the given width.
    pub fn set_visual_scroll(&mut self, width: usize) -> usize {
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

    fn delete_previous_word(&mut self) {
        if self.cursor_position == 0 || self.cursor_position > self.value.len() {
            return;
        }

        if let Some((idx, _ch)) = self.value[..self.cursor_position]
            .char_indices()
            .rev()
            .find(|&(_, ch)| ch.is_whitespace())
        {
            self.cursor_position = idx;
        }
    }

    fn move_cursor_end(&mut self) {
        self.cursor_position = self.value.len();
    }

    fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    pub fn handle_key_events(&mut self, key_event: &KeyEvent) {
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
            _ => {}
        }
    }
}

#[cfg(test)]
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

        assert_eq!(w.value(), "abc");
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
        assert_eq!(w.value(), "ab");
        assert_eq!(w.cursor_position, 2);

        w.handle_key_events(&make_key(KeyCode::Backspace));
        assert_eq!(w.value(), "a");
        assert_eq!(w.cursor_position, 1);
    }

    #[test]
    fn test_delete_previous_word_ctrl_w() {
        let mut w = InputWidget::default();
        w.value = "hello world".to_string();
        w.cursor_position = w.value.len();

        w.handle_key_events(&make_ctrl_w());
        assert_eq!(w.cursor_position, 5); // after "hello"
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
        // “é” is width 1, “你” is width 2
        let mut w = InputWidget::default();
        w.value = "aé你".to_string();

        // cursor after "aé"
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

        let new_scroll = w.set_visual_scroll(5);

        // Cursor column is 10 → must scroll so cursor is at the far right
        // scroll = cursor_col + 1 - width = 10 + 1 - 5 = 6
        assert_eq!(new_scroll, 6);
    }

    #[test]
    fn test_reset() {
        let mut w = InputWidget::default();
        w.value = "abc".to_string();
        w.cursor_position = 3;
        w.visual_scroll = 5;

        w.reset();

        assert_eq!(w.value(), "");
        assert_eq!(w.cursor_position, 0);
        assert_eq!(w.visual_scroll, 0);
    }
}
