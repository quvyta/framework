//! The row model shared by row-based widgets (list, table, tree, menu, log view): which row is
//! first on screen, keyboard steps between rows, wheel and scrollbar handling, and painting a
//! touched row's raised surface with its pillar.

use crate::event::{KeyEvent, MouseButton, MouseEvent, MouseKind};
use crate::geometry::{Rect, clamp_u16};
use crate::keymap::Key;
use crate::style::{CellStyle, WidgetStyle};
use crate::theme::State;
use crate::widget::{EventCx, PaintCx};

use super::scrollbar::{self, ScrollMetrics, ScrollbarStyle};

/// Rows (or lines) scrolled by one wheel step, in every scrolling widget.
pub(crate) const WHEEL_ROWS: u16 = 3;

/// Scroll state of a row widget, kept in runtime memory.
#[derive(Debug, Default)]
pub(crate) struct RowScroll {
    /// First visible row.
    pub(crate) offset: usize,
    /// The selection the offset last followed; a new selection scrolls into view once.
    pub(crate) followed: Option<usize>,
    /// Whether the scrollbar thumb is being dragged.
    pub(crate) dragging: bool,
    /// The row that was activated last, for the press flash.
    pub(crate) flashed: Option<usize>,
}

impl RowScroll {
    /// Scrolls so a newly selected row is visible, keeps the offset inside the content and
    /// returns it.
    pub(crate) fn follow(&mut self, selected: Option<usize>, total: usize, visible: usize) -> usize {
        if self.followed != selected {
            if let Some(selected) = selected {
                if selected < self.offset {
                    self.offset = selected;
                } else if visible > 0 && selected >= self.offset + visible {
                    self.offset = selected + 1 - visible;
                }
            }
            self.followed = selected;
        }
        self.offset = self.offset.min(total.saturating_sub(visible));
        self.offset
    }
}

/// A keyboard step between rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

impl Step {
    /// The step a plain key means: ↑/↓ or k/j, PgUp/PgDn, Home/End.
    pub(crate) fn from_key(key: &KeyEvent) -> Option<Self> {
        let pairs = [
            (Key::Up, Self::Up),
            (Key::Char('k'), Self::Up),
            (Key::Down, Self::Down),
            (Key::Char('j'), Self::Down),
            (Key::PageUp, Self::PageUp),
            (Key::PageDown, Self::PageDown),
            (Key::Home, Self::Home),
            (Key::End, Self::End),
        ];
        pairs.into_iter().find(|(k, _)| key.is_plain(*k)).map(|(_, step)| step)
    }

    /// The row this step reaches from `current` among `len` rows, `page` rows to a page.
    pub(crate) fn apply(self, current: Option<usize>, len: usize, page: usize) -> Option<usize> {
        let last = len.checked_sub(1)?;
        let page = page.max(1);
        Some(match (self, current) {
            (Self::Home, _) | (Self::Down | Self::PageDown, None) => 0,
            (Self::End, _) | (Self::Up | Self::PageUp, None) => last,
            (Self::Up, Some(i)) => i.saturating_sub(1),
            (Self::Down, Some(i)) => (i + 1).min(last),
            (Self::PageUp, Some(i)) => i.saturating_sub(page),
            (Self::PageDown, Some(i)) => (i + page).min(last),
        })
    }
}

/// States of one row: hovered when the pointer is on it, selected (and focused), pressed while
/// its activation flashes.
pub(crate) fn row_states(hovered: bool, selected: bool, focused: bool, pressed: bool) -> Vec<State> {
    let mut states = Vec::new();
    if hovered {
        states.push(State::Hover);
    }
    if selected {
        states.push(State::Selected);
        if focused {
            states.push(State::Focus);
        }
    }
    if pressed {
        states.push(State::Pressed);
    }
    states
}

/// Paints a row's surface and pillar from `style` and returns its text style without a
/// background, ready for the row's content.
pub(crate) fn paint_row(cx: &mut PaintCx<'_>, rect: Rect, style: &WidgetStyle) -> CellStyle {
    let mut text = style.text();
    if let Some(bg) = text.bg {
        cx.fill(rect, bg);
    }
    if let Some(pillar) = style.color("pillar") {
        cx.pillar(rect.x, rect.y, pillar);
    }
    text.bg = None;
    text
}

