//! Switches.

use super::ToggleMessage;
use super::press::{self, Press};
use crate::color::Rgb;
use crate::env::Env;
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::icons::GlyphMode;
use crate::motion::{Easing, steps};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Width of the capsule and rail switches, in cells.
const TRACK: u16 = 5;

/// Width of the knob, in cells.
const KNOB: u16 = 2;

/// How a [`Switch`] looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitchStyle {
    /// A flat five-cell capsule with a two-cell knob.
    #[default]
    Capsule,
    /// A knob on a thin rail, from the same family as sliders. ASCII mode draws a capsule.
    Rail,
    /// A capsule with the state written inside, for places where the state must be read.
    Labeled,
}

/// An on/off switch that applies at once, with an optional label.
///
/// The knob steps one cell every `motion.step`, and the track and knob colours blend with each
/// step; the knob is always the brightest part. Enter, Space or a click toggles it. The
/// application owns the state.
///
/// Style keys: `switch` (`track`, `track-on`, `knob`, `knob-on`) and `switch-labeled` (`bg`,
/// `fg`, `dot`) with states `hover`, `focus`, `checked`, `disabled`; `switch-label` (`fg`).
/// The labeled style reads its words from `quvyta.switch.on` and `quvyta.switch.off`.
pub struct Switch<Msg> {
    on: bool,
    label: Option<String>,
    style: SwitchStyle,
    disabled: bool,
    on_toggle: Option<ToggleMessage<Msg>>,
}

impl<Msg> Switch<Msg> {
    /// A capsule switch showing `on`.
    #[must_use]
    pub fn new(on: bool) -> Self {
        Self { on, label: None, style: SwitchStyle::Capsule, disabled: false, on_toggle: None }
    }

    /// Text after the switch; clicking it toggles too.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Chooses how the switch looks.
    #[must_use]
    pub fn style(mut self, style: SwitchStyle) -> Self {
        self.style = style;
        self
    }

    /// Greys the switch out; it cannot be focused or toggled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for the new state when the switch is toggled.
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(bool) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(message));
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_toggle.is_some()
    }
}

/// The words of the labeled style in the active language: on, then off.
fn words(env: &Env) -> (String, String) {
    let i18n = env.i18n();
    (i18n.translate("quvyta.switch.on", &[]), i18n.translate("quvyta.switch.off", &[]))
}

impl<Msg> Switch<Msg> {
    fn control_width(&self, on: &str, off: &str) -> u16 {
        match self.style {
            SwitchStyle::Capsule | SwitchStyle::Rail => TRACK,
            // Caps, a space, the longer word, a space, the dot and a space.
            SwitchStyle::Labeled => text::width(on).max(text::width(off)) + 6,
        }
    }

    fn paint_track(&self, cx: &mut PaintCx<'_>, area: Rect, states: &[State]) {
        let style = cx.style("switch", None, states);
        let color = |key: &str, fallback: Rgb| style.color(key).unwrap_or(fallback);
        let (track_off, track_on) = (color("track", cx.color("raised")), color("track-on", cx.color("active")));
        let (knob_off, knob_on) = (color("knob", cx.color("muted")), color("knob-on", cx.color("accent")));
        let travel = TRACK - KNOB;
        let duration = cx.env().theme().motion().step * u32::from(travel);
        let progress = cx.animate("knob", if self.on { 1.0 } else { 0.0 }, duration, Easing::Linear);
        let position = steps(progress, travel);
        // Colours follow the knob's cell, so every in-between frame has an in-between colour.
        let t = f32::from(position) / f32::from(travel);
        let track = track_off.mix(track_on, t);
        let knob = knob_off.mix(knob_on, t);
        let rail = self.style == SwitchStyle::Rail && cx.env().glyph_mode() != GlyphMode::Ascii;
        let rail_glyph = cx.env().icons().glyph("switch-rail").into_owned();
        let knob_glyph = cx.env().icons().glyph("switch-knob").into_owned();
        for cell in 0..TRACK.min(area.width) {
            let x = area.x + i32::from(cell);
            let is_knob = (position..position + KNOB).contains(&cell);
            match (rail, is_knob) {
                (true, true) => {
                    cx.text(x, area.y, &knob_glyph, CellStyle::fg(knob), 1);
                }
                (true, false) => {
                    cx.text(x, area.y, &rail_glyph, CellStyle::fg(track), 1);
                }
                (false, is_knob) => cx.clear(Rect::new(x, area.y, 1, 1), if is_knob { knob } else { track }),
            }
        }
    }

