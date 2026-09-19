//! Measuring text in terminal cells: width, truncation with an ellipsis at the end or in the
//! middle, and word wrapping.

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

/// `text` cut to at most `max` cells by removing its middle, with `…` where the middle was.
///
/// For paths and other text whose start and end both matter: the head says where, the tail
/// says what. Text that fits is returned unchanged. Otherwise the cells left after the
/// ellipsis are shared between head and tail, the tail getting the odd one; a grapheme cluster
/// (a wide character, a letter with its combining marks) is never split, and a cell one side
/// cannot use goes to the other. With `max` 1 only the ellipsis remains, and with 0 nothing.
///
/// ```
/// use qframe::text::{truncate_middle, width};
///
/// let path = "~/.config/quvyta/launcher.conf";
/// assert_eq!(truncate_middle(path, 40), path);
/// assert_eq!(truncate_middle(path, 20), "~/.config…ncher.conf");
/// assert_eq!(width(&truncate_middle("~/文書/設定/launcher.conf", 12)), 12);
/// ```
#[must_use]
pub fn truncate_middle(text: &str, max: u16) -> Cow<'_, str> {
    if width(text) <= max {
        return Cow::Borrowed(text);
    }
    if max == 0 {
        return Cow::Borrowed("");
    }
    let budget = max - 1;
    let graphemes: Vec<&str> = text.graphemes(true).collect();
    let (mut head, mut head_used) = fitting(graphemes.iter(), budget / 2);
    let (tail, tail_used) = fitting(graphemes[head..].iter().rev(), budget - head_used);
    // A wide character at the tail's edge can leave a cell the head is able to use.
    let (more, more_used) = fitting(graphemes[head..graphemes.len() - tail].iter(), budget - head_used - tail_used);
    head += more;
    head_used += more_used;
    debug_assert!(head_used + tail_used <= budget);
    let mut out = graphemes[..head].concat();
    out.push_str(ELLIPSIS);
    out.push_str(&graphemes[graphemes.len() - tail..].concat());
    Cow::Owned(out)
}

/// How many of `graphemes`, taken in order, fit in `budget` cells, and the cells they use.
fn fitting<'a>(graphemes: impl Iterator<Item = &'a &'a str>, budget: u16) -> (usize, u16) {
    let mut count = 0;
    let mut used = 0u16;
    for grapheme in graphemes {
        let w = grapheme_width(grapheme);
        if used + w > budget {
            break;
        }
        used += w;
        count += 1;
    }
    (count, used)
}

