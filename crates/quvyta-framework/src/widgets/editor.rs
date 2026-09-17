//! A text editing model: cursor, selection, word motion and undo, for single-line fields and,
//! with line breaks kept, multi-line areas.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

/// Most undo steps kept; older ones are dropped, since every step holds a copy of the text.
const MAX_UNDO: usize = 200;

/// What the last change was, so consecutive typing undoes as one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeKind {
    Typing,
    Deleting,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    text: String,
    cursor: usize,
}

/// Editable text with a cursor at a grapheme boundary and an optional selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Editor {
    text: String,
    cursor: usize,
    anchor: Option<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_change: ChangeKind,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new("")
    }
}

impl Editor {
    pub(crate) fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            cursor: text.len(),
            anchor: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_change: ChangeKind::Other,
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// Byte offset of the cursor.
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    /// The selected byte range, if any text is selected.
    pub(crate) fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        (anchor != self.cursor).then(|| anchor.min(self.cursor)..anchor.max(self.cursor))
    }

    pub(crate) fn selected_text(&self) -> Option<&str> {
        self.selection().map(|range| &self.text[range])
    }

    /// Replaces the text from outside (the application changed the value).
    pub(crate) fn replace_all(&mut self, text: &str) {
        *self = Self::new(text);
    }

    /// Inserts `text` at the cursor, replacing the selection. Returns whether the text changed.
    pub(crate) fn insert(&mut self, text: &str, max_graphemes: Option<usize>) -> bool {
        self.insert_clean(&text.chars().filter(|c| !c.is_control()).collect::<String>(), max_graphemes)
    }

    /// Like [`Editor::insert`], but keeps line breaks for multi-line text: `\r\n` and `\r`
    /// become `\n` and a tab becomes four spaces.
    pub(crate) fn insert_lines(&mut self, text: &str, max_graphemes: Option<usize>) -> bool {
        let text = text.replace("\r\n", "\n").replace('\r', "\n").replace('\t', "    ");
        self.insert_clean(&text.chars().filter(|c| *c == '\n' || !c.is_control()).collect::<String>(), max_graphemes)
    }

    /// Places the cursor at byte `offset`, which must be a grapheme boundary, extending the
    /// selection when `select`.
    pub(crate) fn move_to_offset(&mut self, offset: usize, select: bool) {
        self.move_to(offset.min(self.text.len()), select);
    }

    /// Deletes the byte `range`, whose ends must be grapheme boundaries.
    pub(crate) fn delete_range(&mut self, range: Range<usize>) -> bool {
        self.remove_as(range, ChangeKind::Other)
    }

    fn insert_clean(&mut self, text: &str, max_graphemes: Option<usize>) -> bool {
        if text.is_empty() && self.selection().is_none() {
            return false;
        }
        let kept = self.text.graphemes(true).count()
            - self.selection().map_or(0, |range| self.text[range].graphemes(true).count());
        let allowed = max_graphemes.map_or(usize::MAX, |max| max.saturating_sub(kept));
        let text: String = text.graphemes(true).take(allowed).collect();
        if text.is_empty() && self.selection().is_none() {
            return false;
        }
        self.record(if text.chars().all(|c| !c.is_whitespace()) { ChangeKind::Typing } else { ChangeKind::Other });
        self.delete_selection_inner();
        self.text.insert_str(self.cursor, &text);
        self.cursor += text.len();
        true
    }

    /// Deletes the selection or the grapheme before the cursor.
    pub(crate) fn backspace(&mut self) -> bool {
        if self.selection().is_some() {
            self.record(ChangeKind::Other);
            self.delete_selection_inner();
            return true;
        }
        let start = self.prev_boundary(self.cursor);
        self.remove(start..self.cursor)
    }

    /// Deletes the selection or the grapheme after the cursor.
    pub(crate) fn delete(&mut self) -> bool {
        if self.selection().is_some() {
            self.record(ChangeKind::Other);
            self.delete_selection_inner();
            return true;
        }
        let end = self.next_boundary(self.cursor);
        self.remove(self.cursor..end)
    }

    /// Deletes the word before the cursor.
    pub(crate) fn delete_word_back(&mut self) -> bool {
        let start = self.word_start(self.cursor);
        self.remove_as(start..self.cursor, ChangeKind::Other)
    }

    /// Deletes everything before the cursor.
    pub(crate) fn delete_to_start(&mut self) -> bool {
        self.remove_as(0..self.cursor, ChangeKind::Other)
    }

    /// Moves one grapheme (or word) left. Without `select`, a selection is cleared and the move
    /// starts from its left end, so the cursor lands one step before it (at the start of the text
    /// it stays there).
    pub(crate) fn move_left(&mut self, select: bool, by_word: bool) {
        let from = self.selection().filter(|_| !select).map_or(self.cursor, |range| range.start);
        let target = if by_word { self.word_start(from) } else { self.prev_boundary(from) };
        self.move_to(target, select);
    }

    /// Moves one grapheme (or word) right. Without `select`, a selection is cleared and the move
    /// starts from its right end, so the cursor lands one step after it.
    pub(crate) fn move_right(&mut self, select: bool, by_word: bool) {
        let from = self.selection().filter(|_| !select).map_or(self.cursor, |range| range.end);
        let target = if by_word { self.word_end(from) } else { self.next_boundary(from) };
        self.move_to(target, select);
    }

    pub(crate) fn move_home(&mut self, select: bool) {
        self.move_to(0, select);
    }

    pub(crate) fn move_end(&mut self, select: bool) {
        self.move_to(self.text.len(), select);
    }

    pub(crate) fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Places the cursor at grapheme index `index`, extending the selection when `select`.
    pub(crate) fn move_to_grapheme(&mut self, index: usize, select: bool) {
        let offset = self.text.grapheme_indices(true).nth(index).map_or(self.text.len(), |(offset, _)| offset);
        self.move_to(offset, select);
    }

    pub(crate) fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };
        self.redo.push(Snapshot { text: self.text.clone(), cursor: self.cursor });
        self.restore(snapshot);
        true
    }

    pub(crate) fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        self.undo.push(Snapshot { text: self.text.clone(), cursor: self.cursor });
        self.restore(snapshot);
        true
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.text = snapshot.text;
        self.cursor = snapshot.cursor.min(self.text.len());
        self.anchor = None;
        self.last_change = ChangeKind::Other;
    }

    fn move_to(&mut self, offset: usize, select: bool) {
        if select {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = offset;
        self.last_change = ChangeKind::Other;
    }

    fn remove(&mut self, range: Range<usize>) -> bool {
        self.remove_as(range, ChangeKind::Deleting)
    }

    fn remove_as(&mut self, range: Range<usize>, kind: ChangeKind) -> bool {
        if range.is_empty() {
            return false;
        }
        self.record(kind);
        self.text.replace_range(range.clone(), "");
        self.cursor = range.start;
        self.anchor = None;
        true
    }

    fn delete_selection_inner(&mut self) {
        if let Some(range) = self.selection() {
            self.text.replace_range(range.clone(), "");
            self.cursor = range.start;
        }
        self.anchor = None;
    }

    /// Saves an undo step unless this change continues the previous one of the same kind.
    fn record(&mut self, kind: ChangeKind) {
        let continues = kind != ChangeKind::Other && kind == self.last_change;
        if !continues {
            if self.undo.len() == MAX_UNDO {
                self.undo.remove(0);
            }
            self.undo.push(Snapshot { text: self.text.clone(), cursor: self.cursor });
        }
        self.redo.clear();
        self.last_change = kind;
    }

    fn prev_boundary(&self, offset: usize) -> usize {
        self.text[..offset].grapheme_indices(true).next_back().map_or(0, |(i, _)| i)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.text[offset..].graphemes(true).next().map_or(offset, |g| offset + g.len())
    }

    fn word_start(&self, offset: usize) -> usize {
        let mut result = 0;
        for (index, word) in self.text[..offset].split_word_bound_indices() {
            if !word.chars().all(char::is_whitespace) {
                result = index;
            }
        }
        result
    }

    fn word_end(&self, offset: usize) -> usize {
        for (index, word) in self.text[offset..].split_word_bound_indices() {
            if !word.chars().all(char::is_whitespace) {
                return offset + index + word.len();
            }
        }
        self.text.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_deletes_and_respects_graphemes() {
        let mut editor = Editor::new("çay");
        editor.move_left(false, false);
        assert!(editor.insert("ı", None));
        assert_eq!(editor.text(), "çaıy");
        assert!(editor.backspace());
        editor.move_home(false);
        assert!(editor.delete());
        assert_eq!(editor.text(), "ay");
        assert!(!editor.backspace());
    }

    #[test]
    fn word_motion_and_selection() {
        let mut editor = Editor::new("quvyta framework rocks");
        editor.move_left(false, true);
        assert_eq!(&editor.text()[editor.cursor()..], "rocks");
        editor.move_left(true, true);
        assert_eq!(editor.selected_text(), Some("framework "));
        assert!(editor.insert("ui ", None));
        assert_eq!(editor.text(), "quvyta ui rocks");
        editor.move_end(false);
        assert!(editor.delete_word_back());
        assert_eq!(editor.text(), "quvyta ui ");
        editor.select_all();
        assert!(editor.backspace());
        assert_eq!(editor.text(), "");
    }

    #[test]
    fn arrows_with_a_selection_clear_it_and_move_one_step_past_its_end() {
        let mut editor = Editor::new("deploy api now");
        editor.move_to_offset(7, false);
        editor.move_to_offset(10, true);
        assert_eq!(editor.selected_text(), Some("api"));
        editor.move_right(false, false);
        assert_eq!((editor.selection(), editor.cursor()), (None, 11), "one past the right end");
        editor.move_to_offset(10, false);
        editor.move_to_offset(7, true);
        editor.move_left(false, false);
        assert_eq!((editor.selection(), editor.cursor()), (None, 6), "one before the left end");
        editor.move_to_offset(7, false);
        editor.move_to_offset(10, true);
        editor.move_right(false, true);
        assert_eq!(editor.cursor(), 14, "a word jump starts from the right end");
        editor.move_to_offset(10, false);
        editor.move_to_offset(7, true);
        editor.move_left(false, true);
        assert_eq!(editor.cursor(), 0, "and from the left end");
        editor.select_all();
        editor.move_right(false, false);
        assert_eq!((editor.selection(), editor.cursor()), (None, 14), "at the end it stays");
        editor.select_all();
        editor.move_left(false, false);
        assert_eq!((editor.selection(), editor.cursor()), (None, 0), "at the start too");
        editor.move_to_offset(7, false);
        editor.move_to_offset(10, true);
        editor.move_right(true, false);
        assert_eq!(editor.selected_text(), Some("api "), "shift still extends");
    }

    #[test]
    fn typing_undoes_as_one_step_and_redo_works() {
        let mut editor = Editor::new("");
        for c in ["a", "b", "c"] {
            editor.insert(c, None);
        }
        editor.insert(" ", None);
        editor.insert("d", None);
        assert!(editor.undo());
        assert_eq!(editor.text(), "abc ");
        assert!(editor.undo());
        assert_eq!(editor.text(), "abc");
        assert!(editor.undo());
        assert_eq!(editor.text(), "");
        assert!(editor.redo());
        assert_eq!(editor.text(), "abc");
    }

    #[test]
    fn undo_history_keeps_only_the_latest_steps() {
        let mut editor = Editor::new("");
        for _ in 0..MAX_UNDO + 50 {
            editor.insert(" ", None);
        }
        let mut undone = 0;
        while editor.undo() {
            undone += 1;
        }
        assert_eq!(undone, MAX_UNDO, "the oldest steps are dropped, so a long session does not grow without end");
        assert_eq!(editor.text().len(), 50, "undoing stops at the oldest step kept");
    }

    #[test]
    fn single_line_insert_drops_breaks_and_line_insert_keeps_them() {
        let mut editor = Editor::new("");
        assert!(editor.insert("a\nb\tc", None));
        assert_eq!(editor.text(), "abc");
        let mut lines = Editor::new("");
        assert!(lines.insert_lines("a\r\nb\rc\td\u{7}", Some(9)));
        assert_eq!(lines.text(), "a\nb\nc    ", "nine graphemes: the bell is dropped");
        lines.move_to_offset(2, false);
        lines.move_to_offset(4, true);
        assert_eq!(lines.selected_text(), Some("b\n"));
        assert!(lines.delete_range(0..2));
        assert_eq!(lines.text(), "b\nc    ");
        assert!(lines.undo());
        assert_eq!(lines.text(), "a\nb\nc    ", "undo brings the deleted range back");
    }

    #[test]
    fn max_length_and_click_position() {
        let mut editor = Editor::new("ab");
        assert!(editor.insert("cdef", Some(4)));
        assert_eq!(editor.text(), "abcd");
        assert!(!editor.insert("x", Some(4)));
        editor.move_to_grapheme(1, false);
        editor.move_to_grapheme(3, true);
        assert_eq!(editor.selected_text(), Some("bc"));
        editor.move_end(false);
        assert!(editor.delete_to_start());
        assert_eq!(editor.text(), "");
    }
}
