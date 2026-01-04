use std::borrow::Cow;
use unicode_width::UnicodeWidthChar;

pub fn wrapped_line_count(content: &str, width: usize) -> usize {
    if width == 0 || content.is_empty() {
        return 0;
    }

    let mut total_lines = 0;

    // Preserve existing paragraphs/newlines first
    let line = content;

    let line_trimmed_start = line.trim_start();
    let start_offset = line.len() - line_trimmed_start.len();

    let mut current_width = 0;
    let mut has_content_on_current_line = false;

    // Start the line count if there is any non-whitespace content on this line
    if total_lines == 0 && !line_trimmed_start.is_empty() {
        total_lines = 1;
    }

    // Start iterator past the leading spaces
    let mut char_indices = line.char_indices().skip(start_offset).peekable();
    // ------------------------------------------

    while let Some((_i, c)) = char_indices.next() {
        let char_width = c.width().unwrap_or(0);

        // 1. Check if we are at a breakable point (space)
        if c.is_whitespace() {
            // Accumulate space width only if a word preceded it
            if has_content_on_current_line {
                current_width += char_width;
            }
            continue;
        }

        // --- We are inside a word ---
        let mut word_width = char_width;

        // Look ahead to find the full word width (consuming chars from iterator)
        while let Some((_, next_c)) = char_indices.peek() {
            if next_c.is_whitespace() {
                break;
            }
            let next_w = next_c.width().unwrap_or(0);
            word_width += next_w;
            char_indices.next(); // Consume the char
        }

        // 2. Logic: Does this word fit on the current line?
        if current_width > 0 && current_width + word_width > width {
            // WRAP: The new word overflows.
            total_lines += 1;
            current_width = 0;
        }

        // 3. Handle Long Words (Word is wider than width)
        if word_width > width {
            // Calculate how many *additional* lines this word consumes after the current line.
            // The current line is already accounted for (or was just incremented in step 2).

            let distinct_lines = word_width.div_ceil(width);
            if distinct_lines > 0 {
                total_lines += distinct_lines - 1;
            }

            // Set the column to the width of the final segment on the last line.
            current_width = word_width % width;
            if current_width == 0 {
                current_width = width;
            }

            has_content_on_current_line = true;
        } else {
            // Word fits (either on the existing line or a newly wrapped line)
            current_width += word_width;
            has_content_on_current_line = true;
        }
    }

    total_lines
}

pub fn wrap_content<'a>(content: &'a str, width: usize) -> Vec<Cow<'a, str>> {
    if width == 0 {
        return vec![];
    }

    let mut wrapped_lines = Vec::new();

    // Preserve existing paragraphs/newlines first
    let line = content;

    // --- FIX: Skip initial whitespace segment ---
    let line_trimmed_start = line.trim_start();
    let start_offset = line.len() - line_trimmed_start.len();

    let mut line_start = start_offset;
    let mut current_width = 0;
    let mut last_word_end = start_offset;

    // Start iterator past the leading spaces
    let mut char_indices = line.char_indices().skip(start_offset).peekable();
    // ------------------------------------------

    while let Some((i, c)) = char_indices.next() {
        let char_width = c.width().unwrap_or(0);

        // 1. Check if we are at a breakable point (space)
        if c.is_whitespace() {
            // If the word we just finished fits, we update the "safe" split point
            last_word_end = i;
            current_width += char_width;
            continue;
        }

        // We are inside a word. Let's see how long this word is.
        let word_start = i;
        let mut word_width = char_width;
        let mut word_end = i + c.len_utf8();

        // Look ahead to finish the word (consuming chars from iterator)
        while let Some((_next_i, next_c)) = char_indices.peek() {
            if next_c.is_whitespace() {
                break;
            }
            let next_w = next_c.width().unwrap_or(0);
            word_width += next_w;
            word_end += next_c.len_utf8();
            char_indices.next(); // Consume the char
        }

        // 2. Logic: Does this word fit on the current line?
        // Check includes the accumulated space width (current_width)
        if current_width > 0 && current_width + word_width > width {
            // WRAP: The new word overflows.

            // Push the previous valid content up to the last word end.
            // We trim existing trailing whitespace from the slice for cleanliness.
            let line_slice = &line[line_start..last_word_end];
            wrapped_lines.push(Cow::Borrowed(line_slice.trim_end()));

            // Reset for new line
            line_start = word_start;
            current_width = 0;
        }

        // 3. Handle Long Words (Word itself is wider than width)
        if word_width > width {
            // If the word alone is too big, we must split it by graphemes/chars

            // If there was any prior content on the current line, push it before splitting the long word
            if current_width > 0 {
                wrapped_lines.push(Cow::Borrowed(line[line_start..word_start].trim_end()));
            }

            // Force split the long word and push segments
            let mut temp_w = 0;
            let mut temp_start = word_start;

            // Re-scan the word char by char to find split points
            for (ci, cc) in line[word_start..word_end].char_indices() {
                let cw = cc.width().unwrap_or(0);
                if temp_w + cw > width {
                    wrapped_lines.push(Cow::Borrowed(&line[temp_start..word_start + ci]));
                    temp_start = word_start + ci;
                    temp_w = 0;
                }
                temp_w += cw;
            }

            // Set up for the remainder of the word
            line_start = temp_start;
            last_word_end = word_end;
            current_width = temp_w;
        } else {
            // Word fits (either on new line or existing line)
            current_width += word_width;
            last_word_end = word_end;
        }
    }

    // Push whatever is left in the buffer (the content of the last line)
    // Only push if there is non-whitespace content.
    if line_start < line.len() {
        let remaining_slice = &line[line_start..];
        if !remaining_slice.trim().is_empty() {
            wrapped_lines.push(Cow::Borrowed(remaining_slice.trim_end()));
        }
    }

    wrapped_lines
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
            vec!["https://gi", "thub.com/F", "4r3n/clown", "/"]
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
