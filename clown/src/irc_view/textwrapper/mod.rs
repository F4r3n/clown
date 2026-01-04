use crate::irc_view::message_parser::to_raw;
use std::borrow::Cow;
use std::mem;
use unicode_linebreak::linebreaks;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Debug)]
enum Separator {
    SPACE,
    UNICODE,
}

pub fn wrapped_line_count(content: &str, width: usize) -> usize {
    if width == 0 || content.is_empty() {
        return 0;
    }

    let mut lines = 1;
    let mut col = 0;
    // Tracks if the last thing placed was a word (needed to determine if a space is required).
    let mut last_was_word = false;

    for raw_segment in to_raw(content) {
        let mut start_i = 0;

        for (break_pos, _) in linebreaks(raw_segment) {
            let chunk = &raw_segment[start_i..break_pos];
            //println!("chunk '{}'", chunk);
            start_i = break_pos;
            let separator = match chunk.chars().last() {
                Some(' ') => Separator::SPACE,
                _ => Separator::UNICODE,
            };
            // Separate the word from surrounding whitespace/delimiters
            let word_part = chunk.trim(); //word break gives spaces too
            let word_width = UnicodeWidthStr::width(word_part);

            if word_width == 0 {
                // If it's a zero-width or whitespace-only chunk, skip it.
                continue;
            }

            // Space is needed if we aren't at col 0, the last thing was a word, and it's not CJK.
            let space_needed = last_was_word && col > 0 && separator == Separator::SPACE;
            let space_width = if space_needed { 1 } else { 0 };

            if col + space_width + word_width <= width {
                //Word fits on current line (with separator space).

                if space_needed {
                    col += 1; // Account for the space injected by wrap_content
                }

                col += word_width;
                last_was_word = true;
            } else {
                //Word does not fit. Must wrap.

                // 1. Force move to the next line ONLY if we already placed content (col > 0).
                if col > 0 {
                    lines += 1;
                }

                // --- Start New Line with Word ---

                if word_width <= width {
                    // Case B.1: The word part fits entirely on the new line.
                    col = word_width;
                    last_was_word = true;
                } else {
                    // Long word breaking.

                    // The first break of a massive word will be handled by the 'col > 0' check above.
                    // Now, calculate how many *additional* lines this word consumes.

                    // Use ceiling division to find total number of lines required for the word itself.
                    let distinct_lines = word_width.div_ceil(width);
                    lines += distinct_lines - 1; // Add only the additional lines

                    // Set the column to the width of the final segment on the last line.
                    col = word_width % width;
                    if col == 0 {
                        col = width;
                    }
                    last_was_word = true;
                }
            }
        }
    }

    lines
}

