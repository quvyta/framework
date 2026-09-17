//! Sliders.

use std::time::Duration;

use super::numeric::{Formatter, Steps};
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::motion::Easing;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Cells between the value and the rail.
const GAP: u16 = 2;

/// The fewest rail cells worth drawing.
const MIN_RAIL: u16 = 2;

/// The most cells a keyboard jump animates through; longer jumps move several cells a frame.
const MAX_ANIMATED_CELLS: u32 = 8;

/// Builds a message from a new value.
type ValueMessage<Msg> = Box<dyn Fn(f64) -> Msg>;

/// A value chosen along a range by moving a knob on a rail.
///
/// The value is written before the rail in the accent colour. The done part of the rail is
/// drawn in the accent, the knob `◆` sits at the value and breathes while focused, and the rest
/// of the rail is a quiet raised tone. In ASCII mode the rail is drawn with cell colours instead
/// of glyphs. The widget takes all the width it is given.
///
/// Left and Right move one step, Page Up and Page Down a tenth of the range, Home and End jump
/// to the ends. Pressing the rail jumps the knob there and dragging moves it. The mouse wheel over
/// the slider moves one step, up to increase and down to decrease; the slider keeps the wheel, so
/// a scroll view around it does not scroll while the pointer rests on it, and a disabled slider
/// lets the wheel pass. Keyboard jumps step the knob cell by cell over `motion.step` per cell.
/// The application owns the value.
///
/// Style keys: `slider` (`fill`, `fill-cell`, `track`, `knob`) and `slider-value` (`fg`,
/// `bold`) with states `hover`, `focus`, `disabled`.
pub struct Slider<Msg> {
    value: f64,
    steps: Steps,
    format: Option<Formatter>,
    suffix: String,
    disabled: bool,
    on_change: Option<ValueMessage<Msg>>,
}

#[derive(Debug, Default)]
struct SliderMemory {
    dragging: bool,
    /// Whether the last change came from the pointer, which the knob follows at once.
    from_pointer: bool,
    /// The fraction the knob moved to last, to size the next animation.
    target: Option<f32>,
}

impl<Msg> Slider<Msg> {
    /// A slider from 0 to 100 in steps of 1 showing `value`.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self {
            value,
            steps: Steps::new(0.0, 100.0, 1.0),
            format: None,
            suffix: String::new(),
            disabled: false,
            on_change: None,
        }
    }

    /// The smallest and largest value.
    #[must_use]
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.steps = Steps::new(min, max, self.steps.step);
        self
    }

    /// The distance of one step; values snap to steps from the minimum.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.steps = Steps::new(self.steps.min, self.steps.max, step);
        self
    }

    /// Writes the value with `format` instead of the step's decimals.
    #[must_use]
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Some(Box::new(format));
        self
    }

    /// Text written right after the value, such as `"%"` or `" ms"`.
    #[must_use]
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Greys the slider out; it cannot be focused or moved.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message carrying the new value whenever the knob moves to another step.
    #[must_use]
    pub fn on_change(mut self, message: impl Fn(f64) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(message));
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_change.is_some()
    }

    fn label(&self, value: f64) -> String {
        let written = self.format.as_ref().map_or_else(|| self.steps.write(value), |format| format(value));
        format!("{written}{}", self.suffix)
    }

    /// Width reserved for the value, so the rail does not move while the value changes.
    fn label_width(&self) -> u16 {
        [self.steps.min, self.steps.max, self.value].iter().map(|v| text::width(&self.label(*v))).max().unwrap_or(0)
    }

    /// The rail inside `area`, or nothing when there is no room for one.
    fn rail(&self, area: Rect) -> Option<Rect> {
        let left = self.label_width().saturating_add(GAP);
        let width = area.width.saturating_sub(left);
        (width >= MIN_RAIL).then(|| Rect::new(area.x + i32::from(left), area.y, width, 1))
    }

    /// Moves to `value` when it is another step.
    fn change(&self, cx: &mut EventCx<'_, Msg>, value: f64, from_pointer: bool) {
        cx.memory::<SliderMemory>().from_pointer = from_pointer;
        let value = self.steps.snap(value);
        if value != self.steps.snap(self.value)
            && let Some(message) = &self.on_change
        {
            cx.emit(message(value));
        }
    }

    fn change_at(&self, cx: &mut EventCx<'_, Msg>, x: i32) {
        let Some(rail) = self.rail(cx.area()) else {
            return;
        };
        let cell = (x - rail.x).clamp(0, i32::from(rail.width) - 1);
        let fraction = f64::from(cell) / f64::from(rail.width.saturating_sub(1).max(1));
        self.change(cx, self.steps.at(fraction), true);
    }
}

