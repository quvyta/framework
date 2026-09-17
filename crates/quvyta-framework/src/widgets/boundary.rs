//! The draggable boundary between two areas: the one model behind [`Splitter`](super::Splitter)
//! and [`SidePanel`](super::SidePanel).
//!
//! A boundary is never drawn as a line. It is a one-cell strip that looks like whatever lies
//! under it until the pointer reaches it: then it brightens one step, while dragging it takes
//! the accent, and while it has focus reached with the keyboard it carries a faint accent tint.
//!
//! A boundary can carry a [`Toggle`] at its middle: a two-cell button, the pillar cell and an
//! arrow, that appears while the boundary is pointed at or reached with the keyboard. It climbs
//! the tone ladder of every pressable thing: shown < hovered < pressed. The pillar `▌` itself
//! shows only while the pointer is on the button or keyboard focus is visible; pointing at the
//! rest of the edge leaves the first cell in the button's tone.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::theme::State;
use crate::widget::{EventCx, PaintCx};

/// Which way a boundary moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    /// A column between areas side by side; it moves left and right.
    Columns,
    /// A row between stacked areas; it moves up and down.
    Rows,
}

impl Axis {
    /// The pointer coordinate along the axis the boundary moves on.
    fn along(self, x: i32, y: i32) -> i32 {
        match self {
            Self::Columns => x,
            Self::Rows => y,
        }
    }
}

/// The button a boundary carries at its middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Toggle<'a> {
    /// Icon key of the arrow.
    pub(crate) arrow: &'a str,
    /// Whether the button reaches into the cell after the handle rather than the one before it.
    /// The pillar is always the left cell and the arrow the right one, so the button takes the
    /// handle and the cell after it, or the cell before the handle and the handle.
    pub(crate) reach_after: bool,
}

#[derive(Debug, Default)]
struct BoundaryMemory {
    /// The handle as last painted.
    rect: Rect,
    /// The toggle's two cells as last painted, while it is shown.
    toggle: Option<Rect>,
    /// Pointer went down on the handle at this coordinate along the axis.
    pressed: Option<i32>,
    /// The press went down on the toggle.
    pressed_toggle: bool,
    dragging: bool,
}

/// What an event did to the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// Not about the boundary; let it bubble.
    Ignored,
    /// Used without a result, such as the button going down.
    Used,
    /// The pointer dragged the boundary to this coordinate along the axis.
    DragTo(i32),
    /// The keyboard moved the boundary by this many cells (positive is right or down).
    Nudge(i32),
    /// The keyboard jumped to the start (`false`) or the end (`true`).
    Jump(bool),
    /// The toggle was clicked, or Enter or Space pressed while focused.
    Activate,
}

/// The two cells of the toggle on a column handle `rect`: on its middle row, reaching one cell
/// past the handle on the side `toggle` says.
fn toggle_rect(rect: Rect, toggle: Toggle<'_>) -> Rect {
    let y = rect.y + i32::from(rect.height / 2);
    let x = if toggle.reach_after { rect.x } else { rect.x - 1 };
    Rect::new(x, y, 2, 1)
}

