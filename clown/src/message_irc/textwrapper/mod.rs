use crate::message_irc::message_parser::to_spans;
use ratatui::style::Style;
use ratatui::text::Span;
use std::ops::Range;
use unicode_width::UnicodeWidthChar;

fn wrap_ranges(content: &str, width: usize, mut emit: impl FnMut(Range<usize>)) {
    if width == 0 || content.is_empty() {
        return;
    }

    // `line_end` trails the last non-whitespace char so trailing ws is trimmed.
    let mut line_start = 0;
    let mut line_end = 0;
    let mut current_width = 0;
    let mut line_count = 0;
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        let char_width = c.width().unwrap_or(0);

        if c.is_whitespace() {
            current_width += char_width;
            continue;
        }

        let word_start = i;
        let mut word_end = i + c.len_utf8();
        let mut word_width = char_width;
        while let Some(&(_, next_c)) = chars.peek() {
            if next_c.is_whitespace() {
                break;
            }
            word_width += next_c.width().unwrap_or(0);
            word_end += next_c.len_utf8();
            chars.next();
        }

        if current_width > 0 && current_width + word_width > width {
            emit(line_start..line_end);
            line_count += 1;
            line_start = word_start;
            current_width = 0;
        }

        if word_width > width {
            // Word wider than a line: force-split at char boundaries.
            let mut seg_width = 0;
            for (ci, cc) in content[word_start..word_end].char_indices() {
                let cc_width = cc.width().unwrap_or(0);
                let abs = word_start + ci;
                if seg_width > 0 && seg_width + cc_width > width {
                    emit(line_start..abs);
                    line_count += 1;
                    line_start = abs;
                    seg_width = 0;
                }
                seg_width += cc_width;
            }
            current_width = seg_width;
            line_end = word_end;
        } else {
            current_width += word_width;
            line_end = word_end;
        }
    }

    // Drop an empty final line unless it is the only one (so a whitespace-only
    // message still counts as one row).
    if line_count == 0 || line_end > line_start {
        emit(line_start..line_end);
    }
}

/// Rows `content` (already stripped of IRC formatting) wraps to. Allocation-free.
pub fn wrapped_line_count(content: &str, width: usize) -> usize {
    let mut count = 0;
    wrap_ranges(content, width, |_| count += 1);
    count
}

/// Wrap plain (already stripped) text into borrowed line slices.
pub fn wrap_content(content: &str, width: usize) -> Vec<&str> {
    let mut lines = Vec::new();
    wrap_ranges(content, width, |r| lines.push(&content[r]));
    lines
}

#[derive(Default)]
pub struct WrappedLine<'a> {
    pub spans: Vec<Span<'a>>,
}

/// Wrap IRC-formatted `content` into styled lines. Break decisions run on the
/// visible text (concatenated span contents, byte-identical to what
/// [`wrapped_line_count`] sees), then styling is overlaid per range — so the
/// line count always matches `wrapped_line_count`.
pub fn wrap_spans<'a>(
    content: &'a str,
    width: usize,
    start_style: Option<Style>,
) -> Vec<WrappedLine<'a>> {
    if width == 0 || content.is_empty() {
        return vec![];
    }

    let spans = to_spans(content, start_style);

    // Borrow the visible text for a single span, else concatenate once.
    let visible: std::borrow::Cow<'a, str> = match spans.as_slice() {
        [single] => std::borrow::Cow::Borrowed(single.content),
        _ => {
            let mut s = String::with_capacity(content.len());
            for span in &spans {
                s.push_str(span.content);
            }
            std::borrow::Cow::Owned(s)
        }
    };

    let mut span_starts = Vec::with_capacity(spans.len());
    let mut acc = 0;
    for span in &spans {
        span_starts.push(acc);
        acc += span.content.len();
    }

    let mut lines = Vec::new();
    wrap_ranges(&visible, width, |range| {
        let mut line = WrappedLine::default();
        for (span, &span_start) in spans.iter().zip(&span_starts) {
            let span_end = span_start + span.content.len();
            let start = range.start.max(span_start);
            let end = range.end.min(span_end);
            if start < end {
                line.spans.push(Span::styled(
                    &span.content[start - span_start..end - span_start],
                    span.style,
                ));
            }
        }
        lines.push(line);
    });
    lines
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

    /// The invariant the scroll math depends on: the row count used to position
    /// the viewport must equal the number of rows actually rendered. Counting
    /// runs on the stripped text; rendering runs on the formatted text — they
    /// must agree across plain, multibyte, long-word and IRC-formatted inputs.
    #[test]
    fn count_matches_wrap_spans_len() {
        use crate::message_irc::message_parser::strip_irc_formatting_cow;

        let inputs = [
            "hello world",
            "one two three four",
            "abcdefghij",
            "中中中中中",
            "😀😀😀 hi",
            "   ",
            "  leading spaces then text",
            "https://github.com/F4r3n/clown/ is a long url",
            "\x0313Hello \x02World",
            "plain \x034red\x03 text \x02bold\x02 mixed formatting here",
            "no\x02space\x02between\x02styled\x02runs",
            "中\x02中中\x02中中",
        ];

        for input in inputs {
            for width in 1..=20 {
                let stripped = strip_irc_formatting_cow(input);
                let count = wrapped_line_count(&stripped, width);
                let rendered = wrap_spans(input, width, None).len();
                assert_eq!(
                    count, rendered,
                    "count ({count}) != wrap_spans len ({rendered}) for input={input:?}, width={width}",
                );
            }
        }
    }
}