    fn paint_labeled(&self, cx: &mut PaintCx<'_>, area: Rect, states: &[State], width: u16, words: (String, String)) {
        let style = cx.style("switch-labeled", None, states);
        let surface = style.text();
        let bg = surface.bg.unwrap_or_else(|| cx.color("raised"));
        let dot_color = style.color("dot").unwrap_or_else(|| cx.color("muted"));
        let width = width.min(area.width);
        let inner = Rect::new(area.x + 1, area.y, width.saturating_sub(2), 1);
        cx.clear(inner, bg);
        for (key, x) in [("cap-left", area.x), ("cap-right", area.x + i32::from(width) - 1)] {
            let glyph = cx.env().icons().glyph(key).into_owned();
            if glyph.trim().is_empty() {
                cx.clear(Rect::new(x, area.y, 1, 1), bg);
            } else {
                cx.text(x, area.y, &glyph, CellStyle::fg(bg), 1);
            }
        }
        let (on_word, off_word) = words;
        let word_width = inner.width.saturating_sub(4);
        let dot = cx.env().icons().glyph("dot").into_owned();
        let text_style = CellStyle { bg: None, ..surface };
        let (word, word_x, dot_x) =
            if self.on { (on_word, inner.x + 1, inner.right() - 2) } else { (off_word, inner.x + 3, inner.x + 1) };
        cx.text(word_x, area.y, &word, text_style, word_width);
        cx.text(dot_x, area.y, &dot, CellStyle::fg(dot_color), 1);
    }
}

