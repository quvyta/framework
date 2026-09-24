//! The filter field of pickers inside layers, and the fuzzy matching behind it.

use unicode_segmentation::UnicodeSegmentation;

use super::editor::Editor;
use crate::event::Event;
use crate::geometry::Rect;
use crate::keymap::{Key, Modifiers};
use crate::style::CellStyle;
use crate::text;
use crate::widget::PaintCx;

/// Edits `editor` with a key or paste. Returns whether the event was used. Up, Down, Enter
/// and Esc are left to the picker.
pub(crate) fn edit(editor: &mut Editor, event: &Event) -> bool {
    match event {
        Event::Paste(pasted) => {
            editor.insert(&pasted.replace(['\n', '\r'], " "), None);
            true
        }
        Event::Key(key) => {
            let mods = key.chord.mods;
            let plain = mods == Modifiers::default();
            match key.chord.key {
                Key::Char('w') if mods.ctrl => editor.delete_word_back(),
                Key::Char('u') if mods.ctrl => editor.delete_to_start(),
                Key::Backspace if plain => editor.backspace(),
                Key::Backspace if mods.ctrl => editor.delete_word_back(),
                Key::Delete if plain => editor.delete(),
                Key::Left if !mods.shift => {
                    editor.move_left(false, mods.ctrl);
                    return true;
                }
                Key::Right if !mods.shift => {
                    editor.move_right(false, mods.ctrl);
                    return true;
                }
                Key::Home if plain => {
                    editor.move_home(false);
                    return true;
                }
                Key::End if plain => {
                    editor.move_end(false);
                    return true;
                }
                _ => match key.text {
                    Some(c) if !mods.ctrl && !mods.alt => editor.insert(&c.to_string(), None),
                    _ => return false,
                },
            };
            true
        }
        _ => false,
    }
}

/// Draws the field in `rect`: a raised row with the search mark, the text or a faint
/// `placeholder`, and a steady block cursor (the field always has the keys while its layer
/// is open).
pub(crate) fn paint(cx: &mut PaintCx<'_>, rect: Rect, editor: &Editor, placeholder: &str) {
    // A space typed into the filter is part of the query, however fast it comes.
    cx.takes_text();
    let field = cx.style("layer-filter", None, &[]).text();
    cx.clear(rect, field.bg.unwrap_or_else(|| cx.color("raised")));
    let mark = cx.env().icons().glyph("search").into_owned();
    let mark_style = cx.style("layer-filter-mark", None, &[]).text();
    let x = rect.x + 1;
    let used = cx.text(x, rect.y, &mark, mark_style, rect.width.saturating_sub(1)) + 1;
    let start = x + i32::from(used);
    let width = crate::geometry::clamp_u16(rect.right() - 1 - start);
    let cursor_style = cx.style("layer-filter-cursor", None, &[]).text();
    let text_style = CellStyle { bg: None, ..field };
    let value = editor.text();
    if value.is_empty() {
        cx.text(start, rect.y, " ", cursor_style, 1);
        let faint = cx.style("layer-filter-placeholder", None, &[]).text();
        let shown = text::truncate(placeholder, width.saturating_sub(2)).into_owned();
        cx.text(start + 2, rect.y, &shown, faint, width.saturating_sub(2));
        return;
    }
    // Keep the cursor in view: drop graphemes from the left while it would fall outside.
    let before: Vec<&str> = value[..editor.cursor()].graphemes(true).collect();
    let mut skip = 0;
    while skip < before.len() && before[skip..].iter().map(|g| text::grapheme_width(g)).sum::<u16>() + 1 > width {
        skip += 1;
    }
    let mut column = start;
    for (index, grapheme) in value.graphemes(true).enumerate().skip(skip) {
        let cells = text::grapheme_width(grapheme).max(1);
        if column + i32::from(cells) > start + i32::from(width) {
            break;
        }
        let style = if index == before.len() { cursor_style } else { text_style };
        cx.text(column, rect.y, grapheme, style, cells);
        column += i32::from(cells);
    }
    if editor.cursor() == value.len() && column < start + i32::from(width) {
        cx.text(column, rect.y, " ", cursor_style, 1);
    }
}

/// A fuzzy match: how good it is and which characters of the text matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Match {
    pub(crate) score: i32,
    /// Character indices of the text, ascending.
    pub(crate) positions: Vec<usize>,
}

