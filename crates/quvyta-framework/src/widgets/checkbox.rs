//! Checkboxes, and the two-cell choice box they share with radio groups.

use std::time::Duration;

use super::ToggleMessage;
use super::press::{self, Press};
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::motion::{Easing, Tween};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Width of the choice box, in cells.
pub(super) const BOX: u16 = 2;

/// Width of the check style's box, in cells.
const CHECK_BOX: u16 = 3;

/// Cells between a mark and its label.
pub(super) const LABEL_GAP: u16 = 2;

/// How many `motion.step`s a choice box takes to blend between empty and filled: as long as a
/// switch knob takes to cross its track, so the toggles of a form change together.
const BLEND_STEPS: u32 = 3;

/// Time between frames while a box blends.
const BLEND_FRAME: Duration = Duration::from_millis(16);

/// How a [`Checkbox`] looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckboxStyle {
    /// A two-cell box of solid colour: filled when checked, the empty tone when not, and filled
    /// in its left cell only when partly checked. No glyph, so it looks the same in every glyph
    /// mode. It looks exactly like a radio group's box.
    #[default]
    Box,
    /// A three-cell box with a check `✓` when checked and a dash when partly checked.
    Check,
}

/// A box that is checked or not, with an optional label.
///
/// The default box is two cells of solid colour that blend from the empty tone to the filled
/// tone over the theme's `motion.step` times three when it changes; with reduced motion the
/// change is immediate. A partly checked box (a parent of some checked items) fills its left
/// cell. [`CheckboxStyle::Check`] draws the older three-cell box with a check or a dash. Enter,
/// Space or a click on the box or the label toggles it. The application owns the state.
///
/// Style keys: `checkbox` (`bg`, `fg`) with states `hover`, `focus`, `checked`, `disabled` and
/// variant `partial` (check style); `checkbox-label` (`fg`, `bold`) with the same states.
pub struct Checkbox<Msg> {
    label: Option<String>,
    checked: bool,
    partial: bool,
    style: CheckboxStyle,
    disabled: bool,
    on_toggle: Option<ToggleMessage<Msg>>,
}

impl<Msg> Checkbox<Msg> {
    /// A checkbox showing `checked`.
    #[must_use]
    pub fn new(checked: bool) -> Self {
        Self { label: None, checked, partial: false, style: CheckboxStyle::Box, disabled: false, on_toggle: None }
    }

    /// Text after the box; clicking it toggles too.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shows the box partly checked. Toggling a partly checked box asks for `true`.
    #[must_use]
    pub fn partial(mut self, partial: bool) -> Self {
        self.partial = partial;
        self
    }

    /// Chooses how the box looks.
    #[must_use]
    pub fn style(mut self, style: CheckboxStyle) -> Self {
        self.style = style;
        self
    }

    /// Greys the checkbox out; it cannot be focused or toggled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for the new state when the checkbox is toggled.
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(message));
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_toggle.is_some()
    }

    fn box_width(&self) -> u16 {
        match self.style {
            CheckboxStyle::Box => BOX,
            CheckboxStyle::Check => CHECK_BOX,
        }
    }
}

/// The blend of every choice box a widget draws, by index.
#[derive(Debug, Default)]
struct BoxBlends(Vec<Tween>);

/// How filled choice box `index` of this widget is, moving towards `target` (0 empty, 1 filled).
/// A box starts at its first target without animating.
fn blend(cx: &mut PaintCx<'_>, index: usize, target: f32) -> f32 {
    if cx.reduced_motion() {
        return target;
    }
    let now = cx.now();
    let duration = cx.env().theme().motion().step * BLEND_STEPS;
    let blends = &mut cx.memory::<BoxBlends>().0;
    if blends.len() <= index {
        blends.resize(index + 1, Tween::settled(target));
    }
    let tween = &mut blends[index];
    if tween.target() != target {
        tween.retarget(target, now, duration, Easing::Linear);
    }
    let (value, running) = (tween.value(now), tween.is_running(now));
    if running {
        cx.request_frame_in(BLEND_FRAME);
    }
    value
}

