//! Number entry.

use super::cells;
use super::edit_menu;
use super::numeric::Steps;
use super::text_input::TextInput;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Width of one stepper segment: the sign with a cell of room on each side.
const SEGMENT: u16 = 3;

/// Steps in a Page Up or Page Down.
const LARGE_STEP: f64 = 10.0;

/// Cells a field without a range leaves for its number.
const DEFAULT_DIGITS: u16 = 8;

/// Builds a message from a new value.
type ValueMessage<Msg> = Box<dyn Fn(f64) -> Msg>;

/// A text field for a number: typing edits it like a text input, Up and Down step it.
///
/// Only digits can be typed, a minus when the range allows negative numbers and a decimal
/// point when the step has decimals. The field shows the invalid state while its text is not a
/// number or lies outside the range, and the application only ever receives valid numbers
/// inside the range; an empty or unfinished field keeps the last valid value. Up and Down move
/// one step and Page Up and Page Down ten, always clamped to the range. Everything else edits
/// like a [`TextInput`](crate::widgets::TextInput): cursor, selection, undo and clipboard.
///
/// With [`NumberInput::steppers`] two segments on the right step the value when clicked. The
/// wheel over the field moves one step per notch, and a right click opens the text input's edit
/// menu (Cut, Copy, Paste, Select all); pasted text keeps only the characters a number allows.
///
/// Style keys: those of the text input (`text-input`, `text-input-prompt`, …) and
/// `number-input-stepper` (`bg`, `fg`) with states `hover`, `focus`, `pressed`, `disabled`.
pub struct NumberInput<Msg> {
    value: f64,
    steps: Steps,
    steppers: bool,
    placeholder: String,
    invalid: bool,
    disabled: bool,
    on_change: Option<ValueMessage<Msg>>,
}

#[derive(Debug, Default)]
struct NumberMemory {
    /// The text being edited, which may be unfinished such as `-` or `2.`.
    draft: String,
    /// The value the draft was last written from or sent as.
    seen: Option<f64>,
    /// The decimal separator the draft was written with, so a language changed while the field
    /// stands there rewrites it instead of leaving the old language's point on screen.
    separator: Option<char>,
    /// The stepper segment that was clicked last: 0 down, 1 up.
    pressed: Option<usize>,
}

impl<Msg: 'static> NumberInput<Msg> {
    /// A field showing `value`, without limits, stepping by 1.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self {
            value,
            steps: Steps::new(f64::NEG_INFINITY, f64::INFINITY, 1.0),
            steppers: false,
            placeholder: String::new(),
            invalid: false,
            disabled: false,
            on_change: None,
        }
    }

    /// The smallest and largest allowed value.
    #[must_use]
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.steps = Steps::new(min, max, self.steps.step);
        self
    }

    /// How far Up and Down move the value. The step's decimals decide how the value is written
    /// and whether a decimal point can be typed.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        self.steps = Steps::new(self.steps.min, self.steps.max, step);
        self
    }

    /// Shows clickable minus and plus segments on the right.
    #[must_use]
    pub fn steppers(mut self, steppers: bool) -> Self {
        self.steppers = steppers;
        self
    }

    /// Faint text shown while the field is empty.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Marks the value as failing the application's own validation.
    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Makes the field read-only and unfocusable.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message carrying the new value after every change to a valid number.
    #[must_use]
    pub fn on_change(mut self, message: impl Fn(f64) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(message));
        self
    }

    /// The draft, rewritten from the value when the application changed it or the language did.
    fn draft(&self, memory: &mut NumberMemory, separator: char) -> String {
        if memory.seen != Some(self.value) || memory.separator != Some(separator) {
            memory.draft = self.steps.write_with(self.value, separator);
            memory.seen = Some(self.value);
            memory.separator = Some(separator);
        }
        memory.draft.clone()
    }

    /// The number in `draft` when it is one inside the range.
    fn parse(&self, draft: &str, separator: char) -> Option<f64> {
        let value = read(draft, separator)?;
        (value >= self.steps.min && value <= self.steps.max).then_some(value)
    }

    /// The text field that edits `draft`; it is invalid while the draft is not a number in the
    /// range.
    fn field(&self, draft: String, separator: char) -> TextInput<Msg> {
        let negative = self.steps.min < 0.0;
        let decimal = self.steps.decimals() > 0;
        let broken = !draft.is_empty() && self.parse(&draft, separator).is_none();
        TextInput::new(draft)
            .placeholder(self.placeholder.clone())
            .invalid(self.invalid || broken)
            .disabled(self.disabled)
            .accept(move |c| c.is_ascii_digit() || (negative && c == '-') || (decimal && (c == '.' || c == separator)))
    }

    /// The two stepper segments inside `area`: down, then up.
    fn segments(&self, area: Rect) -> Option<[Rect; 2]> {
        if !self.steppers || area.width < SEGMENT * 2 {
            return None;
        }
        let up = Rect::new(area.right() - i32::from(SEGMENT), area.y, SEGMENT, area.height);
        let down = Rect::new(up.x - i32::from(SEGMENT), area.y, SEGMENT, area.height);
        Some([down, up])
    }

    /// Moves the value by `count` steps from the draft (or the value when the draft is not a
    /// number) and sends it.
    fn nudge(&self, cx: &mut EventCx<'_, Msg>, count: f64) {
        let separator = cx.env().i18n().decimal_separator();
        let memory = cx.memory::<NumberMemory>();
        let draft = self.draft(memory, separator);
        let base = read(&draft, separator).unwrap_or(self.value);
        let value = self.steps.clamp(base + count * self.steps.step);
        memory.draft = self.steps.write_with(value, separator);
        memory.seen = Some(value);
        memory.separator = Some(separator);
        if value != self.value
            && let Some(message) = &self.on_change
        {
            cx.emit(message(value));
        }
    }
}

