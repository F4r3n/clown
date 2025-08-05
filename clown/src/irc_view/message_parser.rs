use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

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

pub fn to_spans<'a>(content: &str, start_style: Option<Style>) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut setting_style = false;
    let mut style_buffer = String::new();
    let mut style = start_style.unwrap_or_default();
    let mut modifier = style.add_modifier & style.sub_modifier;
    let mut colors = [style.fg.unwrap_or_default(), style.bg.unwrap_or_default()];
    let mut index_color = 0;

    for c in content.chars() {
        if c == '\x03' {
            if !buffer.is_empty() {
                spans.push(Span::from(buffer.clone()).style(style.fg(colors[0]).bg(colors[1])));
                buffer.clear();
            }
            setting_style = true;
            style_buffer.clear();
            index_color = 0;
        } else if setting_style && style_buffer.len() < 2 && c.is_ascii_digit() {
            style_buffer.push(c);
        } else if setting_style && c == ',' && index_color == 0 {
            if let Some(color) = colors.get_mut(index_color) {
                *color = irc_to_color(&style_buffer);
            }
            index_color += 1;
            style_buffer.clear();
        } else if c == '\x02' {
            spans.push(Span::from(buffer.clone()).style(style.fg(colors[0]).bg(colors[1])));
            buffer.clear();
            style = toggle_modifier(style, &mut modifier, Modifier::BOLD);
        } else if c == '\x1D' {
            spans.push(Span::from(buffer.clone()).style(style.fg(colors[0]).bg(colors[1])));
            buffer.clear();
            style = toggle_modifier(style, &mut modifier, Modifier::ITALIC);
        } else if c == '\x1F' {
            spans.push(Span::from(buffer.clone()).style(style.fg(colors[0]).bg(colors[1])));
            buffer.clear();
            style = toggle_modifier(style, &mut modifier, Modifier::UNDERLINED);
        } else if c == '\x1E' {
            spans.push(Span::from(buffer.clone()).style(style.fg(colors[0]).bg(colors[1])));
            buffer.clear();
            style = toggle_modifier(style, &mut modifier, Modifier::CROSSED_OUT);
        } else {
            if setting_style {
                setting_style = false;
                if let Some(color) = colors.get_mut(index_color) {
                    *color = irc_to_color(&style_buffer);
                }
            }
            buffer.push(c);
        }
    }
    if !buffer.is_empty() {
        spans.push(Span::from(buffer.clone()).style(style.fg(colors[0]).bg(colors[1])));
    }
    spans
}

#[cfg(test)]
mod tests {
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
    fn test_plain_text() {
        let input = "Hello world";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hello world");
        assert_eq!(fg, Color::default());
        assert_eq!(bg, Color::default());
    }

    #[test]
    fn test_single_color() {
        let input = "\x034Hello";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hello");
        assert_eq!(fg, Color::Red);
        assert_eq!(bg, Color::default());
    }

    #[test]
    fn test_fg_and_bg_color() {
        let input = "\x038,4Hi!";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hi!");
        assert_eq!(fg, Color::Yellow);
        assert_eq!(bg, Color::Red);
    }

    #[test]
    fn test_fg_and_bg_color_double_digits() {
        let input = "\x0308,04Hi!";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 1);
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hi!");
        assert_eq!(fg, Color::Yellow);
        assert_eq!(bg, Color::Red);
    }

    #[test]
    fn test_multispan_multicolor() {
        let input = "A\x034B\x037C";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 3);

        let (a, fg_a, _) = span_data(&spans[0]);
        let (b, fg_b, _) = span_data(&spans[1]);
        let (c, fg_c, _) = span_data(&spans[2]);
        assert_eq!(a, "A");
        assert_eq!(fg_a, Color::default());
        assert_eq!(b, "B");
        assert_eq!(fg_b, Color::Red);
        assert_eq!(c, "C");
        assert_eq!(fg_c, Color::Rgb(252, 127, 0)); // Orange
    }

    #[test]
    fn test_trailing_reset() {
        let input = "\x034Red\x03Normal";
        let spans = to_spans(input, None);
        assert_eq!(spans.len(), 2);

        let (red, fg_red, _) = span_data(&spans[0]);
        let (normal, fg_normal, _) = span_data(&spans[1]);
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
        let (text, fg, bg) = span_data(&spans[0]);
        assert_eq!(text, "Hello");
        assert_eq!(fg, Color::Red);
        assert_eq!(bg, Color::default());
    }

    // Add more tests as needed for edge cases, like incomplete codes, empty input, etc.
}
