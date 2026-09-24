//! A box drawn with the pointer over a widget's rows, selecting the rows it covers.
//!
//! A press on the free space of a [`Tree`](super::Tree), a [`Table`](super::Table) or a
//! [`CardGrid`](super::CardGrid) that selects by box starts one; dragging stretches it from the
//! press to the pointer, and the rows it covers become the selection as it goes, so what will be
//! selected is always what is shown. With Ctrl held at the press the covered rows join the
//! selection there was instead of replacing it. A press that never moves is a click on the free
//! space, which covers no row: it clears the selection, as it does in a file explorer.
//!
//! The box is a tone, never a frame: the cells it covers take the tone of selected text, so it
//! reads as the same thing a text selection is.

use crate::geometry::Rect;
use crate::widget::PaintCx;

/// A box being drawn, kept in the widget's memory while the button is held. `K` is how the widget
/// names a row: a node key or an index.
#[derive(Debug, Clone)]
pub(crate) struct SelectBox<K> {
    start: (i32, i32),
    pointer: (i32, i32),
    /// The selection the covered rows join: the one there was when Ctrl was held at the press,
    /// none otherwise.
    base: Vec<K>,
}

impl<K: Clone + PartialEq> SelectBox<K> {
    /// A box pressed at `at`, adding to `base` when `add` and replacing it otherwise.
    pub(crate) fn new(at: (i32, i32), add: bool, base: &[K]) -> Self {
        Self { start: at, pointer: at, base: if add { base.to_vec() } else { Vec::new() } }
    }

    /// Stretches the box to the pointer at `at`.
    pub(crate) fn stretch(&mut self, at: (i32, i32)) {
        self.pointer = at;
    }

    /// The cells the box covers, from the press to the pointer, both included.
    pub(crate) fn rect(&self) -> Rect {
        let (left, right) = (self.start.0.min(self.pointer.0), self.start.0.max(self.pointer.0));
        let (top, bottom) = (self.start.1.min(self.pointer.1), self.start.1.max(self.pointer.1));
        let size = |low: i32, high: i32| u16::try_from(high - low + 1).unwrap_or(u16::MAX);
        Rect::new(left, top, size(left, right), size(top, bottom))
    }

    /// The selection with the rows the box covers, `covered`, in their order after the ones kept.
    pub(crate) fn selection(&self, covered: impl IntoIterator<Item = K>) -> Vec<K> {
        let mut keys = self.base.clone();
        for key in covered {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }
}

/// Paints the box `rect` inside `area`: the covered cells take the tone of selected text, their
/// glyphs kept and brightened where they would no longer read on it.
pub(crate) fn paint(cx: &mut PaintCx<'_>, rect: Rect, area: Rect) {
    let rect = rect.intersect(area);
    if rect.is_empty() {
        return;
    }
    let style = cx.style("text-selection", None, &[]).text();
    let bg = style.bg.unwrap_or_else(|| cx.color("active"));
    let readable = style.fg.unwrap_or_else(|| cx.color("text"));
    cx.fill_keeping_text_readable(rect, bg, readable);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_runs_from_the_press_to_the_pointer_whichever_way_it_is_dragged() {
        let mut drawn = SelectBox::<usize>::new((10, 8), false, &[]);
        assert_eq!(drawn.rect(), Rect::new(10, 8, 1, 1), "a press alone covers its own cell");
        drawn.stretch((4, 3));
        assert_eq!(drawn.rect(), Rect::new(4, 3, 7, 6));
    }

    #[test]
    fn ctrl_adds_the_covered_rows_to_the_selection_and_a_plain_box_replaces_it() {
        let plain = SelectBox::new((0, 0), false, &[1, 2]);
        assert_eq!(plain.selection([2, 5]), [2, 5]);
        let added = SelectBox::new((0, 0), true, &[1, 2]);
        assert_eq!(added.selection([2, 5]), [1, 2, 5], "a row already selected is not added twice");
        assert_eq!(added.selection([]), [1, 2], "covering nothing keeps what there was");
    }
}
