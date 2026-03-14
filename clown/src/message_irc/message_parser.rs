use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use unicode_width::UnicodeWidthStr;

fn irc_to_color(code: &str) -> Color {
    if let Ok(code) = code.parse::<u16>() {
        match code {
            0 => Color::White,
            1 => Color::Black,
            2 => Color::Blue,
            3 => Color::Green,
            4 => Color::Red,
            5 => Color::Rgb(127, 63, 0), // Brown (Maroon)
            6 => Color::Magenta,
            7 => Color::Rgb(252, 127, 0), // Orange
            8 => Color::Yellow,
            9 => Color::LightGreen,
            10 => Color::Cyan,
            11 => Color::LightCyan,
            12 => Color::LightBlue,
            13 => Color::Rgb(255, 0, 255), // Pink (Magenta)
            14 => Color::Gray,
            15 => Color::Rgb(210, 210, 210), // Light Grey
            _ => Color::default(),
        }
    } else {
        Color::default()
    }
}

fn toggle_modifier(mut style: Style, current: &mut Modifier, toggled: Modifier) -> Style {
    *current ^= toggled;

    if *current & Modifier::BOLD == Modifier::default() {
        style = style.remove_modifier(toggled);
    } else {
        style = style.add_modifier(toggled);
    }
    style
}

pub fn strip_irc_formatting(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut iter = content.char_indices().peekable();

    while let Some((_, c)) = iter.next() {
        match c {
            '\x02' | '\x1D' | '\x0F' | '\x1F' | '\x1E' => {}
            '\x03' => {
                // Skip up to 2 digits
                let mut count = 0;
                while count < 2
                    && iter
                        .peek()
                        .map(|(_, c)| c.is_ascii_digit())
                        .unwrap_or(false)
                {
                    iter.next();
                    count += 1;
                }
                // Skip optional ,NN background
                if iter.peek().map(|(_, c)| *c == ',').unwrap_or(false) {
                    iter.next(); // consume ','
                    let mut count = 0;
                    while count < 2
                        && iter
                            .peek()
                            .map(|(_, c)| c.is_ascii_digit())
                            .unwrap_or(false)
                    {
                        iter.next();
                        count += 1;
                    }
                }
            }
            _ => out.push(c),
        }
    }

    out
}

pub fn strip_irc_formatting_cow(content: &str) -> std::borrow::Cow<'_, str> {
    if is_string_plain(content) {
        std::borrow::Cow::Borrowed(content) // zero allocation
    } else {
        std::borrow::Cow::Owned(strip_irc_formatting(content))
    }
}

pub fn to_spans<'a>(content: &'a str, start_style: Option<Style>) -> Vec<Span<'a>> {
    if content.is_empty() {
        return vec![];
    }

    let mut style = start_style.unwrap_or_default();
    let mut colors = [style.fg.unwrap_or_default(), style.bg.unwrap_or_default()];
    if is_string_plain(content) {
        return vec![Span::from(content).style(style.fg(colors[0]).bg(colors[1]))];
    }
    let mut modifier = style.add_modifier & style.sub_modifier;
    let mut spans = Vec::new();
    let mut setting_style = false;
    let mut style_buffer = String::new();
    let mut index_color = 0;
    let mut start_index = 0;
    for (i, c) in content.char_indices() {
        if c == '\x03' {
            if start_index != i {
                spans.push(
                    Span::from(&content[start_index..i]).style(style.fg(colors[0]).bg(colors[1])),
                );
            }
            setting_style = true;
            style_buffer.clear();
            index_color = 0;
            start_index = i;
        } else if setting_style && style_buffer.len() < 2 && c.is_ascii_digit() {
            style_buffer.push(c);
            start_index = i;
        } else if setting_style && c == ',' && index_color == 0 {
            if let Some(color) = colors.get_mut(index_color) {
                *color = irc_to_color(&style_buffer);
            }
            index_color += 1;
            style_buffer.clear();
            start_index = i;
        } else if c == '\x02' {
            if start_index != i {
                spans.push(
                    Span::from(&content[start_index..i]).style(style.fg(colors[0]).bg(colors[1])),
                );
            }
            start_index = i + 1;
            style = toggle_modifier(style, &mut modifier, Modifier::BOLD);
        } else if c == '\x1D' {
            if start_index != i {
                spans.push(
                    Span::from(&content[start_index..i]).style(style.fg(colors[0]).bg(colors[1])),
                );
            }
            start_index = i + 1;
            style = toggle_modifier(style, &mut modifier, Modifier::ITALIC);
        } else if c == '\x0F' {
            if start_index != i {
                spans.push(
                    Span::from(&content[start_index..i]).style(style.fg(colors[0]).bg(colors[1])),
                );
            }
            start_index = i + 1;
            style = start_style.unwrap_or_default();
            colors = [style.fg.unwrap_or_default(), style.bg.unwrap_or_default()];
            style_buffer.clear();
        } else if c == '\x1F' {
            if start_index != i {
                spans.push(
                    Span::from(&content[start_index..i]).style(style.fg(colors[0]).bg(colors[1])),
                );
            }
            start_index = i + 1;
            style = toggle_modifier(style, &mut modifier, Modifier::UNDERLINED);
        } else if c == '\x1E' {
            if start_index != i {
                spans.push(
                    Span::from(&content[start_index..i]).style(style.fg(colors[0]).bg(colors[1])),
                );
            }
            start_index = i + 1;
            style = toggle_modifier(style, &mut modifier, Modifier::CROSSED_OUT);
        } else if setting_style {
            setting_style = false;
            if let Some(color) = colors.get_mut(index_color) {
                *color = irc_to_color(&style_buffer);
            }
            start_index = i;
        }
    }
    if start_index < content.len() {
        spans.push(Span::from(&content[start_index..]).style(style.fg(colors[0]).bg(colors[1])));
    }
    spans
}

