//! Radio groups.

use std::time::Duration;

use super::IndexMessage;
use super::cells;
use super::checkbox::{self, BOX, LABEL_GAP};
use super::press::{self, Press};
use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::motion::{Easing, Tween};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Cells between options laid out in a row.
const ROW_GAP: u16 = 4;

/// How many `motion.step`s a mark takes to change between small and full.
const MARK_STEPS: u32 = 2;

/// The icon of the mark style's small size, two cells.
const MARK_ICON: &str = "radio-mark-small";

/// Time between frames while a mark grows or shrinks.
const MARK_FRAME: Duration = Duration::from_millis(16);

/// How the options of a [`RadioGroup`] are marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RadioStyle {
    /// A small square centred in two cells for every option; only its colour tells the chosen one.
    /// Choosing blends the new square from the quiet tone to the chosen tone over two
    /// `motion.step`s while the option left behind blends back; the shape never changes. The
    /// square is the icon `radio-mark-small`; a blank icon (as in ASCII) draws a box whose tone
    /// blends the same way.
    #[default]
    Square,
    /// A small square centred in two cells for the options not chosen and a full two-cell box of
    /// solid colour for the chosen one. Choosing blends the mark's colour towards the chosen tone
    /// and swaps the small square for the full box halfway, with no size in between; the option
    /// left behind blends back and shrinks the same way. The small square is the icon
    /// `radio-mark-small`; a blank icon (as in ASCII) draws a faint box instead.
    Mark,
    /// A two-cell box of solid colour, filled for the chosen option and the empty tone for the
    /// others; exactly the box of a checkbox. No glyph, so it looks the same in every glyph mode.
    Box,
    /// A filled dot `●` for the chosen option and a ring `○` for the others.
    Dot,
}

impl RadioStyle {
    /// Width of the mark, in cells.
    fn width(self) -> u16 {
        match self {
            Self::Square | Self::Mark | Self::Box => BOX,
            Self::Dot => 1,
        }
    }
}

/// A set of options of which exactly one is chosen, for a few choices that should all stay
/// visible. For many choices use a `Select`.
///
/// The group takes focus as one control: arrow keys choose the previous or next option, Home
/// and End the first and last, and a click anywhere on an option, mark or label, chooses it.
/// By default ([`RadioStyle::Square`]) every option shows a small square centred in two cells and
/// the chosen one differs by colour: choosing blends the new square towards the chosen tone over
/// two `motion.step`s while the old one blends back. [`RadioStyle::Mark`] also grows the chosen
/// square into a full two-cell box, swapping shape halfway through the same blend. With reduced
/// motion the change is immediate. [`RadioStyle::Box`] gives every option the checkbox's
/// box and [`RadioStyle::Dot`] a dot or a ring. Nothing is bracketed, and every style keeps two
/// cells (one for dots) in every state, so labels never move. The application owns the choice.
///
/// The box style looks exactly like a checkbox: what differs is the meaning. A radio group
/// chooses one option and a checkbox turns each option on or off by itself.
///
/// Style keys: `radio.mark` (`fg`, also the colour of the full box) for marks, whose blank icons
/// take their tones from `radio.box` (`bg`); `radio.box` (`bg`) for boxes; `radio` (`fg`) for
/// dots; `radio-label` (`fg`, `bold`); all with states `hover`, `focus`, `checked`, `disabled`.
/// Icons: `radio-mark-small` for marks, `dot` and `dot-outline` for dots.
pub struct RadioGroup<Msg> {
    options: Vec<String>,
    selected: Option<usize>,
    horizontal: bool,
    style: RadioStyle,
    disabled: bool,
    on_select: Option<IndexMessage<Msg>>,
}