impl<Msg: 'static> Widget<Msg> for NumberInput<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let style = cx.env().theme().style("text-input", None, &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((0, 1));
        let prompt = text::width(&cx.env().icons().glyph("prompt")) + 1;
        let bounds = [self.steps.min, self.steps.max, self.value]
            .iter()
            .filter(|value| value.is_finite())
            .map(|value| text::width(&self.steps.write(*value)))
            .max()
            .unwrap_or(0);
        let digits = if self.steps.min.is_finite() && self.steps.max.is_finite() { bounds } else { DEFAULT_DIGITS };
        let content = digits.max(text::width(&self.placeholder)).max(bounds).saturating_add(1);
        let steppers = if self.steppers { SEGMENT * 2 } else { 0 };
        Size::new(
            cells::sum([prompt, horizontal.saturating_mul(2), content, steppers]),
            vertical.saturating_mul(2).saturating_add(1),
        )
        .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let separator = cx.env().i18n().decimal_separator();
        let draft = self.draft(cx.memory::<NumberMemory>(), separator);
        let segments = self.segments(area);
        let field_width = area.width.saturating_sub(if segments.is_some() { SEGMENT * 2 } else { 0 });
        self.field(draft, separator).paint(cx, Rect::new(area.x, area.y, field_width, area.height));
        let Some(segments) = segments else {
            return;
        };
        if !self.disabled {
            cx.register_hit(area);
        }
        let focused = cx.is_focused();
        let pressed = if cx.is_pressed() { cx.memory::<NumberMemory>().pressed } else { None };
        let pointer = if self.disabled { None } else { cx.pointer() };
        let limits = [self.value <= self.steps.min, self.value >= self.steps.max];
        for (index, (rect, glyph_key)) in segments.into_iter().zip(["stepper-minus", "stepper-plus"]).enumerate() {
            let mut states = Vec::new();
            if pointer.is_some_and(|(x, y)| rect.contains(x, y)) {
                states.push(State::Hover);
            }
            if focused {
                states.push(State::Focus);
            }
            if pressed == Some(index) {
                states.push(State::Pressed);
            }
            if self.disabled || limits[index] {
                states.push(State::Disabled);
            }
            let style = cx.style("number-input-stepper", None, &states).text();
            let background = style.bg.unwrap_or_else(|| cx.color("active"));
            cx.clear(rect, background);
            let glyph = cx.env().icons().glyph(glyph_key).into_owned();
            let x = rect.x + i32::from(SEGMENT.saturating_sub(text::width(&glyph)) / 2);
            let y = rect.y + i32::from(rect.height / 2);
            cx.text(x, y, &glyph, CellStyle { bg: None, ..style }, SEGMENT);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let separator = cx.env().i18n().decimal_separator();
        let draft = self.draft(cx.memory::<NumberMemory>(), separator);
        self.field(draft, separator).paint_overlay(cx, anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.disabled {
            return false;
        }
        // The edit menu, open or asked for, takes the event before the steps do.
        let menu = edit_menu::is_open(cx) || edit_menu::asks(event);
        match event {
            _ if menu => {}
            Event::Key(key) => {
                let count = if key.is_plain(Key::Up) {
                    1.0
                } else if key.is_plain(Key::Down) {
                    -1.0
                } else if key.is_plain(Key::PageUp) {
                    LARGE_STEP
                } else if key.is_plain(Key::PageDown) {
                    -LARGE_STEP
                } else {
                    0.0
                };
                if count != 0.0 {
                    self.nudge(cx, count);
                    return true;
                }
            }
            Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) => {
                self.nudge(cx, if mouse.kind == MouseKind::ScrollUp { 1.0 } else { -1.0 });
                return true;
            }
            Event::Mouse(mouse) => {
                let hit = self
                    .segments(cx.area())
                    .and_then(|segments| segments.iter().position(|rect| rect.contains(mouse.x, mouse.y)));
                if let Some(index) = hit {
                    if mouse.kind == MouseKind::Down(MouseButton::Left) {
                        cx.memory::<NumberMemory>().pressed = Some(index);
                        cx.flash();
                        self.nudge(cx, if index == 0 { -1.0 } else { 1.0 });
                    }
                    return true;
                }
            }
            Event::Paste(_) | Event::PointerOutside => {}
        }
        let separator = cx.env().i18n().decimal_separator();
        let draft = self.draft(cx.memory::<NumberMemory>(), separator);
        let edit = self.field(draft, separator).edit(cx, event);
        if let Some(text) = edit.changed {
            let value = self.parse(&text, separator);
            let memory = cx.memory::<NumberMemory>();
            memory.draft = text;
            if let Some(value) = value
                && value != self.value
            {
                memory.seen = Some(value);
                if let Some(message) = &self.on_change {
                    cx.emit(message(value));
                }
            }
        }
        edit.handled
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }
}

