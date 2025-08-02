use textwrap::wrap;

use crate::irc_view::{
    color_user::nickname_color,
    dimension_discuss::{NICKNAME_LENGTH, TIME_LENGTH},
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
    kind: MessageKind,
}

impl MessageContent {
    pub fn new(source: Option<String>, content: &str) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source,
            content: content.to_string(),
            kind: MessageKind::Normal,
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
            kind,
        }
    }

    pub fn new_error(content: String) -> Self {
        Self {
            time: std::time::SystemTime::now(),
            source: None,
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
        }
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

    pub fn create_rows(&self, content_width: u16, focus_style: &Style) -> Vec<Row> {
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
            self.source.as_ref().unwrap_or(&"".to_string()),
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
}
