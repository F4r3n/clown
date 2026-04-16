use crate::message_irc::message_parser::to_spans;
use ratatui::style::Style;
use ratatui::text::Span;
use unicode_width::UnicodeWidthChar;

pub fn wrapped_line_count(content: &str, width: usize) -> usize {
    if width == 0 || content.is_empty() {
        return 0;
    }

    let mut total_lines = 1;
    let mut chars = content.chars().peekable();
    let mut current_width = 0;

    while let Some(c) = chars.next() {
        let char_width = c.width().unwrap_or(0);

        if c.is_whitespace() {
            current_width += char_width;
            continue;
        }

        // Consume the word
        let mut word_width = char_width;
        while let Some(next_c) = chars.peek() {
            if next_c.is_whitespace() {
                break;
            }
            word_width += next_c.width().unwrap_or(0);
            chars.next();
        }

        // Overflow check
        if current_width > 0 && current_width + word_width > width {
            total_lines += 1;
            current_width = 0;
        }

        // Force split or normal
        if word_width > width {
            let distinct_lines = word_width.div_ceil(width);
            total_lines += distinct_lines - 1;
            current_width = word_width % width;
            if current_width == 0 {
                current_width = width;
            }
        } else {
            current_width += word_width;
        }
    }

    total_lines
}

pub fn wrap_content(content: &str, width: usize) -> Vec<&str> {
    if width == 0 {
        return vec![];
    }
    let mut wrapped_lines = Vec::with_capacity(content.len().div_ceil(width));
    let line = content;
    let mut char_indices = line.char_indices().peekable();
    let mut line_start = 0;
    let mut current_width = 0;
    let mut last_word_end = 0;

    while let Some((i, c)) = char_indices.next() {
        let char_width = c.width().unwrap_or(0);

        if c.is_whitespace() {
            last_word_end = i;
            current_width += char_width;

            continue;
        }

        // Consume the word
        let word_start = i;
        let mut word_width = char_width;
        let mut word_end = i + c.len_utf8();
        while let Some((_next_i, next_c)) = char_indices.peek() {
            if next_c.is_whitespace() {
                break;
            }
            word_width += next_c.width().unwrap_or(0);
            word_end += next_c.len_utf8();
            char_indices.next();
        }

        // Overflow check
        if current_width > 0 && current_width + word_width > width {
            let line_slice = &line[line_start..last_word_end];
            wrapped_lines.push(line_slice.trim_end());
            line_start = word_start;
            current_width = 0;
        }

        // Handle long words
        if word_width > width {
            if current_width > 0 {
                wrapped_lines.push(line[line_start..word_start].trim_end());
            }
            let mut temp_w = 0;
            let mut temp_start = word_start;
            for (ci, cc) in line[word_start..word_end].char_indices() {
                let cw = cc.width().unwrap_or(0);
                if temp_w + cw > width {
                    wrapped_lines.push(&line[temp_start..word_start + ci]);
                    temp_start = word_start + ci;
                    temp_w = 0;
                }
                temp_w += cw;
            }
            line_start = temp_start;
            last_word_end = word_end;
            current_width = temp_w;
        } else {
            current_width += word_width;
            last_word_end = word_end;
        }
    }

    // Push remaining content
    if line_start < line.len() {
        let remaining_slice = &line[line_start..];
        if !remaining_slice.trim().is_empty() {
            wrapped_lines.push(remaining_slice.trim_end());
        }
    }

    wrapped_lines
}

#[derive(Default)]
pub struct WrappedLine<'a> {
    pub spans: Vec<Span<'a>>,
}

