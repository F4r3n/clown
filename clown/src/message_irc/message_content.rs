use crate::message_irc::message_parser::{get_width_without_format, strip_irc_formatting_cow};
use crate::message_irc::textwrapper::{WrappedLine, wrap_spans, wrapped_line_count};
use chrono::{DateTime, Local};
use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Cell, Row},
};
use std::borrow::Cow;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct WordPos {
    byte_start: usize,
    byte_end: usize,
}

impl WordPos {
    pub fn from(byte_start: usize, byte_end: usize) -> Self {
        Self {
            byte_start,
            byte_end,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MessageKind {
    Error,
    Info,
    Normal,
    Highlight,
    Action,
    Log,
    Notice,
}

#[derive(PartialEq, Debug, Clone)]
pub struct MessageContent {
    time: std::time::SystemTime, /*Generated time */
    source: Option<String>,      /*Source*/
    content: String,             /*Content */
    width_without_format: usize,
    kind: MessageKind,
}

impl MessageContent {
    pub fn new(source: Option<String>, content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source,
            width_without_format: get_width_without_format(&content),
            content,
            kind: MessageKind::Normal,
        }
    }

    pub fn get_source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn get_word_from_pos(&self, pos: &WordPos) -> Option<&str> {
        self.content.get(pos.byte_start..pos.byte_end)
    }

    pub fn get_url_from_pos(&self, pos: &WordPos) -> Option<&str> {
        self.get_word_from_pos(pos)
            .filter(|w| w.starts_with("http://") || w.starts_with("https://"))
    }

    pub fn new_highlight(source: Option<String>, content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source,
            width_without_format: get_width_without_format(&content),
            content,
            kind: MessageKind::Highlight,
        }
    }

    pub fn new_log(time: std::time::SystemTime, source: Option<String>, content: String) -> Self {
        Self {
            time,
            source,
            width_without_format: get_width_without_format(&content),
            content,
            kind: MessageKind::Log,
        }
    }

    pub fn new_error(content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source: None,
            width_without_format: get_width_without_format(&content),
            content,
            kind: MessageKind::Error,
        }
    }

    pub fn new_action(source: String, content: String) -> Self {
        let content = format!("{} {}", source, content);
        let source = Some("*".into());
        Self {
            time: std::time::SystemTime::now(),
            source,
            width_without_format: get_width_without_format(&content),
            content,
            kind: MessageKind::Action,
        }
    }

    pub fn set_time(&mut self, time: std::time::SystemTime) {
        self.time = time
    }

    pub fn get_time(&self) -> std::time::SystemTime {
        self.time
    }

    pub fn new_privmsg(target: String, content: String) -> Self {
        let source = Some(format!(">{}<", target));
        Self {
            time: std::time::SystemTime::now(),
            source,
            width_without_format: get_width_without_format(&content),
            content,
            kind: MessageKind::Normal,
        }
    }

