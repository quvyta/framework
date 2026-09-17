//! The side panel's icon strip: an activity bar whose icons switch, open and close the panel's
//! views.

use std::rc::Rc;

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::Side;

/// Width of the strip: three cells for an icon row (the pillar and a two-cell icon) and one spare
/// column on the panel's side, which is the edge while the panel is collapsed.
pub(super) const STRIP: u16 = 4;

/// Builds a message from a view index.
pub(super) type ViewMessage<Msg> = Rc<dyn Fn(u16) -> Msg>;

/// Builds a message from a new open state.
pub(super) type OpenMessage<Msg> = Rc<dyn Fn(bool) -> Msg>;

/// The icon column of a [`SidePanel`](super::SidePanel), one row every other line.
pub(super) struct Strip<Msg> {
    pub(super) side: Side,
    pub(super) icons: Vec<String>,
    /// The view the panel shows, if the application said.
    pub(super) active: Option<u16>,
    pub(super) open: bool,
    pub(super) on_select: ViewMessage<Msg>,
    pub(super) on_toggle: Option<OpenMessage<Msg>>,
}

#[derive(Debug, Default)]
struct StripMemory {
    /// The strip as last painted.
    rect: Rect,
    /// The row the keyboard is on while the strip has focus.
    cursor: Option<usize>,
}

impl<Msg: 'static> Strip<Msg> {
    /// The row of icon `index` in a strip painted into `rect`, while it fits.
    fn row(&self, rect: Rect, index: usize) -> Option<Rect> {
        let y = rect.y + 1 + i32::try_from(index * 2).ok()?;
        // The spare column faces the panel, so the rows keep to the outer three cells.
        let x = match self.side {
            Side::Left => rect.x,
            Side::Right => rect.x + 1,
        };
        (index < self.icons.len() && y < rect.bottom()).then(|| Rect::new(x, y, rect.width.min(STRIP - 1), 1))
    }

    /// The icon under `(x, y)`.
    fn index_at(&self, rect: Rect, x: i32, y: i32) -> Option<usize> {
        (0..self.icons.len()).find(|index| self.row(rect, *index).is_some_and(|row| row.contains(x, y)))
    }

    /// The index shown as the panel's view: only while the panel is open.
    fn selected(&self) -> Option<usize> {
        self.active.filter(|_| self.open).map(usize::from)
    }

    /// Acts on icon `index` like an activity bar: the shown view's icon closes the panel, any
    /// other icon switches to its view, opening the panel if it is closed.
    fn activate(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        if self.selected() == Some(index)
            && let Some(toggle) = &self.on_toggle
        {
            cx.emit(toggle(false));
            return;
        }
        cx.emit((self.on_select)(u16::try_from(index).unwrap_or(u16::MAX)));
    }
}

impl<Msg: 'static> Widget<Msg> for Strip<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let background = cx.style("side-strip", None, &[]).text().bg.unwrap_or_else(|| cx.color("surface"));
        cx.clear(rect, background);
        cx.register_hit(rect);
        let pointer = cx.pointer();
        let focused = cx.is_focused();
        let focus_visible = cx.is_focus_visible();
        let flashing = cx.is_pressed();
        let last = self.icons.len().saturating_sub(1);
        let cursor = {
            let memory = cx.memory::<StripMemory>();
            memory.rect = rect;
            // Tab lands on the shown view's icon, or the first one.
            memory.cursor = focused.then(|| memory.cursor.or(self.selected()).unwrap_or(0).min(last));
            memory.cursor
        };
        for (index, icon) in self.icons.iter().enumerate() {
            let Some(row) = self.row(rect, index) else { break };
            let mut states = Vec::new();
            if pointer.is_some_and(|(x, y)| row.contains(x, y)) {
                states.push(State::Hover);
            }
            if self.selected() == Some(index) {
                states.push(State::Selected);
            }
            // The keyboard's row breathes only when focus came from the keyboard, like every
            // pressable; a clicked strip stays calm.
            if focus_visible && cursor == Some(index) {
                states.push(State::Focus);
                if flashing {
                    states.push(State::Pressed);
                }
            }
            let look = cx.style("side-strip-item", None, &states);
            let style = look.text();
            if let Some(bg) = style.bg {
                cx.clear(row, bg);
            }
            if let Some(pillar) = look.color("pillar") {
                cx.pillar(row.x, row.y, pillar);
            }
            let glyph = cx.env().icons().glyph(icon).into_owned();
            cx.text(row.x + 1, row.y, &glyph, CellStyle { bg: None, ..style }, 2);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        match event {
            Event::Key(key) if cx.is_focused() && !self.icons.is_empty() => {
                let last = self.icons.len() - 1;
                let memory = cx.memory::<StripMemory>();
                let cursor = memory.cursor.unwrap_or(0).min(last);
                let moved = if key.is_plain(Key::Up) {
                    cursor.saturating_sub(1)
                } else if key.is_plain(Key::Down) {
                    (cursor + 1).min(last)
                } else if key.is_plain(Key::Home) {
                    0
                } else if key.is_plain(Key::End) {
                    last
                } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                    cx.flash();
                    self.activate(cx, cursor);
                    return true;
                } else {
                    return false;
                };
                memory.cursor = Some(moved);
                true
            }
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                let memory = cx.memory::<StripMemory>();
                let Some(index) = self.index_at(memory.rect, mouse.x, mouse.y) else {
                    return false;
                };
                memory.cursor = Some(index);
                self.activate(cx, index);
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        true
    }
}
