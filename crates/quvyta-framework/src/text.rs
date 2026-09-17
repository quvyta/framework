//! Measuring text in terminal cells: width, truncation with an ellipsis, and word wrapping.

use std::borrow::Cow;
use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The ellipsis drawn where text is cut.
pub const ELLIPSIS: &str = "…";

/// Display width of `text` in cells.
#[must_use]
pub fn width(text: &str) -> u16 {
    let cells = if is_printable_ascii(text) { text.len() } else { text.width() };
    u16::try_from(cells).unwrap_or(u16::MAX)
}

/// Display width of one grapheme cluster.
#[must_use]
pub fn grapheme_width(grapheme: &str) -> u16 {
    width(grapheme)
}

/// Whether every character of `text` is printable ASCII, one cell and one grapheme cluster each.
/// Most text a terminal application draws is, and it needs no Unicode tables to measure or split.
pub(crate) fn is_printable_ascii(text: &str) -> bool {
    text.bytes().all(|byte| matches!(byte, b' '..=b'~'))
}

/// `text` cut to at most `max` cells, ending in `…` when anything was removed.
#[must_use]
pub fn truncate(text: &str, max: u16) -> Cow<'_, str> {
    if width(text) <= max {
        return Cow::Borrowed(text);
    }
    if max == 0 {
        return Cow::Borrowed("");
    }
    let budget = max - 1;
    let mut used = 0u16;
    let mut out = String::new();
    for grapheme in text.graphemes(true) {
        let w = grapheme_width(grapheme);
        if used + w > budget {
            break;
        }
        used += w;
        out.push_str(grapheme);
    }
    out.push_str(ELLIPSIS);
    Cow::Owned(out)
}

/// Splits `text` into lines no wider than `max` cells.
///
/// Explicit newlines are kept, words move to the next line whole when they fit on it (with
/// the punctuation attached to them, so a comma never starts a line), and words longer than a
/// line are broken between grapheme clusters; the punctuation closing such a word breaks off
/// with the character before it, so `.` or `)` never starts a line alone either. Spaces at a
/// break are dropped.
#[must_use]
pub fn wrap(text: &str, max: u16) -> Vec<String> {
    wrap_ranges(text, max).into_iter().map(|range| text[range].to_owned()).collect()
}

/// Like [`wrap`], but returns byte ranges into `text`, so styled text can be wrapped and drawn
/// with its styles.
#[must_use]
pub fn wrap_ranges(text: &str, max: u16) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    if max == 0 {
        return lines;
    }
    let mut paragraph_start = 0;
    for paragraph in text.split('\n') {
        let mut line: Option<Range<usize>> = None;
        let mut line_width = 0u16;
        for (offset, word, is_space) in runs(paragraph) {
            let start = paragraph_start + offset;
            let end = start + word.len();
            let word_width = width(word);
            if is_space {
                match &mut line {
                    Some(current) if line_width + word_width <= max => {
                        current.end = end;
                        line_width += word_width;
                    }
                    Some(_) => {
                        lines.push(trim_end(text, line.take()));
                        line_width = 0;
                    }
                    None => {}
                }
                continue;
            }
            if line_width + word_width <= max {
                line = Some(line.map_or(start..end, |current| current.start..end));
                line_width += word_width;
                continue;
            }
            if line.is_some() && word_width <= max {
                lines.push(trim_end(text, line.take()));
                line = Some(start..end);
                line_width = word_width;
                continue;
            }
            let graphemes: Vec<(usize, &str)> = word.grapheme_indices(true).collect();
            // The punctuation closing the word never starts a line alone: it breaks off together
            // with the grapheme before it, as in `ui.add(Badge::new("Paused"))` + `.`.
            let tail = graphemes.iter().rposition(|(_, g)| !is_closing_punctuation(g)).unwrap_or(0);
            let tail_width: u16 = graphemes[tail..].iter().map(|(_, g)| grapheme_width(g)).sum();
            for (index, (g_offset, grapheme)) in graphemes.iter().enumerate() {
                let g_start = start + g_offset;
                let g_end = g_start + grapheme.len();
                let w = grapheme_width(grapheme);
                let needed = if index == tail && tail_width <= max { tail_width } else { w };
                if line_width + needed > max && line.is_some() {
                    lines.push(trim_end(text, line.take()));
                    line_width = 0;
                }
                line = Some(line.map_or(g_start..g_end, |current| current.start..g_end));
                line_width += w;
            }
        }
        lines.push(line.map_or(paragraph_start..paragraph_start, |current| trim_end(text, Some(current))));
        paragraph_start += paragraph.len() + 1;
    }
    lines
}

/// The runs of whitespace and of everything between them, with their byte offsets and whether
/// they are whitespace: a word keeps its punctuation (`boundaries,`, `2026.9.1`) and moves to the
/// next line as one.
fn runs(paragraph: &str) -> impl Iterator<Item = (usize, &str, bool)> {
    let mut position = 0;
    std::iter::from_fn(move || {
        let start = position;
        let (space, first) = char_at(paragraph, start)?;
        position += first;
        while let Some((_, len)) = char_at(paragraph, position).filter(|&(next, _)| next == space) {
            position += len;
        }
        Some((start, &paragraph[start..position], space))
    })
}

/// Whether the character starting at byte `index` of `text` is whitespace, and its length in
/// bytes; `None` past the end. Wrapping looks at every character of a text, and ASCII, most of
/// what is wrapped, needs no decoding.
fn char_at(text: &str, index: usize) -> Option<(bool, usize)> {
    let byte = *text.as_bytes().get(index)?;
    if byte.is_ascii() {
        return Some((char::from(byte).is_whitespace(), 1));
    }
    text.get(index..)?.chars().next().map(|c| (c.is_whitespace(), c.len_utf8()))
}