    pub fn new_notice(source: Option<String>, content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source,
            width_without_format: get_width_without_format(&content),
            content,
            kind: MessageKind::Notice,
        }
    }

    pub fn new_info(content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source: None,
            kind: MessageKind::Info,
            width_without_format: get_width_without_format(&content),
            content,
        }
    }

    pub fn from_kind(kind: MessageKind, source: Option<String>, content: String) -> Option<Self> {
        match kind {
            MessageKind::Info => Some(Self::new_info(content)),
            MessageKind::Error => Some(Self::new_error(content)),
            MessageKind::Normal => Some(Self::new(source, content)),
            _ => None,
        }
    }

    pub fn get_word_pos(&self, character_pos: usize) -> Option<WordPos> {
        let text = self.content.as_str(); //Lets say the URL is in a RAW message
        let bytes = text.as_bytes();

        if character_pos >= bytes.len() {
            return None;
        }

        let mut start = character_pos;
        let mut end = character_pos;
        while start > 0
            && !bytes
                .get(start - 1)
                .is_some_and(|c| c.is_ascii_whitespace())
        {
            start -= 1;
        }

        while end < bytes.len() && !bytes.get(end).is_some_and(|c| c.is_ascii_whitespace()) {
            end += 1;
        }
        Some(WordPos::from(start, end))
    }

    fn time_format(&self) -> impl std::fmt::Display {
        let datetime: DateTime<Local> = self.time.into();
        datetime.format("%H:%M:%S")
    }

    pub fn create_rows(
        &self,
        content_width: u16,
        color_source: Option<ratatui::style::Color>,
        time_length: usize,
        nickname_length: usize,
    ) -> impl Iterator<Item = Row<'_>> {
        let mut nickname_style = Style::default();
        if let Some(color_source) = color_source {
            nickname_style = nickname_style.fg(color_source);
        }
        if self.kind.eq(&MessageKind::Highlight) {
            nickname_style = nickname_style.bg(Color::Red).fg(Color::LightYellow);
        }

        let default_style = match &self.kind {
            MessageKind::Error => Style::default().fg(Color::Red),
            MessageKind::Log => Style::default().fg(Color::Gray),
            MessageKind::Info => Style::default().fg(Color::LightBlue),
            MessageKind::Action | MessageKind::Notice => Style::default().fg(Color::LightBlue),
            MessageKind::Normal | MessageKind::Highlight => Style::default(),
        };
        let wrapped = self.wrap_spans(content_width as usize, Some(default_style));
        let mut visible_rows = Vec::with_capacity(wrapped.len());

        visible_rows.push([
            if time_length > 0 {
                Cell::from(format!(
                    "{:>width$}",
                    self.time_format(),
                    width = time_length
                ))
            } else {
                Cell::from("")
            },
            Cell::from(format!(
                "{:<width$}",
                self.source.as_deref().unwrap_or_default(),
                width = nickname_length
            ))
            .style(nickname_style),
            Cell::from("┃ "),
            Cell::from(""),
        ]);

        if wrapped.len() > 1 {
            visible_rows.extend(
                std::iter::repeat_with(|| {
                    [
                        if time_length > 0 {
                            Cell::from(format!("{:<width$}", " ", width = time_length))
                        } else {
                            Cell::from("")
                        },
                        Cell::from(format!("{:<width$}", " ", width = nickname_length))
                            .style(nickname_style),
                        Cell::from("┃ "),
                        Cell::from(""),
                    ]
                })
                .take(wrapped.len() - 1),
            );
        }

        for (i, w) in wrapped.into_iter().enumerate() {
            if let Some(row) = visible_rows.get_mut(i)
                && let Some(last) = row.last_mut()
            {
                *last = Cell::from(Line::from(w.spans.clone()));
            }
        }

        visible_rows.into_iter().map(Row::new)
    }

    pub fn get_message_width(&self) -> usize {
        self.width_without_format
    }

    pub fn wrapped_line_count(&self, width: usize) -> usize {
        wrapped_line_count(&strip_irc_formatting_cow(&self.content), width)
    }

    pub fn stripped_formatting<'a>(&'a self) -> Cow<'a, str> {
        strip_irc_formatting_cow(&self.content)
    }

    pub fn wrap_spans<'a>(
        &'a self,
        width: usize,
        default_style: Option<ratatui::style::Style>,
    ) -> Vec<WrappedLine<'a>> {
        wrap_spans(&self.content, width, default_style)
    }
}

#[cfg(test)]
mod test {
    use crate::message_irc::message_content::{MessageContent, WordPos};

    #[test]
    fn test_wrapped_line_count() {
        let message = MessageContent::new(None, "aaaaa".to_string());
        assert_eq!(message.wrapped_line_count(2), 3);

        let message1 = MessageContent::new(None, "Use the command /help".to_string());
        assert_eq!(message1.wrapped_line_count(12), 3);

        let message2 = MessageContent::new(None, "Try to connect to a.aaaaaaaaa.io...".to_string());
        //2026-01-02T11:57:07.587223Z DEBUG clown::irc_view::main_view: ServerMessage { message: Message { internal: IRCMessage { source: Some(Source { source
        assert_eq!(message2.wrapped_line_count(12), 4);
    }

    #[test]
    fn test_word_find() {
        let message = MessageContent::new(None, "https://test.com".to_string());
        assert_eq!(message.get_word_pos(0), Some(WordPos::from(0, 16)));
        assert_eq!(message.get_word_pos(100), None);

        let message = MessageContent::new(None, "a aa aa https://test.com".to_string());
        assert_eq!(message.get_word_pos(0), Some(WordPos::from(0, 1)));
        assert_eq!(message.get_word_pos(100), None);
        assert_eq!(message.get_word_pos(10), Some(WordPos::from(8, 24)));
    }

    #[test]
    fn test_url_find() {
        let message = MessageContent::new(None, "https://test.com".to_string());
        assert_eq!(
            message.get_url_from_pos(&WordPos::from(0, 16)),
            Some("https://test.com")
        );
        assert_eq!(message.get_url_from_pos(&WordPos::from(0, 100)), None);

        let message = MessageContent::new(None, "a aa aa https://test.com".to_string());
        assert_eq!(message.get_url_from_pos(&WordPos::from(0, 0)), None);
        assert_eq!(message.get_url_from_pos(&WordPos::from(0, 100)), None);
        assert_eq!(
            message.get_url_from_pos(&WordPos::from(8, 24)),
            Some("https://test.com")
        );
    }
}
