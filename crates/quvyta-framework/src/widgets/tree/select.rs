//! Selecting several nodes of a tree at once: Ctrl and Shift with the pointer, Shift with the
//! arrows, Space and Esc.
//!
//! The selection is a set of node keys the application keeps, next to the one selected node of a
//! plain tree, which stays the cursor: the row the keys move from, the row with the pillar. The
//! tree only ever reports whole new selections, so the application never merges anything.

use crate::event::{KeyEvent, KeyKind};
use crate::keymap::{Key, KeyChord, Modifiers};
use crate::widget::EventCx;

use super::super::rows::Step;
use super::{Flat, Tree};

/// Where a range selection starts: the row last clicked or moved to without Shift, kept in the
/// tree's memory by key so it survives rows opening and closing.
#[derive(Debug, Default)]
struct Anchor(Option<String>);

impl<Msg: 'static> Tree<Msg> {
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
