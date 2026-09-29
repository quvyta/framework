//! The filter field of pickers inside layers. The fuzzy matching behind it is [`text::fuzzy`](crate::text::fuzzy).

use unicode_segmentation::UnicodeSegmentation;

use super::editor::Editor;
use crate::event::Event;
use crate::geometry::Rect;
use crate::keymap::{Key, Modifiers};
use crate::style::CellStyle;
use crate::text;
use crate::widget::PaintCx;

pub(crate) use crate::text::fuzzy;

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