impl<Msg> RadioGroup<Msg> {
    /// A vertical group of `options` with nothing chosen.
    #[must_use]
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: None,
            horizontal: false,
            style: RadioStyle::default(),
            disabled: false,
            on_select: None,
        }
    }

    /// The chosen option.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Lays the options out in one row instead of one per line.
    #[must_use]
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Chooses how options are marked.
    #[must_use]
    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = style;
        self
    }

    /// Greys the group out; it cannot be focused or changed.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for choosing option `index`.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_select.is_some() && !self.options.is_empty()
    }

    /// Where option `index` sits inside `area`.
    fn slot(&self, area: Rect, index: usize) -> Rect {
        let width = |label: &str| self.label_offset().saturating_add(text::width(label));
        if self.horizontal {
            let x = cells::sum(self.options[..index].iter().map(|label| width(label).saturating_add(ROW_GAP)));
            Rect::new(area.x + i32::from(x), area.y, width(&self.options[index]), 1)
        } else {
            let row = i32::try_from(index).unwrap_or(i32::MAX);
            Rect::new(area.x, area.y.saturating_add(row), area.width, 1)
        }
    }

    /// Where labels start, after the mark and its gap.
    fn label_offset(&self) -> u16 {
        self.style.width() + LABEL_GAP
    }

    fn choose(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        let index = index.min(self.options.len() - 1);
        if Some(index) != self.selected
            && let Some(message) = &self.on_select
        {
            cx.emit(message(index));
        }
    }
}

/// How far each option's mark has grown, by index: 0 small, 1 full.
#[derive(Debug, Default)]
struct MarkGrowth(Vec<Tween>);

/// How far option `index`'s mark has moved towards full (`chosen`) or small, from 0 to 1, over
/// two `motion.step`s. A mark starts where it belongs without animating.
fn mark_progress(cx: &mut PaintCx<'_>, index: usize, chosen: bool) -> f32 {
    let target = if chosen { 1.0 } else { 0.0 };
    if cx.reduced_motion() {
        return target;
    }
    let now = cx.now();
    let duration = cx.env().theme().motion().step * MARK_STEPS;
    let growth = &mut cx.memory::<MarkGrowth>().0;
    while growth.len() <= index {
        growth.push(Tween::settled(target));
    }
    let tween = &mut growth[index];
    if tween.target() != target {
        tween.retarget(target, now, duration, Easing::Linear);
    }
    let (progress, running) = (tween.value(now), tween.is_running(now));
    if running {
        cx.request_frame_in(MARK_FRAME);
    }
    progress
}

/// Paints the two-cell mark of option `index` at `(x, y)`. With `grow` only two shapes exist, the
/// small square and the full box; without it the square keeps its shape. The colour mixes the quiet tone (the `radio.mark` `fg` of the states without
/// `checked`) into the chosen tone (with `checked`) as the mark moves, and the shape changes once,
/// halfway. A blank small icon draws a box instead, mixing the `radio.box` empty tone the same way.
fn paint_mark(cx: &mut PaintCx<'_>, at: (i32, i32), states: &[State], index: usize, chosen: bool, grow: bool) {
    let t = mark_progress(cx, index, chosen);
    let mut calm: Vec<State> = states.iter().copied().filter(|s| *s != State::Checked).collect();
    let quiet = cx.style("radio", Some("mark"), &calm).text().fg.unwrap_or_else(|| cx.color("muted"));
    let empty = cx.style("radio", Some("box"), &calm).text().bg.unwrap_or_else(|| cx.color("raised"));
    calm.push(State::Checked);
    let full = cx.style("radio", Some("mark"), &calm).text().fg.unwrap_or_else(|| cx.color("accent"));
    let cells = Rect::new(at.0, at.1, BOX, 1);
    let glyph = cx.env().icons().glyph(MARK_ICON).into_owned();
    if glyph.trim().is_empty() {
        cx.clear(cells, empty.mix(full, t));
        return;
    }
    let colour = quiet.mix(full, t);
    if grow && t >= 0.5 {
        cx.clear(cells, colour);
        return;
    }
    // Whatever an icon override draws, the mark keeps its two cells so the label never moves.
    let mut shown = text::truncate(&glyph, BOX).into_owned();
    for _ in text::width(&shown)..BOX {
        shown.push(' ');
    }
    cx.text(at.0, at.1, &shown, CellStyle::fg(colour), BOX);
}