pub fn wrap_spans<'a>(
    content: &'a str,
    width: usize,
    start_style: Option<Style>,
) -> Vec<WrappedLine<'a>> {
    if width == 0 || content.is_empty() {
        return vec![];
    }

    let spans = to_spans(content, start_style);

    let mut lines: Vec<WrappedLine<'a>> = vec![WrappedLine::default()];
    let mut current_width = 0;
    let mut just_wrapped = false;

    for span in spans {
        let style = span.style;
        let text_content: &'a str = span.content;

        let mut char_indices = text_content.char_indices().peekable();

        while let Some((i, c)) = char_indices.next() {
            let char_w = c.width().unwrap_or(0);

            // Handle Whitespace
            if c.is_whitespace() {
                // Skip leading whitespace only after a word wrap, not on the original first line
                if !just_wrapped {
                    if let Some(last_line) = lines.last_mut() {
                        let char_slice = &text_content[i..i + c.len_utf8()];
                        last_line.spans.push(Span::styled(char_slice, style));
                    }
                    current_width += char_w;
                }
                continue;
            }

            // Consume the Word
            let word_start = i;
            let mut word_width = char_w;
            let mut word_end = i + c.len_utf8();

            while let Some(&(_, next_c)) = char_indices.peek() {
                if next_c.is_whitespace() {
                    break;
                }
                word_width += next_c.width().unwrap_or(0);
                word_end += next_c.len_utf8();
                char_indices.next();
            }
            let word_slice = &text_content[word_start..word_end];

            // Overflow Check
            if current_width > 0 && current_width + word_width > width {
                // Before moving to a new line, trim trailing whitespace from the current line
                trim_line_end(lines.last_mut());

                lines.push(WrappedLine::default());
                current_width = 0;
                just_wrapped = true;
            }

            // Force Split or Normal Push
            if word_width > width {
                let mut temp_w = 0;
                let mut temp_start = 0;
                for (ci, cc) in word_slice.char_indices() {
                    let ccw = cc.width().unwrap_or(0);
                    if temp_w + ccw > width && temp_w > 0 {
                        if let Some(last_line) = lines.last_mut() {
                            last_line
                                .spans
                                .push(Span::styled(&word_slice[temp_start..ci], style));
                        }
                        lines.push(WrappedLine::default());
                        just_wrapped = true;
                        temp_start = ci;
                        temp_w = 0;
                    }
                    temp_w += ccw;
                }
                let remaining = &word_slice[temp_start..];
                if !remaining.is_empty() {
                    if let Some(last_line) = lines.last_mut() {
                        last_line.spans.push(Span::styled(remaining, style));
                    }
                    current_width = temp_w;
                    just_wrapped = false;
                }
            } else {
                if let Some(last_line) = lines.last_mut() {
                    last_line.spans.push(Span::styled(word_slice, style));
                }
                current_width += word_width;
                just_wrapped = false;
            }
        }
    }

    // Final cleanup for each line
    for line in &mut lines {
        trim_line_end(Some(line));
    }

    // Remove last line if empty
    if let Some(last) = lines.last()
        && last.spans.is_empty()
        && lines.len() > 1
    {
        lines.pop();
    }

    lines
}