#[derive(Default)]
pub struct WrappedLine<'a> {
    pub spans: Vec<Span<'a>>,
}

use unicode_width::UnicodeWidthChar;
pub fn wrap_spans<'a>(
    content: &'a str,
    width: usize,
    start_style: Option<Style>,
) -> Vec<WrappedLine<'a>> {
    if width == 0 {
        return vec![];
    }

    let spans = to_spans(content, start_style);
    if spans.len() == 1
        && let Some(first_span) = spans.first()
        && first_span.content.width() < width
    {
        return vec![WrappedLine { spans }];
    }
    let mut lines: Vec<WrappedLine<'a>> = vec![WrappedLine::default()];
    let mut current_width = 0usize;

    for span in spans {
        let style = span.style;
        let mut remaining = span.content.as_ref();

        while !remaining.is_empty() {
            let (mut word, rest) = next_word(remaining);
            let mut word_width = word.width();

            // If word doesn't fit on current line
            if current_width > 0 && current_width + word_width > width {
                lines.push(WrappedLine::default());
                current_width = 0;

                // standard wrapping: ignore leading whitespace on a new line
                let trimmed = word.trim_start();
                if trimmed.len() != word.len() {
                    word = trimmed;
                    word_width = word.width();
                }
            }

            if word_width > width {
                // Force-split logic for words longer than the total width
                let mut temp_start = 0;
                let mut temp_w = 0;
                for (i, c) in word.char_indices() {
                    let cw = c.width().unwrap_or(0);
                    if temp_w + cw > width && temp_w > 0 {
                        let chunk = &word[temp_start..i];
                        if let Some(last) = lines.last_mut() {
                            last.spans.push(Span::styled(chunk.to_owned(), style));
                        }

                        lines.push(WrappedLine::default());
                        temp_start = i;
                        temp_w = 0;
                    }
                    temp_w += cw;
                }
                let chunk = &word[temp_start..];
                if !chunk.is_empty() {
                    if let Some(last) = lines.last_mut() {
                        last.spans.push(Span::styled(chunk.to_owned(), style));
                    }

                    current_width = temp_w;
                }
            } else if !word.is_empty() {
                if let Some(last) = lines.last_mut() {
                    last.spans.push(Span::styled(word.to_owned(), style));
                }

                current_width += word_width;
            }

            remaining = rest;
        }
    }

    // Clean up trailing empty line if the last word fit exactly
    if lines.last().map(|l| l.spans.is_empty()).unwrap_or(false) && lines.len() > 1 {
        lines.pop();
    }

    lines
}

