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
use std::borrow::Cow;
use textwrap::wrap;

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
    pub fn new(source: Option<String>, content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source,
            length_without_format: get_size_without_format(&content),
            content,
            kind: MessageKind::Normal,
        }
    }

    pub fn new_message(source: Option<String>, content: String, current_nickname: String) -> Self {
        let kind = if content.contains(&current_nickname) {
            MessageKind::Highlight
        } else {
            MessageKind::Normal
        };
        Self {
            time: std::time::SystemTime::now(),
            source,
            length_without_format: get_size_without_format(&content),
            content,
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

    pub fn new_info(content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source: None,
            kind: MessageKind::Info,
            length_without_format: get_size_without_format(&content),
            content,
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

    pub fn create_rows(&self, content_width: u16) -> Vec<Row<'_>> {
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

        visible_rows.push(vec![
            Cell::from(time_str),
            Cell::from(source_str).style(nickname_style),
            Cell::from("┃ "),
        ]);

        visible_rows.extend(vec![
            vec![
                Cell::from(format!("{:<width$}", " ", width = TIME_LENGTH)),
                Cell::from(format!("{:<width$}", " ", width = NICKNAME_LENGTH))
                    .style(nickname_style),
                Cell::from("┃ ")
            ];
            wrapped.len() - 1
        ]);

        //FIXME: colors on multiline is broken
        for (i, w) in wrapped.into_iter().enumerate() {
            if let Some(row) = visible_rows.get_mut(i) {
                row.push(Cell::from(Line::from(
                    crate::irc_view::message_parser::to_spans(w, Some(default_style)),
                )))
            }
        }

        visible_rows.into_iter().map(Row::new).collect()
    }

    pub fn get_message_length(&self) -> usize {
        self.length_without_format
    }

    pub fn get_wrapped_line(&self, width: usize) -> Vec<Cow<'_, str>> {
        textwrap::wrap(&self.content, width)
    }
}

#[cfg(test)]
mod test {
    use crate::irc_view::message_content::MessageContent;

    #[test]
    fn test_url_find() {
        let message = MessageContent::new(None, "https://test.com".to_string());
        assert_eq!(message.get_url(0), Some("https://test.com"));
        assert_eq!(message.get_url(100), None);

        let message = MessageContent::new(None, "a aa aa https://test.com".to_string());
        assert_eq!(message.get_url(0), None);
        assert_eq!(message.get_url(100), None);
        assert_eq!(message.get_url(10), Some("https://test.com"));
    }
}
