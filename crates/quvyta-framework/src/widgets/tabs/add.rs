//! The add button of a [`Tabs`] strip: a small `+` control after the tabs, and the hint that says
//! what it does.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::Rect;
use crate::keymap::{Key, Modifiers};
use crate::theme::State;
use crate::widget::{EventCx, PaintCx};

use super::super::placement::Placement;
use super::super::tooltip;
use super::paint::paint_control;
use super::strip::Strip;
use super::{Tabs, TabsMemory};

impl<Msg: 'static> Tabs<Msg> {
    /// Whether keyboard focus inside the strip is on the add button. The strip forgets it once it
    /// loses focus, so coming back always starts on the tabs; a strip with no tabs has only the
    /// button to focus.
    pub(super) fn add_focus_paint(&self, cx: &mut PaintCx<'_>) -> bool {
        if self.on_add.is_none() {
            return false;
        }
        let focused = cx.is_focused();
        let memory = cx.memory::<TabsMemory>();
        if !focused {
            memory.on_add = false;
        }
        focused && (memory.on_add || self.labels.is_empty())
    }

    /// Paints the add button of `strip`, if it has one, as a small control like the arrows: the
    /// `pointer` raises the pillar in its first cell, keyboard focus (`on_add`) breathes it and a
    /// press flashes it. Resting on it for the hover delay, or reaching it with the keyboard, asks
    /// for its hint over `area`.
    pub(super) fn paint_add(
        &self,
        cx: &mut PaintCx<'_>,
        area: Rect,
        strip: &Strip,
        pointer: Option<(i32, i32)>,
        on_add: bool,
    ) {
        let Some(rect) = strip.add else {
            cx.memory::<TabsMemory>().add_hint = None;
            return;
        };
        let now = cx.now();
        let motion = cx.env().theme().motion();
        let hovered = pointer.is_some_and(|(x, y)| rect.contains(x, y));
        let keyboard = on_add && cx.is_focus_visible();
        let mut states = Vec::new();
        if hovered {
            states.push(State::Hover);
        }
        if keyboard {
            states.push(State::Focus);
        }
        if let Some(at) = cx.memory::<TabsMemory>().add_pressed
            && now < at + motion.flash
        {
            states.push(State::Pressed);
            cx.request_frame_in(at + motion.flash - now);
        }
        let glyph = cx.env().icons().glyph("add").into_owned();
        paint_control(cx, "tab-add", rect, &states, &glyph, 1);

        let memory = cx.memory::<TabsMemory>();
        memory.add_hovered_since = if hovered { Some(memory.add_hovered_since.unwrap_or(now)) } else { None };
        let due = memory.add_hovered_since.map(|since| since + motion.hover_delay);
        let visible = keyboard || due.is_some_and(|due| now >= due);
        memory.add_hint_since = if visible { Some(memory.add_hint_since.unwrap_or(now)) } else { None };
        memory.add_hint = visible.then_some(rect);
        if visible {
            cx.request_overlay(area);
        } else if let Some(due) = due {
            cx.request_frame_in(due.saturating_sub(now));
        }
    }

    /// Paints the hint of the add button below it, when it shows. The words come from the
    /// `quvyta.tabs.add` locale key.
    pub(super) fn paint_add_hint(&self, cx: &mut PaintCx<'_>) {
        let memory = cx.memory::<TabsMemory>();
        let (Some(rect), since) = (memory.add_hint, memory.add_hint_since.unwrap_or_default()) else {
            return;
        };
        let text = cx.env().i18n().translate("quvyta.tabs.add", &[]);
        tooltip::paint_tip(cx, rect, &text, Placement::Below, since);
    }

    /// Offers `event` to the add button of `strip`; true when the button used it.
    ///
    /// Tab moves keyboard focus from the tabs to the button and, from there, on out of the strip;
    /// shift+Tab and ← go back to the tabs. Enter and Space press it, and so does a left press on
    /// it. Any other key hands focus back to the tabs, which then act on it.
    pub(super) fn add_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event, strip: &Strip) -> bool {
        let (Some(message), Some(rect)) = (&self.on_add, strip.add) else {
            return false;
        };
        let alone = self.labels.is_empty();
        match event {
            Event::Key(key) => {
                let now = cx.now();
                let memory = cx.memory::<TabsMemory>();
                if !alone && !memory.on_add {
                    if key.is_plain(Key::Tab) {
                        memory.on_add = true;
                        return true;
                    }
                    return false;
                }
                if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
                    memory.add_pressed = Some(now);
                    cx.emit(message());
                    return true;
                }
                if alone {
                    return false;
                }
                let shift = Modifiers { shift: true, ..Modifiers::default() };
                let back = (key.chord.key == Key::Tab && key.chord.mods == shift) || key.is_plain(Key::Left);
                memory.on_add = false;
                back
            }
            Event::Mouse(mouse) => {
                let MouseKind::Down(button) = mouse.kind else {
                    return false;
                };
                if !rect.contains(mouse.x, mouse.y) {
                    cx.memory::<TabsMemory>().on_add = false;
                    return false;
                }
                if button == MouseButton::Left {
                    cx.memory::<TabsMemory>().add_pressed = Some(cx.now());
                    cx.emit(message());
                }
                true
            }
            _ => false,
        }
    }
}
