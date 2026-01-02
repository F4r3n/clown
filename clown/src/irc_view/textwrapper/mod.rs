use crate::irc_view::message_parser::to_raw;
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

pub fn wrapped_line_count(content: &str, width: usize) -> usize {
    if width == 0 {
        return 0;
    }

    let mut lines = 1;
    let mut col = 0;

    for raw_segment in to_raw(content) {
        // Use UnicodeSegmentation to split each segment into word and non-word chunks.
        for word_chunk in raw_segment.split_word_bounds() {
            if word_chunk.is_empty() || word_chunk.chars().all(|c| c.is_whitespace()) {
                continue;
            }

            let chunk_width: usize = word_chunk
                .chars()
                .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
                .sum();

            // Width for the potential preceding space, if the current line is not empty
            let space_width = if col > 0 { 1 } else { 0 };

            if col + space_width + chunk_width <= width {
                // It fits!
                col += space_width + chunk_width;
            } else {
                // It does not fit. Start a new line.
                lines += 1;

                // Check if the word needs to be broken across multiple lines
                if chunk_width > width {
                    // Add lines for the subsequent breaks within the long word
                    lines += (chunk_width - 1) / width;

                    // Set the column to the width of the last segment (corrected logic)
                    col = (chunk_width - 1) % width + 1;
                } else {
                    // The word fits on its own new line (no break needed)
                    col = chunk_width;
                }
            }
        }
    }

    lines
}

pub fn wrap_content(content: &str, width: usize) -> Vec<Cow<'_, str>> {
    if width == 0 {
        return vec![];
    }

    let mut wrapped_lines: Vec<String> = vec![String::new()];
    let mut current_line_idx = 0;
    let mut col = 0;

    for raw_segment in to_raw(content) {
        // Use UnicodeSegmentation to split each segment into word and non-word chunks.
        for word_chunk in raw_segment.split_word_bounds() {
            if word_chunk.is_empty() || word_chunk.chars().all(|c| c.is_whitespace()) {
                continue;
            }

            let chunk_width: usize = word_chunk
                .chars()
                .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
                .sum();

            // --- Handle a non-whitespace word chunk ---

            let space_width = if col > 0 { 1 } else { 0 };

            if col + space_width + chunk_width <= width {
                // It fits!
                if col > 0 {
                    wrapped_lines[current_line_idx].push(' ');
                    col += 1;
                }
                wrapped_lines[current_line_idx].push_str(word_chunk);
                col += chunk_width;
            } else {
                // It does not fit. Start a new line.
                wrapped_lines.push(String::new());
                current_line_idx += 1;

                // 2. Break the long word if it's wider than the line (char-by-char split)
                if chunk_width > width {
                    let mut temp_word_col = 0;

                    // Use graphemes for safe display splitting
                    for w_grapheme in word_chunk.graphemes(true) {
                        let w_grapheme_w: usize = w_grapheme
                            .chars()
                            .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
                            .sum();

                        if temp_word_col + w_grapheme_w > width {
                            wrapped_lines.push(String::new());
                            current_line_idx += 1;
                            temp_word_col = 0;
                        }

                        wrapped_lines[current_line_idx].push_str(w_grapheme);
                        temp_word_col += w_grapheme_w;
                    }
                    col = temp_word_col;
                } else {
                    // The word fits on its own new line (no break needed)
                    wrapped_lines[current_line_idx].push_str(word_chunk);
                    col = chunk_width;
                }
            }
        }
    }

    wrapped_lines.into_iter().map(Cow::Owned).collect()
}
