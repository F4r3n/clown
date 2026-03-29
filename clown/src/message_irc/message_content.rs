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
    Notice,
}

#[derive(PartialEq, Debug, Clone)]
pub struct MessageContent {
    time: std::time::SystemTime, /*Generated time */
    source: Option<String>,      /*Source*/
    content: String,             /*Content */
    width_without_format: usize,
    kind: MessageKind,
    is_log: bool,
}

impl MessageContent {
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

    fn new(kind: MessageKind, source: Option<String>, content: String) -> Self {
        Self {
            width_without_format: get_width_without_format(&content),
            content,
            source,
            kind,
            time: std::time::SystemTime::now(),
            is_log: false,
        }
    }

    // --- Entry Points (Replacing your "new_*" methods) ---

    pub fn action(source: String, content: String) -> Self {
        Self::new(
            MessageKind::Action,
            Some("*".into()),
            format!("{} {}", source, content),
        )
    }

    pub fn message(source: Option<String>, content: String) -> Self {
        Self::new(MessageKind::Normal, source, content)
    }

    pub fn highlight(source: Option<String>, content: String) -> Self {
        Self::new(MessageKind::Highlight, source, content)
    }

    pub fn error(content: String) -> Self {
        Self::new(MessageKind::Error, None, content)
    }

    pub fn privmsg(target: String, content: String) -> Self {
        Self::new(MessageKind::Normal, Some(format!(">{}<", target)), content)
    }

    pub fn notice(source: Option<String>, content: String) -> Self {
        Self::new(MessageKind::Notice, source, content)
    }

    pub fn info(content: String) -> Self {
        Self::new(MessageKind::Info, None, content)
    }

    // --- Modifiers ---

    pub fn with_time(mut self, time: std::time::SystemTime) -> Self {
        self.time = time;
        self
    }

    pub fn as_log(mut self) -> Self {
        self.is_log = true;
        self
    }

    // --- Finalizer ---

    pub fn build(self) -> MessageContent {
        MessageContent {
            width_without_format: get_width_without_format(&self.content),
            time: self.time,
            source: self.source,
            content: self.content,
            kind: self.kind,
            is_log: self.is_log,
        }
    }

    pub fn from_kind(kind: MessageKind, source: Option<String>, content: String) -> Option<Self> {
        let builder = match kind {
            MessageKind::Info => MessageContent::info(content),
            MessageKind::Error => MessageContent::error(content),
            MessageKind::Normal => MessageContent::message(source, content),
            _ => return None,
        };
        Some(builder.build())
    }
    pub fn get_time(&self) -> std::time::SystemTime {
        self.time
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
            MessageKind::Info => Style::default().fg(Color::LightBlue),
            MessageKind::Action | MessageKind::Notice => Style::default().fg(Color::LightBlue),
            MessageKind::Normal | MessageKind::Highlight => Style::default(),
        };
        let separator_style = if self.is_log {
            Style::default().fg(Color::Gray)
        } else {
            Style::default()
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
            Cell::from("┃ ").style(separator_style),
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
                        Cell::from("┃ ").style(separator_style),
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
        let message = MessageContent::message(None, "aaaaa".to_string());
        assert_eq!(message.wrapped_line_count(2), 3);

        let message1 = MessageContent::message(None, "Use the command /help".to_string());
        assert_eq!(message1.wrapped_line_count(12), 3);

        let message2 =
            MessageContent::message(None, "Try to connect to a.aaaaaaaaa.io...".to_string());
        //2026-01-02T11:57:07.587223Z DEBUG clown::irc_view::main_view: ServerMessage { message: Message { internal: IRCMessage { source: Some(Source { source
        assert_eq!(message2.wrapped_line_count(12), 4);
    }

    #[test]
    fn test_word_find() {
        let message = MessageContent::message(None, "https://test.com".to_string());
        assert_eq!(message.get_word_pos(0), Some(WordPos::from(0, 16)));
        assert_eq!(message.get_word_pos(100), None);

        let message = MessageContent::message(None, "a aa aa https://test.com".to_string());
        assert_eq!(message.get_word_pos(0), Some(WordPos::from(0, 1)));
        assert_eq!(message.get_word_pos(100), None);
        assert_eq!(message.get_word_pos(10), Some(WordPos::from(8, 24)));
    }

    #[test]
    fn test_url_find() {
        let message = MessageContent::message(None, "https://test.com".to_string());
        assert_eq!(
            message.get_url_from_pos(&WordPos::from(0, 16)),
            Some("https://test.com")
        );
        assert_eq!(message.get_url_from_pos(&WordPos::from(0, 100)), None);

        let message = MessageContent::message(None, "a aa aa https://test.com".to_string());
        assert_eq!(message.get_url_from_pos(&WordPos::from(0, 0)), None);
        assert_eq!(message.get_url_from_pos(&WordPos::from(0, 100)), None);
        assert_eq!(
            message.get_url_from_pos(&WordPos::from(8, 24)),
            Some("https://test.com")
        );
    }
}