impl<Msg: 'static> Widget<Msg> for Slider<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(available.width, 1.min(available.height))
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let mut states = if self.active() { cx.pressable_states() } else { Vec::new() };
        if self.disabled {
            states.push(State::Disabled);
        }
        if self.active() {
            cx.register_hit(area);
        }
        let value = self.steps.clamp(self.value);
        let label_style = cx.style("slider-value", None, &states).text();
        let label = self.label(value);
        let label_width = self.label_width();
        let label_x = area.x + i32::from(label_width.saturating_sub(text::width(&label)));
        cx.text(label_x, area.y, &label, label_style, area.width);
        let Some(rail) = self.rail(area) else {
            return;
        };

        let style = cx.style("slider", None, &states);
        let fill = style.color("fill").unwrap_or_else(|| cx.color("accent"));
        let fill_cell = style.color("fill-cell").unwrap_or_else(|| cx.color("active"));
        let track = style.color("track").unwrap_or_else(|| cx.color("raised"));
        let knob = style.color("knob").unwrap_or_else(|| cx.color("accent"));

        let travel = rail.width.saturating_sub(1);
        // Precision beyond f32 is not visible in a terminal cell.
        let fraction = self.steps.fraction(value) as f32;
        let (from_pointer, previous) = {
            let memory = cx.memory::<SliderMemory>();
            (memory.from_pointer, memory.target.replace(fraction))
        };
        let duration = match previous {
            Some(previous) if !from_pointer => {
                let cells = ((fraction - previous).abs() * f32::from(travel)).round() as u32;
                cx.env().theme().motion().step * cells.min(MAX_ANIMATED_CELLS)
            }
            _ => Duration::ZERO,
        };
        let shown = cx.animate("knob", fraction, duration, Easing::Linear);
        let position = crate::motion::steps(shown, travel);

        let rail_glyph = cx.env().icons().glyph("slider-rail").into_owned();
        let knob_glyph = cx.env().icons().glyph("slider-knob").into_owned();
        // A blank glyph (ASCII mode) shows the rail as cell colours instead.
        let cells = rail_glyph.trim().is_empty() || knob_glyph.trim().is_empty();
        for cell in 0..rail.width {
            let x = rail.x + i32::from(cell);
            let (glyph, color, block) = match cell.cmp(&position) {
                std::cmp::Ordering::Less => (&rail_glyph, fill, fill_cell),
                std::cmp::Ordering::Equal => (&knob_glyph, knob, knob),
                std::cmp::Ordering::Greater => (&rail_glyph, track, track),
            };
            if cells {
                cx.clear(Rect::new(x, area.y, 1, 1), block);
            } else {
                cx.text(x, area.y, glyph, CellStyle::fg(color), 1);
            }
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        let value = self.steps.snap(self.value);
        match event {
            Event::Key(key) => {
                let target = if key.is_plain(Key::Left) {
                    self.steps.nudge(value, -1.0)
                } else if key.is_plain(Key::Right) {
                    self.steps.nudge(value, 1.0)
                } else if key.is_plain(Key::PageDown) {
                    self.steps.nudge(value, -self.steps.large())
                } else if key.is_plain(Key::PageUp) {
                    self.steps.nudge(value, self.steps.large())
                } else if key.is_plain(Key::Home) {
                    self.steps.min
                } else if key.is_plain(Key::End) {
                    self.steps.max
                } else {
                    return false;
                };
                self.change(cx, target, false);
                true
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseKind::Down(MouseButton::Left) => {
                    // Pressing the value only focuses; the rail moves the knob.
                    if self.rail(cx.area()).is_some_and(|rail| mouse.x >= rail.x) {
                        cx.capture_pointer();
                        cx.memory::<SliderMemory>().dragging = true;
                        self.change_at(cx, mouse.x);
                    }
                    true
                }
                MouseKind::Drag(MouseButton::Left) if cx.memory::<SliderMemory>().dragging => {
                    self.change_at(cx, mouse.x);
                    true
                }
                MouseKind::Up(MouseButton::Left) => {
                    cx.memory::<SliderMemory>().dragging = false;
                    true
                }
                MouseKind::ScrollUp | MouseKind::ScrollDown => {
                    let direction = if mouse.kind == MouseKind::ScrollUp { 1.0 } else { -1.0 };
                    self.change(cx, self.steps.nudge(value, direction), true);
                    true
                }
                _ => false,
            },
            _ => false,
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

    struct Demo {
        value: f64,
        disabled: bool,
    }

    impl App for Demo {
        type Msg = f64;
        fn update(&mut self, value: f64) -> Command<f64> {
            self.value = value;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, f64>) {
            ui.add(Slider::new(self.value).suffix("%").disabled(self.disabled).on_change(|v| v)).id("volume");
        }
    }

    fn harness(value: f64) -> Harness<Demo> {
        let mut h = Harness::new(Demo { value, disabled: false }, 17, 1);
        h.set_reduced_motion(true);
        h
    }

    #[test]
    fn value_before_a_rail_with_the_knob_at_the_value() {
        let h = harness(50.0);
        // Four cells for "100%", two of gap, eleven of rail; 50% is the middle cell.
        assert_eq!(h.screen(), " 50%  ━━━━━◆━━━━━\n");
        let theme = h.env().theme();
        assert_eq!(h.fg(1, 0), theme.color("accent"));
        assert!(h.is_bold(1, 0));
        assert_eq!(h.fg(6, 0), theme.color("accent"));
        assert_eq!(h.fg(16, 0), theme.color("raised"));
    }

    #[test]
    fn keys_step_jump_and_stop_at_the_ends() {
        let mut h = harness(50.0);
        h.press("tab").press("right");
        assert_eq!(h.app().value, 51.0);
        h.press("pgup");
        assert_eq!(h.app().value, 61.0);
        h.press("end").press("right");
        assert_eq!(h.app().value, 100.0);
        assert_eq!(h.screen(), "100%  ━━━━━━━━━━◆\n");
        h.press("home").press("pgdn");
        assert_eq!(h.app().value, 0.0);
    }

    #[test]
    fn pressing_the_rail_jumps_and_dragging_follows() {
        let mut h = harness(0.0);
        h.click(16, 0);
        assert_eq!(h.app().value, 100.0);
        h.click(1, 0);
        assert_eq!(h.app().value, 100.0, "pressing the value only focuses");
        h.mouse(MouseKind::Down(MouseButton::Left), 11, 0);
        assert_eq!(h.app().value, 50.0);
        // Dragging past the rail keeps following, clamped to the end.
        h.mouse(MouseKind::Drag(MouseButton::Left), 0, 0);
        assert_eq!(h.app().value, 0.0);
        h.mouse(MouseKind::Up(MouseButton::Left), 0, 0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 16, 0);
        assert_eq!(h.app().value, 0.0, "no drag after release");
    }

    #[test]
    fn keyboard_jumps_step_the_knob_cell_by_cell() {
        let mut h = Harness::new(Demo { value: 0.0, disabled: false }, 17, 1);
        h.press("tab").press("end");
        let step = h.env().theme().motion().step;
        assert_eq!(h.screen(), "100%  ◆━━━━━━━━━━\n", "the value is new, the knob has not left yet");
        h.advance(step * 4);
        assert_eq!(h.screen(), "100%  ━━━━━◆━━━━━\n", "halfway after half of the eight steps");
        h.advance(step * 4);
        assert_eq!(h.screen(), "100%  ━━━━━━━━━━◆\n");
    }

    #[test]
    fn ascii_draws_the_rail_with_colours_and_disabled_ignores_input() {
        let mut h = harness(50.0);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), " 50%\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(11, 0), theme.color("accent"));
        assert_ne!(h.bg(6, 0), h.bg(16, 0));
        let mut h = Harness::new(Demo { value: 50.0, disabled: true }, 17, 1);
        h.press("tab").press("right").click(16, 0);
        assert_eq!(h.app().value, 50.0);
        assert_eq!(h.fg(1, 0), theme.color("muted"));
    }

    /// A slider inside a scroll view taller than the screen, as on a settings page.
    struct Scrolled {
        value: f64,
        disabled: bool,
    }

    impl App for Scrolled {
        type Msg = f64;
        fn update(&mut self, value: f64) -> Command<f64> {
            self.value = value;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, f64>) {
            ui.add_with(crate::widgets::ScrollView::new(), |ui| {
                ui.add(Slider::new(self.value).disabled(self.disabled).on_change(|v| v)).id("volume");
                for row in 0..10 {
                    ui.add(crate::widgets::Text::new(format!("row {row}")));
                }
            })
            .fill();
        }
    }

    #[test]
    fn the_wheel_over_the_slider_moves_one_step_and_keeps_the_page_still() {
        let mut h = Harness::new(Scrolled { value: 50.0, disabled: false }, 17, 4);
        h.set_reduced_motion(true);
        h.mouse(MouseKind::ScrollUp, 10, 0);
        assert_eq!(h.app().value, 51.0);
        h.mouse(MouseKind::ScrollDown, 1, 0).mouse(MouseKind::ScrollDown, 1, 0);
        assert_eq!(h.app().value, 49.0, "the value part takes the wheel too");
        assert!(h.screen().starts_with(" 49  "), "the page did not scroll: {}", h.screen());
        assert!(!h.is_focused("volume"), "the wheel does not take focus");
        h.mouse(MouseKind::ScrollDown, 5, 2);
        assert!(!h.screen().contains(" 49 "), "away from the slider the page scrolls: {}", h.screen());
    }

    #[test]
    fn the_wheel_stops_at_the_ends_and_follows_the_step() {
        struct Stepped(f64);
        impl App for Stepped {
            type Msg = f64;
            fn update(&mut self, value: f64) -> Command<f64> {
                self.0 = value;
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, f64>) {
                ui.add(Slider::new(self.0).range(0.0, 10.0).step(2.5).on_change(|v| v));
            }
        }
        let mut h = Harness::new(Stepped(7.5), 20, 1);
        h.mouse(MouseKind::ScrollUp, 12, 0);
        assert_eq!(h.app().0, 10.0);
        h.mouse(MouseKind::ScrollUp, 12, 0);
        assert_eq!(h.app().0, 10.0, "no message past the end");
        h.mouse(MouseKind::ScrollDown, 12, 0);
        assert_eq!(h.app().0, 7.5);
        h.advance(Duration::from_millis(1));
        assert_eq!(h.screen(), " 7.5  ━━━━━━━━━━◆━━━\n", "the knob follows the wheel at once");
    }

    #[test]
    fn a_disabled_slider_lets_the_wheel_scroll_the_page() {
        let mut h = Harness::new(Scrolled { value: 50.0, disabled: true }, 17, 4);
        h.mouse(MouseKind::ScrollUp, 10, 0).mouse(MouseKind::ScrollDown, 10, 0);
        assert_eq!(h.app().value, 50.0);
        h.mouse(MouseKind::ScrollDown, 10, 0);
        assert!(!h.screen().contains(" 50 "), "the wheel scrolled the page instead: {}", h.screen());
    }

    #[test]
    fn an_open_or_broken_range_draws_without_panicking() {
        struct Open(f64, f64);
        impl App for Open {
            type Msg = f64;
            fn update(&mut self, _: f64) -> Command<f64> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, f64>) {
                ui.add(Slider::new(3.0).range(self.0, self.1).on_change(|v| v));
            }
        }
        for (min, max) in [(f64::NEG_INFINITY, 10.0), (f64::NAN, 10.0), (0.0, f64::NAN)] {
            let mut h = Harness::new(Open(min, max), 20, 1);
            h.press("tab").press("right").press("end").advance(Duration::from_millis(500));
            assert!(h.screen().contains('3'), "{min} to {max}: {}", h.screen());
        }
    }

    #[test]
    fn narrow_space_keeps_only_the_value() {
        let h = Harness::new(Demo { value: 7.0, disabled: false }, 6, 1);
        assert_eq!(h.screen(), "  7%\n");
    }
}