/// Helper to remove trailing whitespace spans from a line
fn trim_line_end(line: Option<&mut WrappedLine<'_>>) {
    if let Some(l) = line {
        while let Some(last_span) = l.spans.last() {
            if last_span.content.chars().all(|c| c.is_whitespace()) {
                l.spans.pop();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_as_str<'b>(lines: &'b [&'_ str]) -> Vec<&'b str> {
        lines.iter().map(|l| l.as_ref()).collect()
    }

    fn span_lines_as_str(lines: &[WrappedLine<'_>]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn test_wrap_spans_multibyte_overflow() {
        // Each '･' is 3 bytes, but width is 1.
        // Total string: "･✧Hi!"
        // If width is 2, it should break after "･✧"
        let input = "･✧Hi!";
        let width = 2;

        // We assume to_spans is working and returns one span for this plain string
        let lines = wrap_spans(input, width, None);

        // Expected:
        // Line 1: "･✧" (Width 2)
        // Line 2: "Hi!" (Width 3 -> will further wrap if width is strictly 2)

        assert!(lines.len() >= 2);

        // Check first line content
        let line_strings = span_lines_as_str(lines.as_slice());

        let first_line_text: Option<&str> = line_strings.first().map(|s| s.as_str());
        assert_eq!(first_line_text, Some("･✧"));
    }

    #[test]
    fn test_wrap_with_style_persistence() {
        // Pink "Hello", then Bold "World"
        // Width 7 forces a wrap after "Hello "
        let input = "\x0313Hello \x02World";
        let width = 7;

        let lines = wrap_spans(input, width, None);

        // Should result in 2 lines:
        // 1. "Hello" (Pink) -> Note: trailing space should be trimmed by your `trim_line_end`
        // 2. "World" (Bold)

        assert_eq!(lines.len(), 2);

        // Verify the second line kept the Bold modifier from the previous span
        let first_line_span = &lines[0].spans[0];
        assert_eq!(first_line_span.content, "Hello");
        let second_line_span = &lines[1].spans[0];
        assert_eq!(second_line_span.content, "World");
        assert!(
            second_line_span
                .style
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD)
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(wrapped_line_count("", 10), 0);
        //assert!(wrap_content("", 10).is_empty());
    }

    #[test]
    fn zero_width() {
        assert_eq!(wrapped_line_count("hello world", 0), 0);
        assert!(wrap_content("hello world", 0).is_empty());
        assert!(wrap_spans("hello world", 0, None).is_empty());
    }

    #[test]
    fn single_short_line() {
        let content = "hello";
        assert_eq!(wrapped_line_count(content, 10), 1);

        let wrapped = wrap_content(content, 10);
        assert_eq!(lines_as_str(&wrapped), vec!["hello"]);

        let wrapped = wrap_spans(content, 10, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["hello"]);
    }

    #[test]
    fn exact_width_fit() {
        let content = "hello world";
        assert_eq!(wrapped_line_count(content, 11), 1);

        let wrapped = wrap_content(content, 11);
        assert_eq!(lines_as_str(&wrapped), vec!["hello world"]);

        let wrapped = wrap_spans(content, 11, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["hello world"]);
    }

    #[test]
    fn simple_wrap() {
        let content = "hello world";
        assert_eq!(wrapped_line_count(content, 5), 2);

        let wrapped = wrap_content(content, 5);
        assert_eq!(lines_as_str(&wrapped), vec!["hello", "world"]);

        let wrapped = wrap_spans(content, 11, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["hello world"]);
    }

    #[test]
    fn multiple_words_wrap() {
        let content = "one two three four";
        assert_eq!(wrapped_line_count(content, 11), 2);

        let wrapped = wrap_content(content, 11);
        assert_eq!(lines_as_str(&wrapped), vec!["one two", "three four"]);

        let wrapped = wrap_spans(content, 11, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["one two", "three four"]);
    }

    #[test]
    fn leading_whitespace_preserved() {
        let content = "  hello world";
        assert_eq!(wrapped_line_count(content, 20), 1);
        let wrapped = wrap_content(content, 20);
        assert_eq!(lines_as_str(&wrapped), vec!["  hello world"]);
        let wrapped = wrap_spans(content, 20, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["  hello world"]);
    }

    #[test]
    fn leading_whitespace_preserved_with_wrap() {
        // "  hello" = 7, "world" = 5 — wraps after hello, new line should NOT strip its leading space
        // "  hello " fits in 8, then " world" would be next but wraps — " world" leading space dropped
        // width=8: "  hello " = 8 wide, "world" pushes to new line (no leading space to worry about)
        // More interesting: leading space on the *input* is preserved on line 1
        let content = "  hi there";
        assert_eq!(wrapped_line_count(content, 6), 2);
        let wrapped = wrap_content(content, 6);
        assert_eq!(lines_as_str(&wrapped), vec!["  hi", "there"]);
        let wrapped = wrap_spans(content, 6, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["  hi", "there"]);
    }

    #[test]
    fn no_leading_whitespace_after_wrap() {
        // After wrapping, the new line should NOT carry over the space that caused the break
        let content = "hello world";
        let wrapped = wrap_content(content, 7);
        assert_eq!(lines_as_str(&wrapped), vec!["hello", "world"]);
        let wrapped = wrap_spans(content, 7, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["hello", "world"]);
    }

    #[test]
    fn multiple_leading_spaces() {
        let content = "    word";
        assert_eq!(wrapped_line_count(content, 10), 1);
        let wrapped = wrap_content(content, 10);
        assert_eq!(lines_as_str(&wrapped), vec!["    word"]);
        let wrapped = wrap_spans(content, 10, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["    word"]);
    }

    #[test]
    fn leading_whitespace_only() {
        // Only whitespace — should produce no lines (or empty, depending on your contract)
        let content = "   ";
        let wrapped = wrap_content(content, 10);
        assert!(wrapped.is_empty() || lines_as_str(&wrapped).iter().all(|l| l.trim().is_empty()));
        let wrapped = wrap_spans(content, 10, None);
        assert!(
            wrapped.is_empty()
                || span_lines_as_str(&wrapped)
                    .iter()
                    .all(|l| l.trim().is_empty())
        );
    }

    #[test]
    fn long_word_is_broken() {
        let content = "abcdefghij"; // len = 10
        assert_eq!(wrapped_line_count(content, 4), 3);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["abcd", "efgh", "ij"]);

        let wrapped = wrap_spans(content, 4, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn long_url_is_broken() {
        let content = "https://github.com/F4r3n/clown/"; // len = 10
        assert_eq!(wrapped_line_count(content, 10), 4);

        let wrapped = wrap_content(content, 10);
        assert_eq!(
            lines_as_str(&wrapped),
            vec!["https://gi", "thub.com/F", "4r3n/clown", "/"]
        );

        let wrapped = wrap_spans(content, 10, None);
        assert_eq!(
            span_lines_as_str(&wrapped),
            vec!["https://gi", "thub.com/F", "4r3n/clown", "/"]
        );
    }

    #[test]
    fn long_word_after_short_word() {
        let content = "hi abcdef";
        assert_eq!(wrapped_line_count(content, 4), 3);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["hi", "abcd", "ef"]);

        let wrapped = wrap_spans(content, 4, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["hi", "abcd", "ef"]);
    }

    #[test]
    fn unicode_cjk_width() {
        // Each 中 is width 2
        let content = "中中中中"; // total width = 8
        assert_eq!(wrapped_line_count(content, 4), 2);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["中中", "中中"]);

        let wrapped = wrap_spans(content, 4, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["中中", "中中"]);
    }

    #[test]
    fn emoji_width_handling() {
        // 😀 is width 2 in most terminals
        let content = "😀😀😀";
        assert_eq!(wrapped_line_count(content, 4), 2);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["😀😀", "😀"]);

        let wrapped = wrap_spans(content, 4, None);
        assert_eq!(span_lines_as_str(&wrapped), vec!["😀😀", "😀"]);
    }

    #[test]
    fn count_matches_wrap_len() {
        let cases = [
            ("hello world", 5),
            ("one two three four", 7),
            ("abcdefghij", 3),
            ("中中中中中", 4),
        ];

        for (content, width) in cases {
            let count = wrapped_line_count(content, width);
            let wrapped = wrap_content(content, width);
            assert_eq!(
                count,
                wrapped.len(),
                "count mismatch for content={:?}, width={}",
                content,
                width
            );
        }
    }
}