/// Splits `text` into lines no wider than `max` cells.
///
/// Explicit newlines are kept, words move to the next line whole when they fit on it (with
/// the punctuation attached to them, so a comma never starts a line), and words longer than a
/// line are broken between grapheme clusters; the punctuation closing such a word breaks off
/// with the character before it, so `.` or `)` never starts a line alone either. Spaces at a
/// break are dropped; no-break spaces (U+00A0, U+202F, U+2007) are part of the word.
///
/// Chinese and Japanese, written without spaces, may break between any two ideographs or
/// kana, except that closing punctuation (`。` `、` `」` `！`), the long vowel mark `ー` and
/// small kana never start a line and opening brackets (`「` `（`) never end one. Only a run of
/// such marks wider than the line itself breaks the rule.
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
        for (offset, word, is_space) in runs(paragraph).flat_map(|(offset, run, space)| pieces(offset, run, space)) {
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
            // A no-break space before that punctuation (French `vrai\u{a0}?`) goes with it too.
            let tail =
                graphemes.iter().rposition(|(_, g)| !is_closing_punctuation(g) && !is_no_break_space(g)).unwrap_or(0);
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

/// The pieces a run from [`runs`] may break between: the run itself, unless it holds Chinese or
/// Japanese, which is written without spaces and breaks between ideographs and kana instead.
fn pieces(offset: usize, run: &str, space: bool) -> impl Iterator<Item = (usize, &str, bool)> {
    let mut ends = Vec::new();
    if !space && !is_printable_ascii(run) {
        let graphemes: Vec<(usize, &str)> = run.grapheme_indices(true).collect();
        ends.extend(graphemes.windows(2).filter(|pair| may_break_between(pair[0].1, pair[1].1)).map(|pair| pair[1].0));
    }
    ends.push(run.len());
    let mut start = 0;
    ends.into_iter().map(move |end| {
        let piece = (offset + start, &run[start..end], space);
        start = end;
        piece
    })
}

/// Whether a line may break between two graphemes of one run: only next to Chinese or Japanese,
/// and never before closing punctuation or after an opening bracket (the kinsoku rule).
fn may_break_between(before: &str, after: &str) -> bool {
    (is_cjk(before) || is_cjk(after)) && !is_closing_punctuation(after) && !is_opening_punctuation(before)
}

/// Whether `grapheme` is Chinese or Japanese text that breaks between its characters: ideographs,
/// kana, CJK punctuation and fullwidth forms. Hangul is not: Korean separates words with spaces.
fn is_cjk(grapheme: &str) -> bool {
    grapheme.chars().next().is_some_and(|c| {
        matches!(c,
            '\u{2E80}'..='\u{2FDF}'      // radicals
            | '\u{3000}'..='\u{30FF}'    // CJK punctuation, hiragana, katakana
            | '\u{31C0}'..='\u{31FF}'    // strokes, katakana extensions
            | '\u{3400}'..='\u{4DBF}'    // extension A
            | '\u{4E00}'..='\u{9FFF}'    // unified ideographs
            | '\u{F900}'..='\u{FAFF}'    // compatibility ideographs
            | '\u{FE30}'..='\u{FE4F}'    // vertical and compatibility forms
            | '\u{FF00}'..='\u{FFEF}'    // fullwidth and halfwidth forms
            | '\u{20000}'..='\u{3FFFF}') // supplementary ideographs
    })
}

/// Whether the character starting at byte `index` of `text` is a space a line may break at, and
/// its length in bytes; `None` past the end. Wrapping looks at every character of a text, and
/// ASCII, most of what is wrapped, needs no decoding.
fn char_at(text: &str, index: usize) -> Option<(bool, usize)> {
    let byte = *text.as_bytes().get(index)?;
    if byte.is_ascii() {
        return Some((char::from(byte).is_whitespace(), 1));
    }
    text.get(index..)?.chars().next().map(|c| (c.is_whitespace() && !is_no_break(c), c.len_utf8()))
}

/// Whether `c` is a space that binds the words on either side: French puts one before `?` and
/// `:`, and numbers group their digits with one.
fn is_no_break(c: char) -> bool {
    matches!(c, '\u{A0}' | '\u{202F}' | '\u{2007}')
}

fn is_no_break_space(grapheme: &str) -> bool {
    grapheme.chars().all(is_no_break)
}

/// Whether `grapheme` is punctuation that closes what comes before it and must not start a line.
/// Chinese and Japanese add their own full stops, commas and brackets, the long vowel mark,
/// iteration marks and the small kana that belong to the syllable before them.
fn is_closing_punctuation(grapheme: &str) -> bool {
    grapheme.chars().all(|c| {
        matches!(
            c,
            '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '"' | '\'' | '…' | '’' | '”' | '»'
                | '、' | '。' | '〃' | '々' | '〉' | '》' | '」' | '』' | '】' | '〕' | '〗' | '〙' | '〛' | '〞' | '〟'
                | '〻' | '・' | 'ー' | 'ゝ' | 'ゞ' | 'ヽ' | 'ヾ' | '゛' | '゜' | '゠' | '‼' | '⁇' | '⁈' | '⁉'
                | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ' | 'ゕ' | 'ゖ'
                | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ' | 'ッ' | 'ャ' | 'ュ' | 'ョ' | 'ヮ' | 'ヵ' | 'ヶ'
                | '\u{31F0}'..='\u{31FF}'
                | '！' | '）' | '，' | '．' | '：' | '；' | '？' | '］' | '｝' | '｠' | '｡' | '｣' | '､' | '･' | 'ｰ'
                | 'ｧ'..='ｯ' | 'ﾞ' | 'ﾟ' | '％' | '〜' | '～'
        )
    })
}

/// Whether `grapheme` opens what comes after it and must not end a line.
fn is_opening_punctuation(grapheme: &str) -> bool {
    grapheme.chars().all(|c| {
        matches!(
            c,
            '(' | '['
                | '{'
                | '‘'
                | '“'
                | '«'
                | '〈'
                | '《'
                | '「'
                | '『'
                | '【'
                | '〔'
                | '〖'
                | '〘'
                | '〚'
                | '〝'
                | '（'
                | '［'
                | '｛'
                | '｟'
                | '｢'
        )
    })
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
    fn truncate_middle_returns_text_that_fits_unchanged() {
        assert!(matches!(truncate_middle("launcher.conf", 13), Cow::Borrowed("launcher.conf")));
        assert!(matches!(truncate_middle("", 0), Cow::Borrowed("")));
    }

    #[test]
    fn truncate_middle_keeps_head_and_tail_of_a_path() {
        let path = "~/.config/quvyta/launcher.conf";
        assert_eq!(truncate_middle(path, 25), "~/.config/qu…auncher.conf");
        assert_eq!(truncate_middle(path, 20), "~/.config…ncher.conf", "the tail gets the odd cell");
        assert_eq!(truncate_middle(path, 5), "~/…nf");
        for max in 0..=30 {
            assert_eq!(width(&truncate_middle(path, max)), max, "{max}");
        }
    }

    #[test]
    fn truncate_middle_never_splits_wide_characters() {
        let path = "~/文書/設定/launcher.conf";
        assert_eq!(width(path), 25);
        // The head cannot use its fifth cell for half of 書, so the tail takes it.
        assert_eq!(truncate_middle(path, 12), "~/文…er.conf");
        assert_eq!(truncate_middle("界界界界界界", 6), "界…界", "one cell stays empty rather than half a character");
        for max in 0..=25 {
            assert!(width(&truncate_middle(path, max)) <= max, "{max}");
            assert!(width(&truncate_middle("界界界界界界", max)) <= max, "{max}");
        }
    }

    #[test]
    fn truncate_middle_keeps_combining_marks_with_their_letter() {
        let accented = "e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}";
        assert_eq!(truncate_middle(accented, 4), "e\u{301}…e\u{301}e\u{301}");
        assert_eq!(truncate_middle("café\u{301}s/ünïcödé\u{301}", 7), "caf…ödé\u{301}");
    }

    #[test]
    fn truncate_middle_at_tiny_widths() {
        assert_eq!(truncate_middle("launcher.conf", 0), "");
        assert_eq!(truncate_middle("launcher.conf", 1), "…");
        assert_eq!(truncate_middle("launcher.conf", 2), "…f");
        assert_eq!(truncate_middle("文書", 2), "…", "a wide tail does not fit in one cell");
        assert_eq!(truncate_middle("文書", 3), "…書");
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

    #[test]
    fn cjk_closing_punctuation_never_starts_a_line() {
        assert_eq!(wrap("これはテストです。", 16), vec!["これはテストで", "す。"], "a full stop keeps its company");
        assert_eq!(wrap("你好，世界", 4), vec!["你", "好，", "世界"], "an ideographic comma stays");
        assert_eq!(
            wrap("彼は「はい」と言った", 6),
            vec!["彼は", "「は", "い」と", "言った"],
            "brackets hold on to what they enclose"
        );
        assert_eq!(wrap("コーヒー", 4), vec!["コー", "ヒー"], "the long vowel mark stays after its kana");
        assert_eq!(wrap("ちょっと", 6), vec!["ちょっ", "と"], "a small kana stays after the one it follows");
        for text in ["一二三四五六七八九十、一二三。", "（全角）です！次は？", "設定を保存しました！次へ進みますか？"]
        {
            for max in 4..12 {
                for line in wrap(text, max).iter().skip(1) {
                    let first = line.graphemes(true).next().unwrap_or_default();
                    assert!(!is_closing_punctuation(first), "{text:?} at {max}: a line starts with {first:?}");
                }
                for line in wrap(text, max) {
                    let last = line.graphemes(true).next_back().unwrap_or_default();
                    assert!(
                        line.graphemes(true).count() == 1 || !is_opening_punctuation(last),
                        "{text:?} at {max}: a line ends with {last:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn cjk_text_without_spaces_breaks_between_ideographs() {
        assert_eq!(wrap("防火墙已启用", 4), vec!["防火", "墙已", "启用"]);
        assert_eq!(wrap("状态 防火墙已启用", 10), vec!["状态 防火", "墙已启用"], "the rest of a line is filled");
        assert_eq!(wrap("hello 你好世界", 8), vec!["hello 你", "好世界"]);
        assert_eq!(wrap("Rust で書く", 7), vec!["Rust で", "書く"]);
        assert_eq!(wrap("パッケージを更新", 10), vec!["パッケージ", "を更新"]);
        assert_eq!(wrap("안녕하세요 세계", 10), vec!["안녕하세요", "세계"], "Korean words stay whole");
    }

    #[test]
    fn no_break_spaces_belong_to_the_word() {
        assert_eq!(wrap("Est-ce vrai\u{a0}? Oui", 11), vec!["Est-ce", "vrai\u{a0}? Oui"]);
        assert_eq!(wrap("Attention\u{202f}: fin", 10), vec!["Attentio", "n\u{202f}: fin"]);
        assert_eq!(wrap("total 10\u{2007}000 kr", 8), vec!["total", "10\u{2007}000", "kr"]);
        assert_eq!(
            wrap("Vraiment\u{a0}?", 9),
            vec!["Vraimen", "t\u{a0}?"],
            "a broken word keeps its space with the mark"
        );
        assert_eq!(wrap("a b\u{a0}c", 3), vec!["a", "b\u{a0}c"]);
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