impl<Msg: 'static> Widget<Msg> for RadioGroup<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let widths = self.options.iter().map(|label| self.label_offset().saturating_add(text::width(label)));
        let size = if self.horizontal {
            let count = u16::try_from(self.options.len()).unwrap_or(u16::MAX);
            Size::new(cells::sum(widths).saturating_add(ROW_GAP.saturating_mul(count.saturating_sub(1))), 1)
        } else {
            Size::new(widths.max().unwrap_or(0), u16::try_from(self.options.len()).unwrap_or(u16::MAX))
        };
        size.min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let active = self.active();
        let focused = active && cx.is_focus_visible();
        let pointer = if active { cx.pointer() } else { None };
        let (on, off) =
            (cx.env().icons().glyph("dot").into_owned(), cx.env().icons().glyph("dot-outline").into_owned());
        for (index, label) in self.options.iter().enumerate() {
            let slot = self.slot(area, index).intersect(area);
            if slot.is_empty() {
                continue;
            }
            let chosen = self.selected == Some(index);
            let mut states = Vec::new();
            if pointer.is_some_and(|(x, y)| slot.contains(x, y)) {
                states.push(State::Hover);
            }
            // With nothing chosen yet, focus rests on the first option.
            if focused && (chosen || (self.selected.is_none() && index == 0)) {
                states.push(State::Focus);
            }
            if chosen {
                states.push(State::Checked);
            }
            if self.disabled {
                states.push(State::Disabled);
            }
            match self.style {
                RadioStyle::Square => paint_mark(cx, (slot.x, slot.y), &states, index, chosen, false),
                RadioStyle::Mark => paint_mark(cx, (slot.x, slot.y), &states, index, chosen, true),
                RadioStyle::Box => {
                    let fill = if chosen { 1.0 } else { 0.0 };
                    checkbox::paint_box(cx, (slot.x, slot.y), ("radio", Some("box")), &states, (index, [fill; 2]));
                }
                RadioStyle::Dot => {
                    let mark = cx.style("radio", None, &states).text();
                    cx.text(slot.x, slot.y, if chosen { &on } else { &off }, CellStyle { bg: None, ..mark }, 1);
                }
            }
            let label_style = cx.style("radio-label", None, &states).text();
            let offset = self.label_offset();
            let budget = slot.width.saturating_sub(offset);
            let shown = text::truncate(label, budget).into_owned();
            cx.text(slot.x + i32::from(offset), slot.y, &shown, label_style, budget);
        }
        if active {
            cx.register_hit(area);
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        let last = self.options.len() - 1;
        if let Event::Key(key) = event {
            let (back, forward) = if self.horizontal { (Key::Left, Key::Right) } else { (Key::Up, Key::Down) };
            let current = self.selected;
            let target = if key.is_plain(back) {
                Some(current.map_or(0, |i| i.saturating_sub(1)))
            } else if key.is_plain(forward) {
                Some(current.map_or(0, |i| (i + 1).min(last)))
            } else if key.is_plain(Key::Home) {
                Some(0)
            } else if key.is_plain(Key::End) {
                Some(last)
            } else {
                None
            };
            if let Some(index) = target {
                self.choose(cx, index);
                return true;
            }
        }
        match press::read(cx, event) {
            Press::Ignored => false,
            Press::Used => true,
            Press::Key => {
                self.choose(cx, self.selected.unwrap_or(0));
                true
            }
            Press::Click(x, y) => {
                let area = cx.area();
                if let Some(index) = (0..self.options.len()).find(|&i| self.slot(area, i).contains(x, y)) {
                    self.choose(cx, index);
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
    use crate::color::Rgb;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo {
        chosen: Option<usize>,
        horizontal: bool,
        style: RadioStyle,
    }

    impl App for Demo {
        type Msg = usize;
        fn update(&mut self, index: usize) -> Command<usize> {
            self.chosen = Some(index);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, usize>) {
            ui.add(
                RadioGroup::new(["Podman", "Docker", "Nerdctl"])
                    .selected(self.chosen)
                    .horizontal(self.horizontal)
                    .style(self.style)
                    .on_select(|i| i),
            )
            .id("engine");
        }
    }

    fn demo(chosen: Option<usize>, horizontal: bool, style: RadioStyle) -> Demo {
        Demo { chosen, horizontal, style }
    }

    #[test]
    fn vertical_group_chooses_by_arrows_and_clicks() {
        let mut h = Harness::new(demo(None, false, RadioStyle::Box), 20, 3);
        h.set_reduced_motion(true);
        assert_eq!(h.screen(), "    Podman\n    Docker\n    Nerdctl\n", "boxes are colour, not glyphs");
        h.press("tab").press("down");
        assert_eq!(h.app().chosen, Some(0));
        h.press("down").press("end");
        assert_eq!(h.app().chosen, Some(2));
        h.click_text("Docker");
        assert_eq!(h.app().chosen, Some(1));
        let theme = h.env().theme().clone();
        h.hover(19, 2);
        assert_eq!((h.bg(0, 0), h.bg(1, 0)), (theme.color("raised"), theme.color("raised")));
        assert_eq!(
            (h.bg(0, 1), h.bg(1, 1)),
            (theme.color("accent"), theme.color("accent")),
            "the chosen box is filled"
        );
    }

    #[test]
    fn horizontal_group_uses_left_and_right() {
        let mut h = Harness::new(demo(Some(1), true, RadioStyle::Box), 40, 1);
        assert_eq!(h.screen(), "    Podman        Docker        Nerdctl\n");
        h.press("tab").press("right");
        assert_eq!(h.app().chosen, Some(2));
        h.click_text("Podman");
        assert_eq!(h.app().chosen, Some(0));
    }

    #[test]
    fn every_cell_of_an_option_chooses_it_and_the_gap_between_does_not() {
        let mut h = Harness::new(demo(None, true, RadioStyle::Box), 40, 1);
        for x in [14, 15, 17, 21] {
            h.click(x, 0);
            assert_eq!(h.app().chosen, Some(1), "column {x} is part of Docker");
            h.click(1, 0);
            assert_eq!(h.app().chosen, Some(0));
        }
        h.click(11, 0);
        assert_eq!(h.app().chosen, Some(0), "the gap between options chooses nothing");
    }

    #[test]
    fn hover_lightens_one_empty_box_and_keyboard_focus_tints_the_chosen_one() {
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Box), 20, 3);
        let theme = h.env().theme().clone();
        h.hover(6, 2);
        assert_eq!(h.bg(0, 2), theme.color("active"), "the hovered empty box lightens");
        assert_eq!(h.bg(0, 1), theme.color("raised"), "the others stay calm");
        h.hover(19, 0).press("tab");
        assert_ne!(h.bg(0, 0), theme.color("accent"), "keyboard focus breathes on the chosen box");
        assert_eq!(h.screen().matches('▌').count(), 0, "a radio group shows no pillar");
    }

    #[test]
    fn choosing_blends_the_old_box_out_and_the_new_box_in() {
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Box), 20, 3);
        let theme = h.env().theme().clone();
        let step = theme.motion().step;
        let (empty, filled) = (theme.color("raised").expect("raised"), theme.color("accent").expect("accent"));
        h.hover(19, 2);
        h.send(2);
        assert_eq!((h.bg(0, 0), h.bg(0, 2)), (Some(filled), Some(empty)), "the change starts where it was");
        h.advance(step * 3 / 2);
        let (old, new) = (h.bg(0, 0).expect("colour"), h.bg(0, 2).expect("colour"));
        assert!(old != filled && old != empty && new != filled && new != empty, "both are in between");
        assert!(old.r.abs_diff(new.r) <= 3, "halfway both boxes are the middle colour: {old:?} {new:?}");
        h.advance(step * 2);
        assert_eq!((h.bg(0, 0), h.bg(0, 2)), (Some(empty), Some(filled)));
        h.set_reduced_motion(true);
        h.send(1);
        assert_eq!((h.bg(0, 1), h.bg(0, 2)), (Some(filled), Some(empty)), "reduced motion switches at once");
    }

    #[test]
    fn the_box_is_the_checkbox_box_in_every_theme_and_ascii_changes_nothing() {
        struct Both;
        impl App for Both {
            type Msg = usize;
            fn update(&mut self, _: usize) -> Command<usize> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, usize>) {
                ui.add(RadioGroup::new(["On", "Off"]).style(RadioStyle::Box).selected(Some(0)).on_select(|i| i));
                ui.add(crate::widgets::Checkbox::new(true).label("On").on_toggle(|_| 0));
                ui.add(crate::widgets::Checkbox::new(false).label("Off").on_toggle(|_| 0));
            }
        }
        let mut h = Harness::new(Both, 20, 4);
        for theme in ["monochrome", "iris", "nordic", "amber"] {
            h.set_theme(theme);
            // Rows 0 and 2 are chosen and checked, rows 1 and 3 empty; at rest and under the pointer.
            for (radio_row, box_row) in [(0_u16, 2_u16), (1, 3)] {
                for hovered in [false, true] {
                    let x = if hovered { 1 } else { 19 };
                    h.hover(x, i32::from(radio_row));
                    let radio = (h.bg(0, radio_row), h.bg(1, radio_row));
                    h.hover(x, i32::from(box_row));
                    let checkbox = (h.bg(0, box_row), h.bg(1, box_row));
                    assert_eq!(checkbox, radio, "{theme}, row {radio_row}, hovered {hovered}");
                }
            }
        }
        let unicode = (h.screen(), h.bg(0, 0), h.bg(0, 1));
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert_eq!((h.screen(), h.bg(0, 0), h.bg(0, 1)), unicode);
    }

    #[test]
    fn the_dot_style_keeps_dots_and_rings() {
        let mut h = Harness::new(demo(Some(1), false, RadioStyle::Dot), 20, 3);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        assert_eq!(h.screen(), "○  Podman\n●  Docker\n○  Nerdctl\n");
        assert_eq!(h.fg(0, 1), h.env().theme().color("accent"));
        let mut h = Harness::new(demo(Some(1), true, RadioStyle::Dot), 40, 1);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        assert_eq!(h.screen(), "○  Podman    ●  Docker    ○  Nerdctl\n");
        h.click_text("Nerdctl");
        assert_eq!(h.app().chosen, Some(2));
    }

    #[test]
    fn disabled_boxes_use_disabled_tones_and_ignore_input() {
        struct Off;
        impl App for Off {
            type Msg = usize;
            fn update(&mut self, _: usize) -> Command<usize> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, usize>) {
                ui.add(
                    RadioGroup::new(["Podman", "Docker"])
                        .style(RadioStyle::Box)
                        .selected(Some(0))
                        .disabled(true)
                        .on_select(|i| i),
                );
            }
        }
        let mut h = Harness::new(Off, 20, 2);
        let theme = h.env().theme().clone();
        assert_eq!((h.bg(0, 0), h.bg(0, 1)), (theme.color("active"), theme.color("raised")));
        assert_eq!(h.fg(4, 1), theme.color("muted"));
        h.click_text("Docker").press("tab").press("down");
        assert_eq!(h.screen(), "    Podman\n    Docker\n");
    }

    /// The `fg` of `radio.mark` in `states`, at the start of any pulse.
    fn mark_tone(theme: &crate::theme::Theme, states: &[State]) -> Rgb {
        theme.style("radio", Some("mark"), states).paint("fg").expect("radio.mark has an fg").at(0.0)
    }

    #[test]
    fn the_mark_style_is_a_small_square_and_the_chosen_option_a_full_box() {
        let mut h = Harness::new(demo(Some(1), false, RadioStyle::Mark), 20, 3);
        assert_eq!(h.screen(), "🬇🬃  Podman\n    Docker\n🬇🬃  Nerdctl\n");
        let theme = h.env().theme().clone();
        let (quiet, full) = (Some(mark_tone(&theme, &[])), theme.color("accent"));
        assert_eq!((h.fg(0, 0), h.fg(1, 0), h.fg(0, 2)), (quiet, quiet, quiet), "unchosen squares are quiet");
        assert_eq!((h.bg(0, 0), h.bg(1, 0)), (h.bg(4, 0), h.bg(4, 0)), "a square has no box behind it");
        assert_eq!((h.bg(0, 1), h.bg(1, 1)), (full, full), "the chosen option is a full box");
        h.set_glyph_mode(crate::icons::GlyphMode::Nerd);
        assert_eq!(h.screen(), "🬇🬃  Podman\n    Docker\n🬇🬃  Nerdctl\n", "Nerd Font mode draws the same sextants");
        let mut h = Harness::new(demo(Some(1), true, RadioStyle::Mark), 40, 1);
        assert_eq!(h.screen(), "🬇🬃  Podman        Docker    🬇🬃  Nerdctl\n");
        h.click_text("Nerdctl");
        assert_eq!(h.app().chosen, Some(2));
    }

    #[test]
    fn the_default_square_keeps_its_shape_and_the_chosen_option_blends_to_the_chosen_colour() {
        assert_eq!(RadioStyle::default(), RadioStyle::Square);
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Square), 20, 3);
        assert_eq!(h.screen(), "🬇🬃  Podman\n🬇🬃  Docker\n🬇🬃  Nerdctl\n", "every option is the same square");
        let theme = h.env().theme().clone();
        let step = theme.motion().step;
        let (quiet, full) = (mark_tone(&theme, &[]), theme.color("accent").expect("accent"));
        assert_eq!((h.fg(0, 0), h.fg(0, 2)), (Some(full), Some(quiet)), "the chosen square has the chosen colour");
        assert_eq!(h.bg(0, 0), h.bg(4, 0), "and no box behind it");
        let between = |c: Option<Rgb>| c.is_some_and(|c| c.r > quiet.r && c.r < full.r);
        h.send(2);
        for _ in 0..8 {
            h.advance(step / 4);
            assert_eq!(h.screen(), "🬇🬃  Podman\n🬇🬃  Docker\n🬇🬃  Nerdctl\n", "the shape never changes");
        }
        h.send(0);
        h.advance(step);
        assert!(between(h.fg(0, 0)) && between(h.fg(0, 2)), "halfway both squares have a blended colour");
        h.advance(step);
        assert_eq!((h.fg(0, 0), h.fg(0, 2)), (Some(full), Some(quiet)));
        h.set_reduced_motion(true);
        h.send(1);
        assert_eq!((h.fg(0, 1), h.fg(0, 0)), (Some(full), Some(quiet)), "reduced motion changes at once");
    }

    #[test]
    fn a_group_without_a_style_draws_squares_and_no_full_box() {
        struct Plain;
        impl App for Plain {
            type Msg = usize;
            fn update(&mut self, _: usize) -> Command<usize> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, usize>) {
                ui.add(RadioGroup::new(["Podman", "Docker", "Nerdctl"]).selected(Some(1)).on_select(|i| i));
            }
        }
        let h = Harness::new(Plain, 20, 3);
        assert_eq!(h.screen(), "🬇🬃  Podman\n🬇🬃  Docker\n🬇🬃  Nerdctl\n", "the chosen option keeps its square");
        let accent = h.env().theme().color("accent");
        assert_eq!(h.fg(0, 1), accent, "the chosen square has the chosen colour");
        assert_eq!((h.bg(0, 1), h.bg(1, 1)), (h.bg(4, 1), h.bg(4, 1)), "no full box behind it");
    }

    /// Screen rows and colours of the first and third options while the mark moves from one to the
    /// other.
    fn frame(h: &Harness<Demo>) -> (String, [Option<Rgb>; 4]) {
        let rows: Vec<String> = h.screen().lines().map(|line| line.chars().take(2).collect()).collect();
        (format!("{}|{}", rows[0], rows[2]), [h.fg(0, 0), h.bg(0, 0), h.fg(0, 2), h.bg(0, 2)])
    }

    #[test]
    fn choosing_blends_the_colours_and_swaps_square_and_box_halfway_with_no_size_between() {
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Mark), 20, 3);
        let theme = h.env().theme().clone();
        let step = theme.motion().step;
        let (quiet, full) = (mark_tone(&theme, &[]), theme.color("accent").expect("accent"));
        let ground = h.bg(10, 0);
        let one_ms = Duration::from_millis(1);
        let expect = |a: &str, b: &str| format!("{a}|{b}");
        // Strictly between the quiet and the chosen tone, by red channel (both are greys here).
        let between = |c: Option<Rgb>| c.is_some_and(|c| c.r > quiet.r && c.r < full.r);
        h.send(2);
        let (rows, colours) = frame(&h);
        assert_eq!(rows, expect("  ", "🬇🬃"), "the change starts where it was");
        assert_eq!((colours[1], colours[2], colours[3]), (Some(full), Some(quiet), ground));
        h.advance(step / 2);
        let (rows, colours) = frame(&h);
        assert_eq!(rows, expect("  ", "🬇🬃"), "before halfway both keep their shape");
        assert!(between(colours[1]) && between(colours[2]), "while their colours blend: {colours:?}");
        h.advance(step / 2 - one_ms);
        assert_eq!(frame(&h).0, expect("  ", "🬇🬃"));
        h.advance(one_ms * 2);
        let (rows, colours) = frame(&h);
        assert_eq!(rows, expect("🬇🬃", "  "), "halfway the shapes swap, with no size in between");
        assert!(between(colours[0]) && between(colours[3]), "and the colours keep blending: {colours:?}");
        h.advance(step);
        let (rows, colours) = frame(&h);
        assert_eq!(rows, expect("🬇🬃", "  "));
        assert_eq!((colours[0], colours[1], colours[3]), (Some(quiet), ground, Some(full)));
        for _ in 0..8 {
            h.advance(step / 4);
            let rows = frame(&h).0;
            assert!(!rows.contains('▐') && !rows.contains('▌'), "no medium size ever: {rows}");
        }

        // Back again: the same two shapes, swapping halfway.
        h.send(0);
        h.advance(step - one_ms);
        assert_eq!(frame(&h).0, expect("🬇🬃", "  "));
        h.advance(one_ms * 2);
        assert_eq!(frame(&h).0, expect("  ", "🬇🬃"));
        h.advance(step);
        let (rows, colours) = frame(&h);
        assert_eq!(rows, expect("  ", "🬇🬃"));
        assert_eq!((colours[1], colours[2], colours[3]), (Some(full), Some(quiet), ground));
        assert_eq!(h.screen().lines().nth(1), Some("🬇🬃  Docker"), "the untouched option never moved");
    }

    #[test]
    fn reduced_motion_swaps_the_marks_at_once() {
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Mark), 20, 3);
        h.set_reduced_motion(true);
        h.send(2);
        assert_eq!(h.screen(), "🬇🬃  Podman\n🬇🬃  Docker\n    Nerdctl\n");
        assert_eq!(h.bg(1, 2), h.env().theme().color("accent"));
    }

    #[test]
    fn ascii_marks_fall_back_to_boxes_that_blend_from_faint_to_full() {
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Mark), 20, 3);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        let theme = h.env().theme().clone();
        let (faint, full) = (theme.color("raised").expect("raised"), theme.color("accent").expect("accent"));
        assert_eq!(h.screen(), "    Podman\n    Docker\n    Nerdctl\n");
        assert_eq!((h.bg(0, 0), h.bg(1, 1), h.bg(0, 2)), (Some(full), Some(faint), Some(faint)));
        h.send(2);
        h.advance(theme.motion().step);
        let middle = faint.mix(full, 0.5);
        assert_eq!((h.bg(1, 0), h.bg(0, 2)), (Some(middle), Some(middle)), "halfway both boxes have the middle tone");
        assert_eq!(h.screen(), "    Podman\n    Docker\n    Nerdctl\n", "labels stay in place");
        h.advance(theme.motion().step);
        assert_eq!((h.bg(0, 0), h.bg(1, 2)), (Some(faint), Some(full)));
        h.hover(6, 1);
        assert_eq!(h.bg(0, 1), theme.color("active"), "hover lifts the faint box like the box style");
    }

    #[test]
    fn a_theme_can_replace_the_mark_glyphs() {
        let dir = std::env::temp_dir().join(format!("quvyta-radio-mark-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let theme = "[meta]\nname = \"Plain marks\"\nextends = \"monochrome\"\n\n[icons]\n\
                     radio-mark-small = { nerd = \"•\", unicode = \"•\", ascii = \".\" }\n";
        std::fs::write(dir.join("plain-marks.toml"), theme).expect("theme file");
        let dirs = crate::env::AssetDirs { themes: Some(dir.clone()), ..Default::default() };
        let env = crate::env::Env::load(&dirs).expect("loads");
        std::fs::remove_dir_all(&dir).ok();
        assert!(env.diagnostics().is_empty(), "{:?}", env.diagnostics());
        let mut h = Harness::with_env(demo(Some(0), false, RadioStyle::Mark), env, 20, 3);
        h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
        h.set_theme("plain-marks");
        assert_eq!(h.screen(), "    Podman\n•   Docker\n•   Nerdctl\n", "a narrower glyph keeps two cells");
        h.send(1);
        h.advance(h.env().theme().motion().step * 2);
        assert_eq!(h.screen(), "•   Podman\n    Docker\n•   Nerdctl\n");
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert_eq!(h.screen(), ".   Podman\n    Docker\n.   Nerdctl\n");
    }

    #[test]
    fn hover_lifts_a_square_focus_warms_it_and_the_chosen_box_follows_the_box_tones() {
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Mark), 20, 3);
        let theme = h.env().theme().clone();
        h.hover(6, 2);
        assert_eq!(h.fg(0, 2), Some(mark_tone(&theme, &[State::Hover])), "the hovered square lifts");
        assert_eq!(h.fg(0, 1), Some(mark_tone(&theme, &[])), "the others stay calm");
        h.hover(6, 0);
        assert_eq!(h.bg(0, 0), Some(mark_tone(&theme, &[State::Checked, State::Hover])));
        assert_eq!(
            h.bg(0, 0),
            theme.style("radio", Some("box"), &[State::Checked, State::Hover]).paint("bg").map(|p| p.at(0.0))
        );
        h.hover(19, 5).press("tab");
        assert_ne!(h.bg(0, 0), theme.color("accent"), "keyboard focus breathes on the chosen box");
        assert_eq!(h.screen().matches('▌').count(), 0, "a radio group shows no pillar");

        let mut h = Harness::new(demo(None, false, RadioStyle::Mark), 20, 3);
        h.press("tab");
        assert_eq!(h.fg(0, 0), Some(mark_tone(&theme, &[State::Focus])), "focus rests on the first square");
        assert_ne!(h.fg(0, 0), Some(mark_tone(&theme, &[])));
        assert_eq!(h.fg(0, 1), Some(mark_tone(&theme, &[])));
        h.click(6, 1);
        assert_eq!(h.fg(0, 0), Some(mark_tone(&theme, &[])), "a pointer focus shows no focus tone");
    }

    #[test]
    fn disabled_marks_use_disabled_tones_and_ignore_input() {
        struct Off;
        impl App for Off {
            type Msg = usize;
            fn update(&mut self, _: usize) -> Command<usize> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, usize>) {
                ui.add(
                    RadioGroup::new(["Podman", "Docker"])
                        .style(RadioStyle::Mark)
                        .selected(Some(0))
                        .disabled(true)
                        .on_select(|i| i),
                );
            }
        }
        let mut h = Harness::new(Off, 20, 2);
        let theme = h.env().theme().clone();
        assert_eq!(h.screen(), "    Podman\n🬇🬃  Docker\n");
        assert_eq!(h.bg(0, 0), Some(mark_tone(&theme, &[State::Checked, State::Disabled])));
        assert_eq!(h.bg(0, 0), theme.color("active"));
        assert_eq!(h.fg(0, 1), Some(mark_tone(&theme, &[State::Disabled])));
        assert_eq!(h.fg(4, 1), theme.color("muted"));
        h.hover(6, 1);
        assert_eq!(h.fg(0, 1), Some(mark_tone(&theme, &[State::Disabled])), "hover changes nothing");
        h.click_text("Docker").press("tab").press("down");
        assert_eq!(h.screen(), "    Podman\n🬇🬃  Docker\n");
    }

    /// Marks sit on the canvas or a panel surface. In every theme a square must show on both, stay
    /// quieter than the chosen box, lift under the pointer, and stay visible when disabled.
    #[test]
    fn marks_read_in_every_theme() {
        let mut h = Harness::new(demo(Some(0), false, RadioStyle::Mark), 20, 3);
        for id in ["monochrome", "iris", "nordic", "amber"] {
            h.set_theme(id);
            let theme = h.env().theme().clone();
            let color = |name: &str| theme.color(name).expect("token");
            let (quiet, hover, full) =
                (mark_tone(&theme, &[]), mark_tone(&theme, &[State::Hover]), mark_tone(&theme, &[State::Checked]));
            let disabled = mark_tone(&theme, &[State::Disabled]);
            for ground in ["canvas", "surface"] {
                let ground = color(ground);
                assert!(
                    quiet.contrast_ratio(ground) >= 2.0,
                    "{id}: a square reads {:.2}:1",
                    quiet.contrast_ratio(ground)
                );
                assert!(
                    disabled.contrast_ratio(ground) >= 1.4,
                    "{id}: a disabled square reads {:.2}:1",
                    disabled.contrast_ratio(ground)
                );
            }
            assert!(hover.relative_luminance() > quiet.relative_luminance(), "{id}: hover lifts the square");
            assert!(
                full.relative_luminance() > hover.relative_luminance() && full.contrast_ratio(hover) >= 1.25,
                "{id}: a hovered square never looks chosen ({:.2}:1)",
                full.contrast_ratio(hover)
            );
            assert!(
                full.contrast_ratio(quiet) >= 1.5,
                "{id}: the chosen box stands apart from a square in tone as well as size"
            );
            assert!(disabled.relative_luminance() < quiet.relative_luminance(), "{id}: disabled is quieter");
        }
    }
}