impl<Msg: 'static> Widget<Msg> for Switch<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let (on, off) = words(cx.env());
        let label = self.label.as_deref().map_or(0, |label| text::width(label).saturating_add(2));
        Size::new(self.control_width(&on, &off).saturating_add(label), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let mut states = if self.active() { cx.pressable_states() } else { Vec::new() };
        if self.disabled {
            states.push(State::Disabled);
        }
        if self.on {
            states.push(State::Checked);
        }
        let words = words(cx.env());
        let width = self.control_width(&words.0, &words.1);
        match self.style {
            SwitchStyle::Capsule | SwitchStyle::Rail => self.paint_track(cx, area, &states),
            SwitchStyle::Labeled => self.paint_labeled(cx, area, &states, width, words),
        }
        if let Some(label) = &self.label {
            let label_style = cx.style("switch-label", None, &states).text();
            let budget = area.width.saturating_sub(width + 2);
            let shown = text::truncate(label, budget).into_owned();
            cx.text(area.x + i32::from(width) + 2, area.y, &shown, label_style, budget);
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
                    cx.emit(message(!self.on));
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
    use std::time::Duration;

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo {
        on: bool,
        style: SwitchStyle,
    }

    impl App for Demo {
        type Msg = bool;
        fn update(&mut self, on: bool) -> Command<bool> {
            self.on = on;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, bool>) {
            ui.add(Switch::new(self.on).style(self.style).label("Sounds").on_toggle(|on| on)).id("switch");
        }
    }

    #[test]
    fn knob_steps_across_with_blended_colours_and_stays_brightest() {
        let mut h = Harness::new(Demo { on: false, style: SwitchStyle::Capsule }, 20, 1);
        assert_eq!(h.screen(), "       Sounds\n");
        let (knob_off, track_off) = (h.bg(0, 0), h.bg(2, 0));
        assert_eq!(h.bg(1, 0), knob_off, "the knob is two cells");
        assert_ne!(knob_off, track_off);
        h.set_reduced_motion(true);
        h.send(true);
        let (track_on, knob_on) = (h.bg(0, 0), h.bg(4, 0));
        h.set_reduced_motion(false);
        h.send(false);
        h.send(true);
        let step = h.env().theme().motion().step;
        h.advance(step);
        let middle_knob = h.bg(1, 0);
        assert_ne!(middle_knob, knob_off);
        assert_ne!(middle_knob, knob_on);
        assert_eq!(h.bg(0, 0), h.bg(3, 0), "track colour on both sides of the knob");
        h.advance(step * 3);
        assert_eq!((h.bg(0, 0), h.bg(3, 0), h.bg(4, 0)), (track_on, knob_on, knob_on));
        let luminance = |color: Option<Rgb>| color.map_or(0.0, Rgb::relative_luminance);
        assert!(luminance(knob_on) > luminance(track_on), "the lit knob stays brightest");
    }

    struct Disabled;

    impl App for Disabled {
        type Msg = bool;
        fn update(&mut self, _: bool) -> Command<bool> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, bool>) {
            ui.add(Switch::new(false).disabled(true).on_toggle(|on| on));
        }
    }

    #[test]
    fn a_disabled_off_knob_stays_visible_on_its_track_in_every_theme() {
        for theme in ["monochrome", "iris", "nordic", "amber"] {
            let mut h = Harness::new(Disabled, 10, 1);
            h.set_theme(theme);
            let ratio = h.bg(0, 0).zip(h.bg(4, 0)).map_or(1.0, |(knob, track)| knob.contrast_ratio(track));
            assert!(ratio >= 1.2, "{theme}: disabled knob reads at {ratio:.2}:1 on its track");
        }
    }

    /// A column of switches (on, off, on, on): an off knob must never read like the
    /// lit track beside an on knob, and hovering must visibly change both states, in every theme.
    #[test]
    fn off_and_on_read_apart_and_hover_lifts_both_in_every_theme() {
        let luminance = |color: Option<Rgb>| color.map_or(0.0, Rgb::relative_luminance);
        let contrast = |a: Option<Rgb>, b: Option<Rgb>| a.zip(b).map_or(1.0, |(a, b)| a.contrast_ratio(b));
        for theme in ["monochrome", "iris", "nordic", "amber"] {
            let mut off = Harness::new(Demo { on: false, style: SwitchStyle::Capsule }, 20, 1);
            let mut on = Harness::new(Demo { on: true, style: SwitchStyle::Capsule }, 20, 1);
            for h in [&mut off, &mut on] {
                h.set_theme(theme);
                h.set_reduced_motion(true);
                h.hover(12, 0);
                h.hover(19, 0);
            }
            let (off_knob, off_track, on_track, on_knob) = (off.bg(0, 0), off.bg(4, 0), on.bg(0, 0), on.bg(4, 0));
            assert!(contrast(off_knob, on_track) >= 1.6, "{theme}: an off knob looks like an on track");
            assert!(luminance(off_knob) < luminance(on_track), "{theme}: off is quieter than on");
            assert!(contrast(off_knob, off_track) >= 1.3, "{theme}: the off knob shows on its track");
            assert!(contrast(on_knob, on_track) >= 1.5, "{theme}: the on knob shows on its track");
            for (h, name) in [(&mut off, "off"), (&mut on, "on")] {
                let rest = (h.bg(0, 0), h.bg(4, 0));
                h.hover(0, 0);
                let hovered = (h.bg(0, 0), h.bg(4, 0));
                assert!(hovered.0 != rest.0 && hovered.1 != rest.1, "{theme}: hovering an {name} switch changes it");
                assert!(luminance(hovered.0) > luminance(rest.0), "{theme}: hover lifts an {name} switch");
            }
        }
    }

    #[test]
    fn reduced_motion_jumps_and_keyboard_toggles() {
        let mut h = Harness::new(Demo { on: false, style: SwitchStyle::Capsule }, 20, 1);
        h.set_reduced_motion(true);
        h.press("tab").press("enter");
        assert!(h.app().on);
        assert_eq!(h.bg(4, 0), h.env().theme().color("accent"));
    }

    #[test]
    fn rail_and_labeled_styles() {
        let mut h = Harness::new(Demo { on: true, style: SwitchStyle::Rail }, 24, 1);
        assert_eq!(h.screen(), "━━━██  Sounds\n");
        h.set_glyph_mode(GlyphMode::Ascii);
        assert!(h.screen().is_ascii());
        let mut h = Harness::new(Demo { on: true, style: SwitchStyle::Labeled }, 24, 1);
        assert_eq!(h.screen(), "▐ ON  ● ▌  Sounds\n");
        h.click_text("Sounds");
        h.advance(Duration::from_millis(10));
        assert_eq!(h.screen(), "▐ ● OFF ▌  Sounds\n");
    }
}