fn next_word(s: &str) -> (&str, &str) {
    // Include leading whitespace with the word
    let trimmed = s.trim_start();
    let leading = &s[..s.len() - trimmed.len()];
    let end = trimmed
        .find(|c: char| c.is_whitespace())
        .unwrap_or(trimmed.len());
    let word_end = leading.len() + end;
    (&s[..word_end], &s[word_end..])
}

pub fn get_width_without_format(content: &str) -> usize {
    if is_string_plain(content) {
        return content.width();
    }
    let mut count = 0;
    let mut i = 0;
    let mut start_i = 0;
    let bytes = content.as_bytes();
    while i < bytes.len() {
        match bytes.get(i) {
            // Simple formatting bytes to skip
            Some(0x02) | Some(0x1D) | Some(0x1E) | Some(0x1F) | Some(0x0F) => {
                count += content[start_i..i].width();
                i += 1;
                start_i = i;
            }

            // Color format: \x03([0-9]{1,2})(,[0-9]{1,2})?
            Some(0x03) => {
                count += content[start_i..i].width();

                i += 1;

                // up to 2 digits
                for _ in 0..2 {
                    if i < bytes.len() && bytes.get(i).is_some_and(|f| f.is_ascii_digit()) {
                        i += 1;
                    } else {
                        break;
                    }
                }

                // optional ",NN"
                if i < bytes.len() && bytes.get(i).is_some_and(|f| f == &b',') {
                    i += 1;
                    for _ in 0..2 {
                        if i < bytes.len() && bytes.get(i).is_some_and(|f| f.is_ascii_digit()) {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
                start_i = i;
            }
            _ => {
                i += 1;
            }
        }
    }
    count += content[start_i..i].width();
    count
}

pub fn is_string_plain(content: &str) -> bool {
    for c in content.bytes() {
        if c == 0x03 || c == 0x01 || c == 0x02 || c == 0x1D || c == 0x1E || c == 0x1F || c == 0x0F {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use unicode_width::UnicodeWidthStr;

    use super::*;

    // Helper to extract text and colors from spans for assertion
    fn span_data<'a>(span: &'a Span<'_>) -> (&'a str, Color, Color) {
        // Assuming you have methods or public fields to get these:
        (
            &span.content,
            span.style.fg.unwrap_or_default(),
            span.style.bg.unwrap_or_default(),
        )
    }
    #[test]
    fn test_is_plain() {
        assert!(is_string_plain("Hello world"));
        assert!(!is_string_plain("\x034Hello"));
    }

    #[test]
    fn test_plain_text() {
        let input = "Hello world";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(spans.first().unwrap());
        assert_eq!(text, "Hello world");
        assert_eq!(fg, Color::default());
        assert_eq!(bg, Color::default());
    }

    #[test]
    fn test_single_color() {
        let input = "\x034Hello";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(spans.first().unwrap());
        assert_eq!(text, "Hello");
        assert_eq!(fg, Color::Red);
        assert_eq!(bg, Color::default());
    }

    #[test]
    fn test_fg_and_bg_color() {
        let input = "\x038,4Hi!";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(spans.first().unwrap());
        assert_eq!(text, "Hi!");
        assert_eq!(fg, Color::Yellow);
        assert_eq!(bg, Color::Red);
    }

    #[test]
    fn test_fg_and_bg_color_double_digits() {
        let input = "\x0308,04Hi!";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(spans.first().unwrap());
        assert_eq!(text, "Hi!");
        assert_eq!(fg, Color::Yellow);
        assert_eq!(bg, Color::Red);
    }

    #[test]
    fn test_multispan_multicolor() {
        let input = "A\x034B\x037C";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 3);

        let (a, fg_a, _) = span_data(spans.first().unwrap());
        let (b, fg_b, _) = span_data(spans.get(1).unwrap());
        let (c, fg_c, _) = span_data(spans.get(2).unwrap());
        assert_eq!(a, "A");
        assert_eq!(fg_a, Color::default());
        assert_eq!(b, "B");
        assert_eq!(fg_b, Color::Red);
        assert_eq!(c, "C");
        assert_eq!(fg_c, Color::Rgb(252, 127, 0)); // Orange
    }

    #[test]
    fn test_faut_dormir_reset() {
        let input = "\x0313f\x0306a\x0302u\x0312t \x0311a\x0310l\x0303l\x0309e\x0308r \x0307d\x0304o\x0305r\x0313m\x0306i\x0302r\x0f";
        assert_eq!(get_width_without_format(input), 17);
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 15);
        let (text, _fg, _bg) = span_data(spans.last().unwrap());
        assert_eq!(text, "r");
    }

    #[test]
    fn test_trailing_reset() {
        let input = "\x034Red\x03Normal";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 2);

        let (red, fg_red, _) = span_data(spans.first().unwrap());
        let (normal, fg_normal, _) = span_data(spans.get(1).unwrap());
        assert_eq!(red, "Red");
        assert_eq!(fg_red, Color::Red);
        assert_eq!(normal, "Normal");
        assert_eq!(fg_normal, Color::default());
    }

    #[test]
    fn test_italic() {
        let input = "\x034Hello";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(spans.first().unwrap());
        assert_eq!(text, "Hello");
        assert_eq!(fg, Color::Red);
        assert_eq!(bg, Color::default());
    }

    #[test]
    fn test_size_without_color() {
        let input = "\x034Hello";
        assert_eq!(get_width_without_format(input), 5);

        let input = "A\x034B\x037C";
        assert_eq!(get_width_without_format(input), 3);
    }
    //(ﾉ´ヮ´)ﾉ *:･ﾟ✧*:･ﾟ✧*:･ﾟ✧*:･ﾟ✧
    #[test]
    fn test_unicode() {
        let input = " ﾟ";
        assert_eq!(input.width(), 1);
        // A + unicode(7) + B = 9
        assert_eq!(get_width_without_format(input), 1);
    }

    #[test]
    fn test_unicode_color() {
        let input = "\x034✧\x034A";
        // A + unicode(7) + B = 9
        assert_eq!(get_width_without_format(input), 2);
    }

    #[test]
    fn test_multiple_formats() {
        let input = "\x0312H\x02e\x1Dl\x1El\x037o\x0F!";
        assert_eq!(get_width_without_format(input), 6);
    }

    #[test]
    fn test_color_limit_and_trailing_digits() {
        // \x03 followed by more than 2 digits should only consume 2
        let input = "\x031234Text";
        let spans = to_spans(input, None);
        // Color should be 12 (Light Blue), and "34Text" should be the content
        assert_eq!(spans[0].content, "34Text");
        assert_eq!(spans[0].style.fg.unwrap_or_default(), Color::LightBlue);
    }

    #[test]
    fn test_background_only_invalid() {
        // IRC usually requires a foreground before a comma-background
        // This tests if your parser handles a comma after a reset or without digits correctly
        let input = "\x03,04Oops";
        let spans = to_spans(input, None);
        // Depending on your parser logic, this might either be plain text starting with ","
        // or it should handle it gracefully.
        // Current logic skips ',' if index_color == 0.
        assert!(spans[0].content.contains("Oops"));
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let spans = to_spans(input, None);
        assert!(spans.is_empty());
    }

    #[test]
    fn test_modifier_toggle_persistence() {
        // Bold on, Color Red, Bold off.
        // "Red" should be Bold+Red, "Still Red" should be Red only.
        let input = "\x02\x034Red\x02Still Red";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 2);

        // First span: Bold + Red
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[0].style.fg.unwrap_or_default(), Color::Red);

        // Second span: Red but NOT Bold
        assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].style.fg.unwrap_or_default(), Color::Red);
    }

    #[test]
    fn test_wrap_exact_width() {
        let input = "AAAA BBBB";
        let width = 4;
        let wrapped = wrap_spans(input, width, None);

        // Should result in two lines
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].spans[0].content, "AAAA");
        // Depending on next_word, the space might be leading the second line
        assert!(wrapped[1].spans[0].content.contains("BBBB"));
    }

    #[test]
    fn test_wrap_force_split_wide_char() {
        // Testing force-split on a 0-width or wide character if applicable
        let input = "🚀🚀🚀🚀"; // Emoji are often width 2
        let width = 4;
        let wrapped = wrap_spans(input, width, None);

        // Each 🚀 is width 2. Two should fit per line.
        assert_eq!(wrapped.len(), 2);
    }

    #[test]
    fn test_global_reset() {
        let input = "\x02\x1F\x034Heavy Red\x0FPlain";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 2);

        assert_eq!(spans[1].content, "Plain");
        assert_eq!(
            spans[1].style,
            Style::new().fg(Color::Reset).bg(Color::Reset)
        );
    }
}