/// Whether `grapheme` is punctuation that closes what comes before it and must not start a line.
fn is_closing_punctuation(grapheme: &str) -> bool {
    grapheme
        .chars()
        .all(|c| matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '"' | '\'' | '…' | '’' | '”' | '»'))
}

fn trim_end(text: &str, range: Option<Range<usize>>) -> Range<usize> {
    let range = range.unwrap_or(0..0);
    let trimmed = text[range.clone()].trim_end();
    range.start..range.start + trimmed.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_wide_and_combining_text() {
        assert_eq!(width("abc"), 3);
        assert_eq!(width("çığ"), 3);
        assert_eq!(width("界"), 2);
        assert_eq!(width("e\u{301}"), 1);
    }

    #[test]
    fn truncates_with_ellipsis_by_cells() {
        assert_eq!(truncate("quvyta", 10), "quvyta");
        assert_eq!(truncate("quvyta-framework", 8), "quvyta-…");
        assert_eq!(truncate("界界界", 4), "界…");
        assert_eq!(truncate("abc", 0), "");
        assert_eq!(width(&truncate("quvyta-framework", 8)), 8);
    }

    #[test]
    fn wraps_words_and_breaks_long_ones() {
        assert_eq!(wrap("the quick brown fox", 9), vec!["the quick", "brown fox"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap("a\n\nb", 5), vec!["a", "", "b"]);
        assert_eq!(wrap("one  two", 4), vec!["one", "two"]);
        assert_eq!(wrap("at word boundaries, never", 18), vec!["at word", "boundaries, never"], "a comma stays");
        assert_eq!(wrap("deploy 2026.9.1 done", 12), vec!["deploy", "2026.9.1", "done"]);
        assert_eq!(wrap("abcdefgh.", 8), vec!["abcdefg", "h."], "a broken word keeps its full stop company");
        assert_eq!(wrap("add abcdefghijk),", 8), vec!["add abcd", "efghij", "k),"]);
        assert_eq!(wrap("abcdefghijk.", 4), vec!["abcd", "efgh", "ijk."]);
        assert_eq!(wrap("........", 4), vec!["....", "...."], "all punctuation still breaks");
        assert!(wrap("x", 0).is_empty());
    }

    /// Pins wrapping, truncation and width of text the ASCII fast paths do not take: other
    /// whitespace, control characters, wide and combining characters, emoji sequences.
    #[test]
    fn unusual_text_measures_and_wraps_as_before() {
        /// Text, width, its lines, their ranges, and the text truncated to the width.
        type Case = (&'static str, u16, &'static [&'static str], &'static [Range<usize>], &'static str);
        let cases: [Case; 12] = [
            ("a\u{a0}b c\u{a0}\u{a0}dd", 3, &["a\u{a0}b", "c", "dd"], &[0..4, 5..6, 10..12], "a\u{a0}…"),
            ("x\u{3000}y z", 2, &["x", "y", "z"], &[0..1, 4..5, 6..7], "x…"),
            ("tab\there and\u{b}vt", 4, &["tab", "here", "and", "vt"], &[0..3, 4..8, 9..12, 13..15], "tab…"),
            (
                "界界 界界界 e\u{301}e\u{301}e\u{301}",
                3,
                &["界", "界", "界", "界", "界", "e\u{301}e\u{301}e\u{301}"],
                &[0..3, 3..6, 7..10, 10..13, 13..16, 17..26],
                "界…",
            ),
            ("  lead  and trail  ", 5, &["lead", "and", "trail", ""], &[2..6, 8..11, 12..17, 0..0], "  le…"),
            ("😀😀 ok", 3, &["😀", "😀", "ok"], &[0..4, 4..8, 9..11], "😀…"),
            ("a\r\nb c", 2, &["a", "b", "c"], &[0..1, 3..4, 5..6], "a…"),
            ("über straße ünïcödé", 6, &["über", "straße", "ünïcöd", "é"], &[0..5, 6..13, 14..23, 23..25], "über …"),
            ("x\u{85}y\u{2028}z", 1, &["x", "y", "z"], &[0..1, 3..4, 7..8], "…"),
            ("control\u{7}bell word", 8, &["control\u{7}", "bell", "word"], &[0..8, 8..12, 13..17], "control…"),
            (
                "👨\u{200d}👩\u{200d}👧 family",
                4,
                &["👨\u{200d}👩\u{200d}👧 f", "amil", "y"],
                &[0..20, 20..24, 24..25],
                "👨\u{200d}👩\u{200d}👧 …",
            ),
            ("add abcdefghijk),", 8, &["add abcd", "efghij", "k),"], &[0..8, 8..14, 14..17], "add abc…"),
        ];
        for (text, max, lines, ranges, truncated) in cases {
            assert_eq!(wrap(text, max), lines, "{text:?}");
            assert_eq!(wrap_ranges(text, max), ranges, "{text:?}");
            assert_eq!(truncate(text, max), truncated, "{text:?}");
        }
        let widths = ["\t", "\u{7}", "\u{b}", "\r\n", "\u{a0}", "~", " ", "👨\u{200d}👩", "\u{7f}", ""].map(width);
        assert_eq!(widths, [1, 1, 1, 1, 1, 1, 1, 2, 1, 0]);
    }
}
