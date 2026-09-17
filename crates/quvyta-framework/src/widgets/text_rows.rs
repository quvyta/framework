//! Word-wrapped rows of editable text, and moving between byte offsets and row cells.
//!
//! Unlike [`crate::text::wrap_ranges`], which drops the spaces at a break for display, these
//! rows cover every byte of the text so every cursor position belongs to a row: spaces hang at
//! the end of the row they follow, and a row ends at its line break.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::text;

/// Byte ranges of the rows `text` wraps into at `width` cells. Rows of one line touch; a line
/// break sits between the last row of its line and the first row of the next. Empty text has
/// one empty row.
pub(crate) fn wrap(text: &str, width: u16) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut line_start = 0;
    for line in text.split('\n') {
        let mut row_start = line_start;
        let mut row_width = 0u16;
        for (offset, word) in line.split_word_bound_indices() {
            let start = line_start + offset;
            let word_width = text::width(word);
            if word.chars().all(char::is_whitespace) {
                row_width = row_width.saturating_add(word_width);
                continue;
            }
            if row_width.saturating_add(word_width) <= width {
                row_width += word_width;
                continue;
            }
            if row_width > 0 && word_width <= width {
                rows.push(row_start..start);
                row_start = start;
                row_width = word_width;
                continue;
            }
            // A word longer than a row breaks between graphemes.
            for (g_offset, grapheme) in word.grapheme_indices(true) {
                let g_width = text::grapheme_width(grapheme);
                if row_width > 0 && row_width.saturating_add(g_width) > width {
                    rows.push(row_start..start + g_offset);
                    row_start = start + g_offset;
                    row_width = 0;
                }
                row_width = row_width.saturating_add(g_width);
            }
        }
        rows.push(row_start..line_start + line.len());
        line_start += line.len() + 1;
    }
    rows
}

/// Whether `row` ends its line, so the cursor may sit after its last grapheme.
pub(crate) fn ends_line(text: &str, row: &Range<usize>) -> bool {
    row.end == text.len() || text.as_bytes()[row.end] == b'\n'
}

/// Whether `row` starts a line, where a line number belongs.
pub(crate) fn starts_line(text: &str, row: &Range<usize>) -> bool {
    row.start == 0 || text.as_bytes()[row.start - 1] == b'\n'
}

/// The row and cell column of byte `offset`.
pub(crate) fn locate(text: &str, rows: &[Range<usize>], offset: usize) -> (usize, u16) {
    let row = rows.iter().rposition(|row| row.start <= offset).unwrap_or(0);
    let start = rows.get(row).map_or(0, |row| row.start);
    (row, text::width(&text[start..offset.clamp(start, text.len())]))
}

/// The byte offset in `row` closest to cell `column` without passing it. On a row that wraps
/// onto the next, the offset stays before the row's end, which belongs to the next row.
pub(crate) fn offset_at(text: &str, rows: &[Range<usize>], row: usize, column: u16) -> usize {
    let Some(range) = rows.get(row) else {
        return text.len();
    };
    let mut offset = range.start;
    let mut cells = 0u16;
    for (index, grapheme) in text[range.clone()].grapheme_indices(true) {
        let g_width = text::grapheme_width(grapheme).max(1);
        let next = range.start + index + grapheme.len();
        if cells.saturating_add(g_width) > column || (next == range.end && !ends_line(text, range)) {
            break;
        }
        cells += g_width;
        offset = next;
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown<'t>(text: &'t str, rows: &[Range<usize>]) -> Vec<&'t str> {
        rows.iter().map(|row| &text[row.clone()]).collect()
    }

    #[test]
    fn rows_cover_every_byte_and_keep_indentation() {
        let text = "deploy the canary\n  then wait\n\nsupercalifragilistic";
        let rows = wrap(text, 8);
        assert_eq!(
            shown(text, &rows),
            ["deploy ", "the ", "canary", "  then ", "wait", "", "supercal", "ifragili", "stic"]
        );
        assert!(ends_line(text, &rows[2]) && !ends_line(text, &rows[1]));
        assert!(starts_line(text, &rows[3]) && !starts_line(text, &rows[4]));
        assert_eq!(wrap("", 5).len(), 1);
        assert!(wrap("", 5)[0].is_empty());
    }

    #[test]
    fn offsets_and_cells_round_trip() {
        let text = "çay demlendi\nbitti";
        let rows = wrap(text, 9);
        assert_eq!(shown(text, &rows), ["çay ", "demlendi", "bitti"]);
        // The start of a wrapped row belongs to that row, a line end to its own row.
        assert_eq!(locate(text, &rows, rows[1].start), (1, 0));
        assert_eq!(locate(text, &rows, rows[1].end), (1, 8));
        assert_eq!(offset_at(text, &rows, 0, 10), rows[0].end - 1, "stays before the wrap");
        assert_eq!(offset_at(text, &rows, 2, 3), rows[2].start + 3);
        assert_eq!(offset_at(text, &rows, 1, 99), rows[1].end, "a line end is reachable");
        assert_eq!(offset_at(text, &rows, 9, 0), text.len());
    }
}
