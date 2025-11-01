use textwrap::wrap;

use crate::irc_view::{
    color_user::nickname_color,
    dimension_discuss::{NICKNAME_LENGTH, TIME_LENGTH},
    message_parser::get_size_without_format,
};
use chrono::{DateTime, Local, Timelike};
use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Cell, Row},
};

#[derive(Debug, PartialEq, Clone)]
enum MessageKind {
    Error,
    Info,
    Normal,
    Highlight,
}

#[derive(PartialEq, Debug, Clone)]
pub struct MessageContent {
    time: std::time::SystemTime, /*Generated time */
    source: Option<String>,      /*Source*/
    content: String,             /*Content */
    length_without_format: usize,
    kind: MessageKind,
}

impl MessageContent {
    pub fn new(source: Option<String>, content: &str) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source,
            content: content.to_string(),
            kind: MessageKind::Normal,
            length_without_format: get_size_without_format(content),
        }
    }

    pub fn new_message(source: Option<String>, content: &str, current_nickname: &str) -> Self {
        let kind = if content.contains(current_nickname) {
            MessageKind::Highlight
        } else {
            MessageKind::Normal
        };
        Self {
            time: std::time::SystemTime::now(),
            source,
            content: content.to_string(),
            length_without_format: get_size_without_format(content),
            kind,
        }
    }

    pub fn new_error(content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source: None,
            length_without_format: get_size_without_format(&content),
            content,
            kind: MessageKind::Error,
        }
    }

    pub fn new_info(content: &str) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source: None,
            content: content.to_string(),
            kind: MessageKind::Info,
            length_without_format: get_size_without_format(content),
        }
    }

    pub fn get_url(&self, character_pos: usize) -> Option<&str> {
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
        let word = &text[start..end];
        if word.starts_with("https://") {
            return Some(word);
        }
        None
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

    pub fn create_rows(&self, content_width: u16, focus_style: &Style) -> Vec<Row<'_>> {
        let mut visible_rows = Vec::new();
        let time_str = format!("{:>width$}", self.time_format(), width = TIME_LENGTH);
        let mut nickname_style = if let Some(source) = &self.source {
            Style::default().fg(nickname_color(source))
        } else {
            Style::default()
        };
        if self.kind.eq(&MessageKind::Highlight) {
            nickname_style = nickname_style.bg(Color::LightRed);
        }
        let source_str = format!(
            "{:<width$}",
            self.source.as_deref().unwrap_or_default(),
            width = NICKNAME_LENGTH
        );
        let default_style = match &self.kind {
            MessageKind::Error => Style::default().fg(Color::Red),
            MessageKind::Info => Style::default().fg(Color::LightBlue),
            MessageKind::Normal => Style::default(),
            _ => Style::default(),
        };
        let wrapped = wrap(&self.content, content_width as usize);
        let first_part = wrapped
            .first()
            .map(|v| crate::irc_view::message_parser::to_spans(v, Some(default_style)))
            .unwrap_or_default();
        let style_first_part = first_part.last().map(|v| v.style);

        visible_rows.push(Row::new(vec![
            Cell::from(time_str),
            Cell::from(source_str).style(nickname_style),
            Cell::from("┃ ").style(*focus_style),
            Cell::from(Line::from(first_part)),
        ]));
        for w in wrapped.iter().skip(1) {
            visible_rows.push(Row::new(vec![
                Cell::from(format!("{:<width$}", " ", width = TIME_LENGTH)),
                Cell::from(format!("{:<width$}", " ", width = NICKNAME_LENGTH))
                    .style(nickname_style),
                Cell::from("┃ ").style(*focus_style),
                Cell::from(Line::from(crate::irc_view::message_parser::to_spans(
                    w,
                    style_first_part,
                ))),
            ]));
        }
        visible_rows
    }

    pub fn get_message_length(&self) -> usize {
        self.length_without_format
    }
}

#[cfg(test)]
mod test {
    use crate::irc_view::message_content::MessageContent;

    #[test]
    fn test_url_find() {
        let message = MessageContent::new(None, "https://test.com");
        assert_eq!(message.get_url(0), Some("https://test.com"));
        assert_eq!(message.get_url(100), None);

        let message = MessageContent::new(None, "a aa aa https://test.com");
        assert_eq!(message.get_url(0), None);
        assert_eq!(message.get_url(100), None);
        assert_eq!(message.get_url(10), Some("https://test.com"));
    }
}
