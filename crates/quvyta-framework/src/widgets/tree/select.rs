//! Selecting several nodes of a tree at once: Ctrl and Shift with the pointer, Shift with the
//! arrows, Space and Esc.
//!
//! The selection is a set of node keys the application keeps, next to the one selected node of a
//! plain tree, which stays the cursor: the row the keys move from, the row with the pillar. The
//! tree only ever reports whole new selections, so the application never merges anything.

use crate::event::{KeyEvent, KeyKind, MouseButton, MouseEvent, MouseKind};
use crate::geometry::Rect;
use crate::keymap::{Key, KeyChord, Modifiers};
use crate::widget::EventCx;

use super::super::click::{Click, LastPress};
use super::super::rows::{RowScroll, Step};
use super::super::select_box::SelectBox;
use super::{Flat, Tree};

/// Where a range selection starts: the row last clicked or moved to without Shift, kept in the
/// tree's memory by key so it survives rows opening and closing.
#[derive(Debug, Default)]
struct Anchor(Option<String>);

/// The last press on a row, to tell a double click, kept in the tree's memory by key.
#[derive(Debug, Default)]
struct Presses(LastPress<String>);

/// The selection box being drawn over the tree, kept in its memory while the button is held.
#[derive(Debug, Default)]
pub(super) struct TreeBox(Option<SelectBox<String>>);

impl TreeBox {
    /// The cells of the box being drawn, while it is.
    pub(super) fn drawn(&self) -> Option<Rect> {
        self.0.as_ref().map(SelectBox::rect)
    }
}