pub fn wrap_content<'a>(content: &'a str, width: usize) -> Vec<Cow<'a, str>> {
    if width == 0 {
        return vec![];
    }

    // Check if the entire content (without internal linebreaks/wraps) fits.
    let total_width = UnicodeWidthStr::width(content);
    if total_width <= width {
        return vec![Cow::Borrowed(content)];
    }

    let mut wrapped_lines: Vec<Cow<'a, str>> = Vec::new();
    let mut current_line = String::new();
    let mut col = 0;

    // This tracks if the content just appended was a word, not a space.
    let mut last_was_word = false;

    for raw_segment in to_raw(content) {
        let mut start_i = 0;
        let mut separator = Separator::UNICODE;
        for (break_pos, _) in linebreaks(raw_segment) {
            let chunk = &raw_segment[start_i..break_pos];
            start_i = break_pos;
            if !last_was_word {
                separator = match chunk.chars().last() {
                    Some(' ') => Separator::SPACE,
                    _ => Separator::UNICODE,
                };
            }

            // Separate the word from its potential trailing space/delimiter
            let word_part = chunk.trim_end();

            let word_width = UnicodeWidthStr::width(word_part);

            if word_width == 0 {
                if !chunk.trim().is_empty() {
                    // Skip zero-width chars, but not visible spaces (though they should be handled by linebreaks)
                    continue;
                }
                // If it's pure whitespace and col > 0, it means a space chunk forced a wrap. Discard and continue.
                if col > 0 && chunk.trim().is_empty() {
                    continue;
                }
            }

            let space_needed = last_was_word && col > 0 && (separator == Separator::SPACE);
            let space_width = if space_needed { 1 } else { 0 };

            if col + space_width + word_width <= width {
                // Word fits on current line (with separator space).

                if space_needed {
                    current_line.push(' ');
                    col += 1;
                }

                // Append only the word part, discarding any trailing delimiter
                current_line.push_str(word_part);
                col += word_width;
                last_was_word = true;
            } else {
                // Word does not fit. Must wrap.

                // 1. Finalize and push the current line (if it has content)
                if !current_line.is_empty() {
                    wrapped_lines.push(Cow::Owned(mem::take(&mut current_line)));
                    col = 0;
                }

                // 2. New Line: Discard the word's delimiter and push the word part.

                if word_width <= width {
                    // The word part fits entirely on the new line.
                    current_line.push_str(word_part);
                    col = word_width;
                    last_was_word = true;
                } else {
                    //  Long word breaking.

                    for grapheme in word_part.graphemes(true) {
                        let grapheme_w = UnicodeWidthStr::width(grapheme);

                        if col + grapheme_w > width {
                            wrapped_lines.push(Cow::Owned(mem::take(&mut current_line)));
                            col = 0;
                        }

                        current_line.push_str(grapheme);
                        col += grapheme_w;
                        last_was_word = true;
                    }
                }
            }
        }
    }

    // Final push (correctly using mem::take)
    if !current_line.is_empty() {
        wrapped_lines.push(Cow::Owned(mem::take(&mut current_line)));
    }

    // Final cleanup
    if wrapped_lines.len() == 1 && wrapped_lines.first().is_some_and(|v| v.is_empty()) {
        vec![]
    } else {
        wrapped_lines
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    fn lines_as_str<'b>(lines: &'b [Cow<'_, str>]) -> Vec<&'b str> {
        lines.iter().map(|l| l.as_ref()).collect()
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
    }

    #[test]
    fn single_short_line() {
        let content = "hello";
        assert_eq!(wrapped_line_count(content, 10), 1);

        let wrapped = wrap_content(content, 10);
        assert_eq!(lines_as_str(&wrapped), vec!["hello"]);
    }

    #[test]
    fn exact_width_fit() {
        let content = "hello world";
        assert_eq!(wrapped_line_count(content, 11), 1);

        let wrapped = wrap_content(content, 11);
        assert_eq!(lines_as_str(&wrapped), vec!["hello world"]);
    }

    #[test]
    fn simple_wrap() {
        let content = "hello world";
        assert_eq!(wrapped_line_count(content, 5), 2);

        let wrapped = wrap_content(content, 5);
        assert_eq!(lines_as_str(&wrapped), vec!["hello", "world"]);
    }

    #[test]
    fn multiple_words_wrap() {
        let content = "one two three four";
        assert_eq!(wrapped_line_count(content, 11), 2);

        let wrapped = wrap_content(content, 11);
        assert_eq!(lines_as_str(&wrapped), vec!["one two", "three four"]);
    }

    #[test]
    fn ignores_extra_whitespace() {
        let content = "  hello   world  ";
        assert_eq!(wrapped_line_count(content, 5), 2);

        let wrapped = wrap_content(content, 5);
        assert_eq!(lines_as_str(&wrapped), vec!["hello", "world"]);
    }

    #[test]
    fn long_word_is_broken() {
        let content = "abcdefghij"; // len = 10
        assert_eq!(wrapped_line_count(content, 4), 3);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn long_url_is_broken() {
        let content = "https://github.com/F4r3n/clown/"; // len = 10
        assert_eq!(wrapped_line_count(content, 10), 4);

        let wrapped = wrap_content(content, 10);
        assert_eq!(
            lines_as_str(&wrapped),
            vec!["https://", "github.com", "/F4r3n/", "clown/"]
        );
    }

    #[test]
    fn long_word_after_short_word() {
        let content = "hi abcdef";
        assert_eq!(wrapped_line_count(content, 4), 3);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["hi", "abcd", "ef"]);
    }

    #[test]
    fn unicode_cjk_width() {
        // Each 中 is width 2
        let content = "中中中中"; // total width = 8
        assert_eq!(wrapped_line_count(content, 4), 2);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["中中", "中中"]);
    }

    #[test]
    fn emoji_width_handling() {
        // 😀 is width 2 in most terminals
        let content = "😀😀😀";
        assert_eq!(wrapped_line_count(content, 4), 2);

        let wrapped = wrap_content(content, 4);
        assert_eq!(lines_as_str(&wrapped), vec!["😀😀", "😀"]);
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

    #[test]
    fn borrowed_vs_owned_smoke_test() {
        let content = "hello world";
        let wrapped = wrap_content(content, 20);

        assert_eq!(wrapped.len(), 1);
        match &wrapped[0] {
            Cow::Borrowed(s) => assert_eq!(*s, "hello world"),
            Cow::Owned(_) => panic!("expected borrowed Cow"),
        }
    }
}