/// Matches `query` against `text`, ignoring case and spaces in the query: every query
/// character must appear in order. Consecutive characters and characters at word starts score
/// higher. An empty query matches everything with score 0.
pub(crate) fn fuzzy(query: &str, text: &str) -> Option<Match> {
    // Both sides keep the first character of a lower case form, so a letter such as `İ`, whose
    // lower case is two characters, compares equal to itself.
    let lower_char = |c: char| c.to_lowercase().next().unwrap_or(c);
    let wanted: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).map(lower_char).collect();
    if wanted.is_empty() {
        return Some(Match { score: 0, positions: Vec::new() });
    }
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars.iter().copied().map(lower_char).collect();
    let word_start = |i: usize| {
        i == 0 || !chars[i - 1].is_alphanumeric() || (chars[i].is_uppercase() && chars[i - 1].is_lowercase())
    };
    // Try every place the first character occurs and keep the best greedy alignment.
    let mut best: Option<Match> = None;
    for first in (0..lower.len()).filter(|i| lower[*i] == wanted[0]) {
        let mut positions = vec![first];
        let mut at = first;
        for c in &wanted[1..] {
            let Some(next) = (at + 1..lower.len()).find(|i| lower[*i] == *c) else {
                break;
            };
            positions.push(next);
            at = next;
        }
        if positions.len() < wanted.len() {
            break;
        }
        let mut score = 0;
        for (index, position) in positions.iter().enumerate() {
            score += 1;
            if word_start(*position) {
                score += 4;
            }
            if index > 0 && positions[index - 1] + 1 == *position {
                score += 3;
            }
        }
        score -= i32::try_from(positions[positions.len() - 1] - positions[0]).unwrap_or(i32::MAX / 2) / 4;
        score -= i32::try_from(first).unwrap_or(0).min(8) / 4;
        if best.as_ref().is_none_or(|b| score > b.score) {
            best = Some(Match { score, positions });
        }
    }
    best
}

/// Draws `label` from `(x, y)` in at most `budget` cells, cut with `…`, with the characters at
/// `positions` in the match style.
pub(crate) fn paint_matched(
    cx: &mut PaintCx<'_>,
    x: i32,
    y: i32,
    label: &str,
    budget: u16,
    positions: &[usize],
    style: CellStyle,
) {
    let shown = text::truncate(label, budget);
    let cut = shown.len() != label.len();
    let highlight = CellStyle { bg: None, ..cx.style("layer-match", None, &[]).text() };
    let graphemes: Vec<&str> = shown.graphemes(true).collect();
    let mut column = x;
    let mut char_index = 0;
    for (index, grapheme) in graphemes.iter().enumerate() {
        let ellipsis = cut && index + 1 == graphemes.len();
        let count = grapheme.chars().count();
        let matched = !ellipsis && positions.iter().any(|p| (char_index..char_index + count).contains(p));
        let cells = text::grapheme_width(grapheme);
        let mut grapheme_style = if matched { highlight } else { style };
        grapheme_style.bg = None;
        cx.text(column, y, grapheme, grapheme_style, cells.max(1));
        column += i32::from(cells);
        char_index += count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_in_order_ignoring_case() {
        let found = fuzzy("rst", "Restart container").expect("matches");
        assert_eq!(found.positions, vec![0, 2, 3]);
        assert!(fuzzy("tsr", "Restart").is_none());
        assert_eq!(fuzzy("", "anything").map(|m| m.score), Some(0));
        assert_eq!(fuzzy("o l", "Open logs").map(|m| m.positions), Some(vec![0, 5]));
    }

    #[test]
    fn a_letter_whose_lower_case_is_two_characters_matches_itself() {
        // `İ` lowers to `i` and a combining dot; the text side keeps only the `i`, so the query
        // must do the same or Turkish names never match.
        assert_eq!(fuzzy("İz", "İzmir").map(|m| m.positions), Some(vec![0, 1]));
        assert_eq!(fuzzy("iz", "İzmir").map(|m| m.positions), Some(vec![0, 1]));
    }

    #[test]
    fn word_starts_and_runs_score_higher() {
        let starts = fuzzy("ol", "Open logs").expect("matches").score;
        let middle = fuzzy("ol", "Pool").expect("matches").score;
        assert!(starts > middle, "{starts} {middle}");
        let run = fuzzy("dep", "Deploy").expect("matches").score;
        let spread = fuzzy("dep", "Delete old snapshots and prune").expect("matches").score;
        assert!(run > spread);
        let better = fuzzy("log", "Toggle logs").expect("matches");
        assert_eq!(better.positions, vec![7, 8, 9], "the word start beats the first occurrence");
    }

    #[test]
    fn edits_like_a_field() {
        let mut editor = Editor::default();
        for c in "open lgos".chars() {
            let chord = if c == ' ' { "space".to_owned() } else { c.to_string() };
            edit(&mut editor, &Event::Key(crate::event::KeyEvent::press(&chord)));
        }
        edit(&mut editor, &Event::Key(crate::event::KeyEvent::press("ctrl+w")));
        assert_eq!(editor.text(), "open ");
        edit(&mut editor, &Event::Paste("logs\n".to_owned()));
        assert_eq!(editor.text(), "open logs ");
        assert!(!edit(&mut editor, &Event::Key(crate::event::KeyEvent::press("down"))));
    }
}