impl<Msg: 'static> Tree<Msg> {
    /// Counts a press on the row of `key`; true when it makes a double click in a tree that opens
    /// rows with two clicks. A tree that opens with one click counts nothing.
    pub(super) fn double_press(&self, cx: &mut EventCx<'_, Msg>, key: &str) -> bool {
        if self.activate_on != Click::Double {
            return false;
        }
        let now = cx.now();
        cx.memory::<Presses>().0.press(key.to_owned(), now)
    }

    /// Forgets the last press: it became a drag or a modified click.
    pub(super) fn forget_press(&self, cx: &mut EventCx<'_, Msg>) {
        cx.memory::<Presses>().0.forget();
    }

    /// The keys of the rows of `flat` that the cells of `rect` cover, in tree order.
    fn covered(cx: &mut EventCx<'_, Msg>, flat: &[Flat<'_>], rect: Rect) -> Vec<String> {
        let area = cx.area();
        let offset = cx.memory::<RowScroll>().offset;
        (rect.y.max(area.y)..rect.bottom().min(area.bottom()))
            .filter_map(|y| usize::try_from(y - area.y).ok())
            .filter_map(|row| flat.get(offset + row))
            .map(|row| row.node.key.clone())
            .collect()
    }

    /// Pointer input of a tree that selects by box, with `index` the row under the pointer: a
    /// press on the free space starts a box, a drag stretches it and the release ends it, the
    /// covered rows reported as the selection all along. `None` when the event is not the box's.
    pub(super) fn box_pointer(
        &self,
        cx: &mut EventCx<'_, Msg>,
        mouse: &MouseEvent,
        flat: &[Flat<'_>],
        index: Option<usize>,
    ) -> Option<bool> {
        let at = (mouse.x, mouse.y);
        let drawn = match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                let free = Self::rows_area(cx.area(), flat.len()).contains(mouse.x, mouse.y) && index.is_none();
                if !self.box_select || !self.is_multi() || !free {
                    return None;
                }
                self.forget_press(cx);
                cx.capture_pointer();
                SelectBox::new(at, mouse.mods.ctrl && !mouse.mods.alt, &self.chosen)
            }
            MouseKind::Drag(MouseButton::Left) | MouseKind::Up(MouseButton::Left) => {
                let mut drawn = cx.memory::<TreeBox>().0.take()?;
                drawn.stretch(at);
                drawn
            }
            _ => return None,
        };
        let covered = Self::covered(cx, flat, drawn.rect());
        self.choose(cx, drawn.selection(covered));
        if mouse.kind != MouseKind::Up(MouseButton::Left) {
            cx.memory::<TreeBox>().0 = Some(drawn);
        }
        Some(true)
    }

    /// Whether the tree selects several nodes.
    pub(super) fn is_multi(&self) -> bool {
        self.on_choose.is_some()
    }

    /// Whether `key` is selected: in the selection of a multi-select tree, the selected node of a
    /// plain one.
    pub(super) fn is_chosen(&self, key: &str) -> bool {
        if self.is_multi() {
            self.chosen.iter().any(|chosen| chosen == key)
        } else {
            self.selected.as_deref() == Some(key)
        }
    }

    /// Whether `key` is one of several selected nodes, the case where a press on it keeps the
    /// selection so it can be dragged or given a context menu as a whole.
    pub(super) fn is_among_many(&self, key: &str) -> bool {
        self.is_multi() && self.chosen.len() > 1 && self.is_chosen(key)
    }

    /// Reports `keys` as the new selection, when it differs from the current one.
    pub(super) fn choose(&self, cx: &mut EventCx<'_, Msg>, keys: Vec<String>) {
        if let Some(message) = &self.on_choose
            && keys != self.chosen
        {
            cx.emit(message(keys));
        }
    }

    /// Moves the cursor to row `index` and, in a multi-select tree, makes that row the whole
    /// selection and the start of the next range: what a plain click or arrow does.
    pub(super) fn select_one(&self, cx: &mut EventCx<'_, Msg>, flat: &[Flat<'_>], index: usize) {
        let Some(row) = flat.get(index) else { return };
        self.select(cx, flat, index);
        if self.is_multi() {
            let key = row.node.key.clone();
            cx.memory::<Anchor>().0 = Some(key.clone());
            self.choose(cx, vec![key]);
        }
    }

    /// Adds row `index` to the selection or takes it out, and moves the cursor there.
    fn toggle(&self, cx: &mut EventCx<'_, Msg>, flat: &[Flat<'_>], index: usize) {
        let Some(row) = flat.get(index) else { return };
        let key = &row.node.key;
        let mut keys = self.chosen.clone();
        match keys.iter().position(|chosen| chosen == key) {
            Some(at) => {
                keys.remove(at);
            }
            None => keys.push(key.clone()),
        }
        self.select(cx, flat, index);
        cx.memory::<Anchor>().0 = Some(key.clone());
        self.choose(cx, keys);
    }

    /// Selects the rows from the anchor to row `index` and moves the cursor there; the anchor stays,
    /// so a second Shift+click or arrow reshapes the same range.
    fn select_range(&self, cx: &mut EventCx<'_, Msg>, flat: &[Flat<'_>], index: usize) {
        if index >= flat.len() {
            return;
        }
        let anchor = cx.memory::<Anchor>().0.clone();
        let start = anchor
            .and_then(|key| flat.iter().position(|row| row.node.key == key))
            .or_else(|| self.selected_index(flat))
            .unwrap_or(index);
        let (low, high) = (start.min(index), start.max(index));
        let keys = flat[low..=high].iter().map(|row| row.node.key.clone()).collect();
        if cx.memory::<Anchor>().0.is_none() {
            cx.memory::<Anchor>().0 = Some(flat[start].node.key.clone());
        }
        self.select(cx, flat, index);
        self.choose(cx, keys);
    }

    /// A press of the left button with `mods` held on row `index` of a multi-select tree: Ctrl
    /// adds or removes the row, Shift selects the range to it. True when the press was one of
    /// those and is used up; a plain press is left to the caller.
    pub(super) fn modified_press(
        &self,
        cx: &mut EventCx<'_, Msg>,
        flat: &[Flat<'_>],
        index: usize,
        mods: Modifiers,
    ) -> bool {
        if !self.is_multi() || mods.alt {
            return false;
        }
        if mods.ctrl != mods.shift {
            // A modified click is never the first half of a double click.
            self.forget_press(cx);
        }
        match (mods.ctrl, mods.shift) {
            (true, false) => self.toggle(cx, flat, index),
            (false, true) => self.select_range(cx, flat, index),
            _ => return false,
        }
        true
    }

    /// The selection keys of a multi-select tree: Shift with ↑/↓, PgUp/PgDn or Home/End extends
    /// the range, Space adds or removes the cursor's row, Esc reduces several selected nodes to the
    /// cursor's. True when used.
    pub(super) fn selection_key(&self, cx: &mut EventCx<'_, Msg>, key: &KeyEvent, flat: &[Flat<'_>]) -> bool {
        if !self.is_multi() || key.kind == KeyKind::Release {
            return false;
        }
        let current = self.selected_index(flat);
        let shifted = Modifiers { shift: true, ..Modifiers::default() };
        let steps = [Key::Up, Key::Down, Key::PageUp, Key::PageDown, Key::Home, Key::End];
        if key.chord.mods == shifted && steps.contains(&key.chord.key) {
            let plain = KeyEvent { chord: KeyChord { mods: Modifiers::default(), ..key.chord }, ..*key };
            let page = usize::from(cx.area().height);
            if let Some(target) = Step::from_key(&plain).and_then(|step| step.apply(current, flat.len(), page)) {
                self.select_range(cx, flat, target);
            }
            return true;
        }
        if key.is_plain(Key::Space) {
            return current.is_some_and(|index| {
                self.toggle(cx, flat, index);
                true
            });
        }
        if key.is_plain(Key::Esc) && self.chosen.len() > 1 {
            // Esc goes on to the parents when there is nothing to reduce, so a dialog holding the
            // tree still closes on it.
            let Some(index) = current else { return false };
            self.select_one(cx, flat, index);
            return true;
        }
        false
    }
}
