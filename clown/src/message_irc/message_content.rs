use crate::message_irc::message_parser::{get_width_without_format, strip_irc_formatting_cow};
use crate::message_irc::textwrapper::{WrappedLine, wrap_spans, wrapped_line_count};
use chrono::{DateTime, Local};
use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Cell, Row},
};
use std::borrow::Cow;

const SPACES: &str = "                  "; //Max 18 spaces
fn spaces(n: u16) -> &'static str {
    SPACES.get(..n as usize).unwrap_or_default()
}

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MessageKind {
    Error,
    Info,
    Normal,
    Highlight,
    Action,
    Notice,
}

pub enum TimeFormat {
    Hour,
    Day,
}

impl TimeFormat {
    pub fn length(&self) -> u16 {
        match self {
            Self::Hour => 8,
            Self::Day => 8,
        }
    }

    pub fn chrono_fmt(&self) -> &'static str {
        match self {
            Self::Hour => "%H:%M:%S",
            Self::Day => "%d/%m/%y",
        }
    }
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

    #[cfg(test)]
    pub fn get_content(&self) -> &str {
        &self.content
    }

    pub fn is_log(&self) -> bool {
        self.is_log
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

    pub fn with_log(mut self) -> Self {
        self.is_log = true;
        self
    }

    pub fn from_kind(kind: MessageKind, source: Option<String>, content: String) -> Option<Self> {
        let builder = match kind {
            MessageKind::Info => MessageContent::info(content),
            MessageKind::Error => MessageContent::error(content),
            MessageKind::Normal => MessageContent::message(source, content),
            _ => return None,
        };
        Some(builder)
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

    fn time_format(&self, format: &TimeFormat) -> impl std::fmt::Display {
        let datetime: DateTime<Local> = self.time.into();
        datetime.format(format.chrono_fmt())
    }

    pub fn create_rows(
        &self,
        content_width: u16,
        color_source: Option<ratatui::style::Color>,
        time_format: Option<&TimeFormat>,
        nickname_length: u16,
        rows_to_skip: usize,
        rows_to_take: usize,
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
        let time_length = time_format.as_ref().map(|v| v.length()).unwrap_or(0);
        let time_pad = spaces(time_length);
        let nick_pad = spaces(nickname_length);

        let visible_rows = wrapped
            .into_iter()
            .skip(rows_to_skip)
            .take(rows_to_take)
            .enumerate()
            .map(move |(offset, w)| {
                let is_header = rows_to_skip == 0 && offset == 0;
                [
                    if is_header {
                        time_format
                            .map(|v| {
                                Cell::from(format!(
                                    "{:>width$}",
                                    self.time_format(v),
                                    width = time_length as usize
                                ))
                            })
                            .unwrap_or_else(|| Cell::default().column_span(1))
                    } else {
                        Cell::from(time_pad)
                    },
                    if is_header {
                        Cell::from(format!(
                            "{:<width$}",
                            self.source.as_deref().unwrap_or_default(),
                            width = nickname_length as usize
                        ))
                    } else {
                        Cell::from(nick_pad)
                    }
                    .style(nickname_style),
                    Cell::from("┃ ").style(separator_style),
                    Cell::from(Line::from(w.spans)),
                ]
            });

        visible_rows.map(Row::new)
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
    use crate::message_irc::message_content::{MessageContent, TimeFormat, WordPos};
    use ratatui::buffer::Buffer;
    use ratatui::layout::{Constraint, Rect};
    use ratatui::widgets::{Row, Table, Widget};

    const NICK_LEN: u16 = 10;

    /// Renders rows through the same table layout the discuss widget uses, and
    /// returns one trimmed string per terminal line.
    fn render_rows(rows: Vec<Row<'_>>, term_width: u16) -> Vec<String> {
        if rows.is_empty() {
            return Vec::new();
        }
        let area = Rect::new(0, 0, term_width, rows.len() as u16);
        let mut buf = Buffer::empty(area);
        Table::new(
            rows,
            [
                Constraint::Length(TimeFormat::Hour.length().saturating_add(1)),
                Constraint::Max(NICK_LEN.saturating_add(1)),
                Constraint::Length(2),
                Constraint::Min(10),
            ],
        )
        .column_spacing(0)
        .render(area, &mut buf);

        (0..area.height)
            .map(|y| {
                (0..term_width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn render_window(
        message: &MessageContent,
        content_width: u16,
        skip: usize,
        take: usize,
    ) -> Vec<String> {
        render_rows(
            message
                .create_rows(
                    content_width,
                    None,
                    Some(&TimeFormat::Hour),
                    NICK_LEN,
                    skip,
                    take,
                )
                .collect(),
            40,
        )
    }

    #[test]
    fn create_rows_window_matches_full_render() {
        let message = MessageContent::message(
            Some("alice".to_string()),
            "aaa bbb ccc ddd eee fff".to_string(),
        );
        let content_width: u16 = 8;
        let total = message.wrapped_line_count(content_width as usize);
        assert_eq!(total, 3, "precondition: message wraps to 3 rows");

        let full = render_window(&message, content_width, 0, total);
        assert_eq!(full.len(), total);

        for skip in 0..=total + 1 {
            for take in 0..=total + 1 {
                let expected: Vec<String> = full.iter().skip(skip).take(take).cloned().collect();

                let actual = render_window(&message, content_width, skip, take);

                assert_eq!(
                    actual, expected,
                    "create_rows(skip={skip}, take={take}) diverged from the full render"
                );
            }
        }
    }

    #[test]
    fn create_rows_skipped_window_has_no_header() {
        let message = MessageContent::message(
            Some("alice".to_string()),
            "aaa bbb ccc ddd eee fff".to_string(),
        );

        let rows = render_window(&message, 8, 1, 2);

        assert_eq!(rows.len(), 2, "2 rows remain after skipping 1 of 3");
        assert!(
            !rows[0].contains("alice"),
            "first row of a skipped window must be padding, got {:?}",
            rows[0]
        );
        assert!(
            rows[0].contains("ccc ddd"),
            "first row of a skipped window must carry the 2nd wrapped line, got {:?}",
            rows[0]
        );
        assert!(
            rows[1].contains("eee fff"),
            "second row must carry the 3rd wrapped line, got {:?}",
            rows[1]
        );
    }

    /// A zero-height window must produce nothing.
    #[test]
    fn create_rows_empty_window_yields_no_rows() {
        let message = MessageContent::message(Some("alice".to_string()), "hello world".to_string());

        assert!(render_window(&message, 20, 0, 0).is_empty());
        assert!(render_window(&message, 20, 5, 3).is_empty());
    }

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