/// The number `text` holds, written either with `separator`, the language's own, or with a point,
/// which a numeric keypad gives whatever the language is.
fn read(text: &str, separator: char) -> Option<f64> {
    let text = if separator == '.' { text.to_owned() } else { text.replace(separator, ".") };
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    struct Demo {
        value: f64,
        steppers: bool,
        step: f64,
    }

    impl App for Demo {
        type Msg = f64;
        fn update(&mut self, value: f64) -> Command<f64> {
            self.value = value;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, f64>) {
            ui.add(
                NumberInput::new(self.value)
                    .range(-5.0, 20.0)
                    .step(self.step)
                    .steppers(self.steppers)
                    .on_change(|value| value),
            )
            .width(Length::Cells(16))
            .id("replicas");
        }
    }

    fn harness(value: f64, steppers: bool, step: f64) -> Harness<Demo> {
        Harness::new(Demo { value, steppers, step }, 20, 1)
    }

    #[test]
    fn the_field_writes_and_reads_the_language_s_own_decimal_separator() {
        let mut h = harness(3.0, false, 0.5);
        assert_eq!(h.screen(), "  ❯ 3.0\n", "English writes a point");
        h.set_locale("fr");
        assert_eq!(h.screen(), "  ❯ 3,0\n", "a language changed while the field stands there rewrites it");
        // The comma the field shows is the one that can be typed into it.
        h.press("tab").press("backspace").type_text("5");
        assert_eq!(h.screen(), "▌ ❯ 3,5\n");
        assert_eq!(h.app().value, 3.5, "the comma reached the value");
        // A numeric keypad gives a point whatever the language is, and it reads the same.
        h.press("backspace").type_text("5");
        assert_eq!(h.app().value, 3.5);
        h.set_locale("en");
        assert_eq!(h.screen(), "▌ ❯ 3.5\n", "and back the other way");
    }

    #[test]
    fn arrows_step_and_clamp_to_the_range() {
        let mut h = harness(3.0, false, 1.0);
        assert_eq!(h.screen(), "  ❯ 3\n");
        h.press("tab").press("up").press("up");
        assert_eq!(h.app().value, 5.0);
        h.press("pgup").press("pgup");
        assert_eq!(h.app().value, 20.0);
        assert_eq!(h.screen(), "▌ ❯ 20\n");
        h.press("pgdn").press("pgdn").press("pgdn");
        assert_eq!(h.app().value, -5.0);
    }

    #[test]
    fn typing_accepts_numbers_only_and_marks_unfinished_or_out_of_range_text() {
        let mut h = harness(3.0, false, 1.0);
        let focused = h.press("tab").bg(0, 0);
        h.press("backspace").type_text("1x2.");
        assert_eq!(h.screen(), "▌ ❯ 12\n", "letters and the point of a whole-number field are ignored");
        assert_eq!(h.app().value, 12.0);
        h.type_text("5");
        assert_eq!(h.app().value, 12.0, "125 is out of range and not sent");
        assert_ne!(h.bg(0, 0), focused, "the invalid tint replaces the focus surface");
        h.press("ctrl+a").type_text("-");
        assert_eq!(h.screen(), "▌ ❯ -\n");
        assert_eq!(h.app().value, 12.0);
        h.type_text("4");
        assert_eq!(h.app().value, -4.0);
        assert_eq!(h.bg(0, 0), focused);
    }

    #[test]
    fn decimal_steps_allow_a_point_and_write_their_decimals() {
        let mut h = harness(1.0, false, 0.25);
        assert_eq!(h.screen(), "  ❯ 1.00\n");
        h.press("tab").press("up");
        assert_eq!(h.app().value, 1.25);
        h.press("ctrl+a").type_text("2.5");
        assert_eq!(h.app().value, 2.5);
        assert_eq!(h.screen(), "▌ ❯ 2.5\n", "a typed value keeps its own writing");
    }

    #[test]
    fn stepper_segments_click_and_grey_out_at_the_limits() {
        let mut h = harness(19.0, true, 1.0);
        assert_eq!(h.screen(), "  ❯ 19     −  +\n");
        h.click(14, 0);
        assert_eq!(h.app().value, 20.0);
        assert!(h.is_focused("replicas"));
        let theme = h.env().theme();
        assert_eq!(h.fg(14, 0), theme.color("muted"), "plus is spent at the maximum");
        h.click(11, 0);
        assert_eq!(h.app().value, 19.0);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert_eq!(h.screen(), "  > 19     -  +\n");
    }

    #[test]
    fn disabled_ignores_keys_and_clicks() {
        struct Disabled(f64);
        impl App for Disabled {
            type Msg = f64;
            fn update(&mut self, value: f64) -> Command<f64> {
                self.0 = value;
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, f64>) {
                ui.add(NumberInput::new(self.0).steppers(true).disabled(true).on_change(|v| v))
                    .width(Length::Cells(16));
            }
        }
        let mut h = Harness::new(Disabled(4.0), 20, 1);
        h.press("tab").press("up").click(14, 0).type_text("9");
        assert_eq!(h.app().0, 4.0);
        assert_eq!(h.fg(4, 0), h.env().theme().color("muted"));
    }

    #[test]
    fn the_wheel_steps_and_a_right_click_opens_the_edit_menu() {
        let mut h = Harness::new(Demo { value: 3.0, steppers: false, step: 1.0 }, 30, 6);
        h.set_reduced_motion(true);
        h.mouse(MouseKind::ScrollUp, 4, 0).mouse(MouseKind::ScrollUp, 4, 0);
        assert_eq!(h.app().value, 5.0, "one step per notch");
        h.mouse(MouseKind::ScrollDown, 4, 0);
        assert_eq!(h.app().value, 4.0);
        h.mouse(MouseKind::Down(MouseButton::Right), 4, 0).mouse(MouseKind::Up(MouseButton::Right), 4, 0);
        assert!(h.screen().contains("Select all"), "{}", h.screen());
        h.press("up").press("enter");
        assert!(!h.screen().contains("Select all"), "the menu took ↑ and Enter");
        assert_eq!(h.app().value, 4.0, "↑ moved the highlight, not the value");
        h.set_system_clipboard(Some("12 replicas")).mouse(MouseKind::Down(MouseButton::Right), 4, 0);
        h.click_text("Paste");
        assert_eq!(h.app().value, 12.0, "Select all, then Paste kept the digits");
    }

    #[test]
    fn external_changes_replace_the_draft() {
        let mut h = harness(3.0, false, 1.0);
        h.press("tab").press("backspace");
        assert_eq!(h.screen(), "▌ ❯\n");
        h.send(7.0);
        assert_eq!(h.screen(), "▌ ❯ 7\n");
    }
}