/// Paints a two-cell choice box at `(x, y)` from the `bg` of `widget`/`variant`: each cell mixes
/// the empty colour (the states without `checked`) into the filled colour (with `checked`) by its
/// fill. Blends are kept per `index` so a group of boxes animates each one on its own.
pub(super) fn paint_box(
    cx: &mut PaintCx<'_>,
    at: (i32, i32),
    style: (&str, Option<&str>),
    states: &[State],
    blends: (usize, [f32; 2]),
) {
    let (widget, variant) = style;
    let mut empty_states: Vec<State> = states.iter().copied().filter(|s| *s != State::Checked).collect();
    let empty = cx.style(widget, variant, &empty_states).text().bg.unwrap_or_else(|| cx.color("raised"));
    empty_states.push(State::Checked);
    let filled = cx.style(widget, variant, &empty_states).text().bg.unwrap_or_else(|| cx.color("accent"));
    let (index, targets) = blends;
    for (cell, target) in (0..BOX).zip(targets) {
        let fill = blend(cx, index * usize::from(BOX) + usize::from(cell), target);
        cx.clear(Rect::new(at.0 + i32::from(cell), at.1, 1, 1), empty.mix(filled, fill));
    }
}

impl<Msg: 'static> Widget<Msg> for Checkbox<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let label = self.label.as_deref().map_or(0, |label| text::width(label).saturating_add(LABEL_GAP));
        Size::new(self.box_width().saturating_add(label), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let mut states = if self.active() { cx.pressable_states() } else { Vec::new() };
        if self.disabled {
            states.push(State::Disabled);
        }
        if self.checked || self.partial {
            states.push(State::Checked);
        }
        let box_width = self.box_width();
        match self.style {
            CheckboxStyle::Box => {
                let left = if self.checked || self.partial { 1.0 } else { 0.0 };
                let right = if self.checked { 1.0 } else { 0.0 };
                paint_box(cx, (area.x, area.y), ("checkbox", None), &states, (0, [left, right]));
            }
            CheckboxStyle::Check => {
                let variant = (self.partial && !self.checked).then_some("partial");
                let style = cx.style("checkbox", variant, &states).text();
                let background = style.bg.unwrap_or_else(|| cx.color("raised"));
                cx.clear(Rect::new(area.x, area.y, box_width.min(area.width), 1), background);
                if self.checked || self.partial {
                    let key = if self.checked { "check" } else { "check-partial" };
                    let glyph = cx.env().icons().glyph(key).into_owned();
                    cx.text(area.x + 1, area.y, &glyph, CellStyle { bg: None, ..style }, 1);
                }
            }
        }
        if let Some(label) = &self.label {
            let label_style = cx.style("checkbox-label", None, &states).text();
            let budget = area.width.saturating_sub(box_width + LABEL_GAP);
            let shown = text::truncate(label, budget).into_owned();
            cx.text(area.x + i32::from(box_width + LABEL_GAP), area.y, &shown, label_style, budget);
        }
        if self.active() {
            cx.register_hit(area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        match press::read(cx, event) {
            Press::Ignored => false,
            Press::Used => true,
            Press::Key | Press::Click(..) => {
                if let Some(message) = &self.on_toggle {
                    cx.emit(message(!self.checked || self.partial));
                }
                true
            }
        }
    }

    fn focusable(&self) -> bool {
        self.active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    #[derive(Default)]
    struct Demo {
        checked: bool,
        partial: bool,
        style: CheckboxStyle,
        disabled: bool,
    }

    impl App for Demo {
        type Msg = bool;
        fn update(&mut self, on: bool) -> Command<bool> {
            self.checked = on;
            self.partial = false;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, bool>) {
            ui.add(
                Checkbox::new(self.checked)
                    .partial(self.partial)
                    .style(self.style)
                    .disabled(self.disabled)
                    .label("Autosave")
                    .on_toggle(|on| on),
            )
            .id("box");
        }
    }

    #[test]
    fn the_default_box_is_two_cells_of_colour_that_fill_when_checked() {
        let mut h = Harness::new(Demo::default(), 20, 1);
        h.set_reduced_motion(true);
        assert_eq!(h.screen(), "    Autosave\n", "no glyph, no bracket");
        let theme = h.env().theme().clone();
        assert_eq!(
            (h.bg(0, 0), h.bg(1, 0), h.bg(2, 0)),
            (theme.color("raised"), theme.color("raised"), theme.color("canvas"))
        );
        h.click_text("Autosave");
        assert!(h.app().checked);
        h.hover(19, 0);
        assert_eq!(h.screen(), "    Autosave\n", "a checkbox shows no pillar");
        assert_eq!((h.bg(0, 0), h.bg(1, 0)), (theme.color("accent"), theme.color("accent")));
        h.press("tab").press("space");
        assert!(!h.app().checked, "the keyboard toggles too");
    }

    #[test]
    fn the_whole_label_area_toggles() {
        let mut h = Harness::new(Demo::default(), 20, 1);
        for x in [0, 1, 2, 3, 6, 11] {
            let before = h.app().checked;
            h.click(x, 0);
            assert_ne!(h.app().checked, before, "a click at column {x} toggles");
        }
    }

    #[test]
    fn hover_lightens_the_empty_box_and_keyboard_focus_tints_it() {
        let mut h = Harness::new(Demo::default(), 20, 1);
        let theme = h.env().theme().clone();
        let rest = h.bg(0, 0);
        h.hover(6, 0);
        assert_eq!(h.bg(0, 0), theme.color("active"));
        assert_ne!(h.bg(0, 0), rest);
        h.hover(19, 0);
        assert_eq!(h.bg(0, 0), rest);
        h.hover(40, 0).press("tab");
        assert_ne!(h.bg(0, 0), rest, "keyboard focus shows on the box");
        assert_ne!(h.bg(0, 0), theme.color("accent"), "focus is a tint, not the checked fill");
    }

    #[test]
    fn checking_blends_the_colour_over_three_steps_and_reduced_motion_jumps() {
        let mut h = Harness::new(Demo::default(), 20, 1);
        let theme = h.env().theme().clone();
        let step = theme.motion().step;
        h.hover(40, 0);
        h.send(true);
        let (empty, filled) = (theme.color("raised").expect("raised"), theme.color("accent").expect("accent"));
        assert_eq!(h.bg(0, 0), Some(empty), "the change starts from the empty tone");
        h.advance(step * 3 / 2);
        let middle = empty.mix(filled, 0.5);
        let shown = h.bg(0, 0).expect("colour");
        let close = |a: u8, b: u8| a.abs_diff(b) <= 3;
        assert!(close(shown.r, middle.r) && close(shown.g, middle.g), "halfway is the middle colour: {shown:?}");
        assert_eq!(h.bg(0, 0), h.bg(1, 0), "both cells blend together");
        h.advance(step * 2);
        assert_eq!(h.bg(0, 0), Some(filled));
        h.set_reduced_motion(true);
        h.send(false);
        assert_eq!(h.bg(1, 0), Some(empty), "reduced motion empties at once");
    }

    #[test]
    fn partial_fills_the_left_cell_and_asks_for_checked() {
        let mut h = Harness::new(Demo { partial: true, ..Demo::default() }, 20, 1);
        let theme = h.env().theme().clone();
        assert_eq!(h.screen(), "    Autosave\n");
        assert_eq!((h.bg(0, 0), h.bg(1, 0)), (theme.color("accent"), theme.color("raised")));
        h.click_text("Autosave");
        assert!(h.app().checked);
    }

    #[test]
    fn ascii_mode_draws_the_same_box() {
        let mut h = Harness::new(Demo { checked: true, ..Demo::default() }, 20, 1);
        let unicode = (h.screen(), h.bg(0, 0), h.bg(1, 0));
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!((h.screen(), h.bg(0, 0), h.bg(1, 0)), unicode);
    }

    #[test]
    fn disabled_uses_the_disabled_tones_and_ignores_presses() {
        let mut h = Harness::new(Demo { checked: true, disabled: true, ..Demo::default() }, 20, 1);
        let theme = h.env().theme().clone();
        assert_eq!(h.bg(0, 0), theme.color("active"));
        assert_eq!(h.fg(4, 0), theme.color("muted"));
        h.click_text("Autosave").press("tab").press("space");
        assert!(h.app().checked);
        let h = Harness::new(Demo { disabled: true, ..Demo::default() }, 20, 1);
        assert_eq!(h.bg(0, 0), theme.color("raised"));
    }

    #[test]
    fn the_check_style_keeps_the_three_cell_box_with_a_mark() {
        let mut h = Harness::new(Demo { style: CheckboxStyle::Check, ..Demo::default() }, 20, 1);
        h.set_glyph_mode(GlyphMode::Unicode);
        assert_eq!(h.screen(), "     Autosave\n");
        assert_eq!(h.bg(1, 0), h.env().theme().color("raised"));
        h.click_text("Autosave");
        assert_eq!(h.screen(), " ✓   Autosave\n");
        let mut h = Harness::new(Demo { style: CheckboxStyle::Check, partial: true, ..Demo::default() }, 20, 1);
        h.set_glyph_mode(GlyphMode::Unicode);
        assert_eq!(h.screen(), " –   Autosave\n");
    }

    #[test]
    fn a_narrow_space_keeps_the_box_and_cuts_the_label() {
        let h = Harness::new(Demo::default(), 8, 1);
        assert_eq!(h.screen(), "    Aut…\n");
    }
}