/// How far the leading content of a row in `states` slides: one cell when hovered or selected
/// and the theme allows sliding.
pub(crate) fn slide(cx: &PaintCx<'_>, states: &[State]) -> u16 {
    let raised = states.contains(&State::Hover) || states.contains(&State::Selected);
    u16::from(raised && cx.env().slide())
}

/// Paints the scrollbar in the last column of `body` when `total` rows overflow it, in the
/// `pinned` style or the theme's.
pub(crate) fn paint_scrollbar(
    cx: &mut PaintCx<'_>,
    body: Rect,
    total: usize,
    offset: usize,
    pinned: Option<ScrollbarStyle>,
) {
    let metrics = ScrollMetrics { total, visible: usize::from(body.height), offset };
    if !metrics.overflows() {
        return;
    }
    let bar = Rect::new(body.right() - 1, body.y, 1, body.height);
    let dragging = cx.memory::<RowScroll>().dragging;
    let active = dragging || cx.pointer().is_some_and(|(x, y)| x == bar.x && y >= bar.y && y < bar.bottom());
    scrollbar::paint(cx, bar, metrics, active, pinned);
}

/// Handles the wheel and the scrollbar of a row area `body` showing `total` rows. Returns
/// whether the event was a scroll interaction; row clicks are left to the widget.
pub(crate) fn scroll_mouse<Msg>(cx: &mut EventCx<'_, Msg>, mouse: &MouseEvent, body: Rect, total: usize) -> bool {
    let offset = cx.memory::<RowScroll>().offset;
    let metrics = ScrollMetrics { total, visible: usize::from(body.height), offset };
    let on_bar = metrics.overflows() && mouse.x == body.right() - 1 && mouse.y >= body.y && mouse.y < body.bottom();
    let memory = cx.memory::<RowScroll>();
    match mouse.kind {
        MouseKind::ScrollUp => {
            memory.offset = memory.offset.saturating_sub(usize::from(WHEEL_ROWS));
            true
        }
        MouseKind::ScrollDown => {
            memory.offset = (memory.offset + usize::from(WHEEL_ROWS)).min(metrics.max_offset());
            true
        }
        MouseKind::Down(MouseButton::Left) if on_bar => {
            memory.dragging = true;
            memory.offset = metrics.offset_at(clamp_u16(mouse.y - body.y), body.height);
            cx.capture_pointer();
            true
        }
        MouseKind::Drag(MouseButton::Left) if memory.dragging => {
            memory.offset = metrics.offset_at(clamp_u16(mouse.y - body.y), body.height);
            true
        }
        MouseKind::Up(MouseButton::Left) if memory.dragging => {
            memory.dragging = false;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_inside_the_rows() {
        assert_eq!(Step::Down.apply(None, 5, 3), Some(0));
        assert_eq!(Step::Up.apply(None, 5, 3), Some(4));
        assert_eq!(Step::Down.apply(Some(4), 5, 3), Some(4));
        assert_eq!(Step::PageDown.apply(Some(1), 5, 3), Some(4));
        assert_eq!(Step::PageUp.apply(Some(4), 5, 3), Some(1));
        assert_eq!(Step::Home.apply(Some(3), 5, 3), Some(0));
        assert_eq!(Step::End.apply(None, 0, 3), None);
    }

    #[test]
    fn follow_scrolls_a_new_selection_into_view_once() {
        let mut scroll = RowScroll::default();
        assert_eq!(scroll.follow(Some(9), 20, 4), 6);
        scroll.offset = 0;
        assert_eq!(scroll.follow(Some(9), 20, 4), 0, "an unchanged selection does not pull the view back");
        assert_eq!(scroll.follow(Some(2), 20, 4), 0);
        scroll.offset = 50;
        assert_eq!(scroll.follow(Some(2), 20, 4), 16);
    }
}