/// Paints the handle into `rect`, registers it for the pointer and remembers it for events.
/// `toggle` is the button shown at its middle while the handle is pointed at, dragged or reached
/// with the keyboard.
///
/// Style keys: `split-handle` (`bg`, `fg`) with `hover`, `focus` and `active` (dragging), and
/// `side-toggle` (`bg`, `fg`, `pillar`) with `hover`, `focus` and `pressed`.
pub(crate) fn paint(cx: &mut PaintCx<'_>, rect: Rect, toggle: Option<Toggle<'_>>) {
    if rect.is_empty() {
        return;
    }
    cx.register_hit(rect);
    let pointer = cx.pointer();
    let hovered = pointer.is_some_and(|(x, y)| rect.contains(x, y));
    let (dragging, held_on_toggle) = {
        let memory = cx.memory::<BoundaryMemory>();
        (memory.dragging, memory.pressed.is_some() && memory.pressed_toggle)
    };
    let focus_visible = cx.is_focus_visible();
    let mut states = Vec::new();
    if hovered {
        states.push(State::Hover);
    }
    // Only focus reached with the keyboard tints the edge: after a click or a drag the edge must
    // not stay behind as a coloured line.
    if focus_visible {
        states.push(State::Focus);
    }
    if dragging {
        states.push(State::Active);
    }
    let style = cx.style("split-handle", None, &states).text();
    if let Some(bg) = style.bg {
        cx.fill(rect, bg);
    }
    let mut painted = None;
    if let Some(toggle) = toggle {
        let button = toggle_rect(rect, toggle);
        let on_button = pointer.is_some_and(|(x, y)| button.contains(x, y));
        if hovered || on_button || dragging || held_on_toggle || focus_visible {
            let mut states = Vec::new();
            if on_button {
                states.push(State::Hover);
            }
            if focus_visible {
                states.push(State::Focus);
            }
            // Held down on the toggle, or flashing right after it toggled.
            if (held_on_toggle && on_button) || cx.is_pressed() {
                states.push(State::Pressed);
            }
            let look = cx.style("side-toggle", None, &states);
            let text = look.text();
            if let Some(bg) = text.bg {
                cx.fill(button, bg);
            }
            // The pillar marks the button being pointed at or reached with the keyboard, like
            // every pressable; the lit edge alone only raises the button's two cells.
            if (on_button || focus_visible)
                && let Some(pillar) = look.color("pillar")
            {
                cx.pillar(button.x, button.y, pillar);
            }
            let glyph = cx.env().icons().glyph(toggle.arrow).into_owned();
            cx.text(button.x + 1, button.y, &glyph, CellStyle { bg: None, ..text }, 1);
            // The toggle reaches one cell past the handle, so it is a press target of its own.
            cx.register_hit(button);
            painted = Some(button);
        }
    }
    let memory = cx.memory::<BoundaryMemory>();
    memory.rect = rect;
    memory.toggle = painted;
}

/// Reads `event` for the boundary of a widget whose keyboard focus is the handle.
pub(crate) fn event<Msg>(cx: &mut EventCx<'_, Msg>, event: &Event, axis: Axis) -> Change {
    match event {
        Event::Key(key) if cx.is_focused() => {
            let (back, forward) = match axis {
                Axis::Columns => (Key::Left, Key::Right),
                Axis::Rows => (Key::Up, Key::Down),
            };
            let big = crate::keymap::Modifiers { shift: true, ..Default::default() };
            if key.chord.key == back && (key.chord.mods == big || key.is_plain(back)) {
                Change::Nudge(if key.chord.mods == big { -5 } else { -1 })
            } else if key.chord.key == forward && (key.chord.mods == big || key.is_plain(forward)) {
                Change::Nudge(if key.chord.mods == big { 5 } else { 1 })
            } else if key.is_plain(Key::Home) {
                Change::Jump(false)
            } else if key.is_plain(Key::End) {
                Change::Jump(true)
            } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                cx.flash();
                Change::Activate
            } else {
                Change::Ignored
            }
        }
        Event::Mouse(mouse) => {
            let at = axis.along(mouse.x, mouse.y);
            let memory = cx.memory::<BoundaryMemory>();
            let on_toggle = memory.toggle.is_some_and(|button| button.contains(mouse.x, mouse.y));
            match mouse.kind {
                MouseKind::Down(MouseButton::Left) if on_toggle || memory.rect.contains(mouse.x, mouse.y) => {
                    memory.pressed = Some(at);
                    memory.pressed_toggle = on_toggle;
                    cx.capture_pointer();
                    Change::Used
                }
                MouseKind::Drag(MouseButton::Left) if memory.pressed.is_some() => {
                    // A press on the toggle is a button press: sliding across its two cells must
                    // not resize.
                    if !memory.pressed_toggle && (memory.dragging || memory.pressed != Some(at)) {
                        memory.dragging = true;
                        Change::DragTo(at)
                    } else {
                        Change::Used
                    }
                }
                MouseKind::Up(MouseButton::Left) if memory.pressed.is_some() => {
                    let clicked = !memory.dragging && memory.pressed_toggle && on_toggle;
                    memory.pressed = None;
                    memory.pressed_toggle = false;
                    memory.dragging = false;
                    if clicked {
                        cx.flash();
                        Change::Activate
                    } else {
                        Change::Used
                    }
                }
                _ => Change::Ignored,
            }
        }
        _ => Change::Ignored,
    }
}

/// Moves `size` to `target` within `min..=max`, where `max` never drops below `min`.
pub(crate) fn clamp_size(target: i32, min: u16, max: u16) -> u16 {
    clamp_u16(target).clamp(min, max.max(min))
}
