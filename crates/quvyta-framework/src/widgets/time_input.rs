//! Time of day entry.

use super::edit_menu::{self, EditAction, TextMenu};
use crate::event::{Event, KeyEvent, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::{Key, Modifiers, Scope};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Cells of one segment: two digits with a cell of room on each side.
const SEGMENT: u16 = 4;

/// Largest value of the hour, minute and second segments.
const MAX: [u8; 3] = [23, 59, 59];

/// A time of day on a 24-hour clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimeOfDay {
    /// Hour, 0 to 23.
    pub hour: u8,
    /// Minute, 0 to 59.
    pub minute: u8,
    /// Second, 0 to 59.
    pub second: u8,
}

impl TimeOfDay {
    /// The time `hour:minute:second`; each part is capped at its largest value.
    #[must_use]
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self { hour: hour.min(MAX[0]), minute: minute.min(MAX[1]), second: second.min(MAX[2]) }
    }

    fn part(self, index: usize) -> u8 {
        [self.hour, self.minute, self.second][index]
    }

    fn with_part(self, index: usize, value: u8) -> Self {
        let mut parts = [self.hour, self.minute, self.second];
        parts[index] = value.min(MAX[index]);
        Self { hour: parts[0], minute: parts[1], second: parts[2] }
    }

    /// `hh:mm`, or `hh:mm:ss` with `seconds`.
    fn write(self, seconds: bool) -> String {
        if seconds {
            format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
        } else {
            format!("{:02}:{:02}", self.hour, self.minute)
        }
    }

    /// Reads `h:mm` or `h:mm:ss` with every part in range; surrounding spaces are ignored.
    fn parse(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.trim().split(':').collect();
        if !(2..=3).contains(&parts.len()) {
            return None;
        }
        let mut values = [0u8; 3];
        for (index, part) in parts.iter().enumerate() {
            let valid = (1..=2).contains(&part.len()) && part.chars().all(|c| c.is_ascii_digit());
            values[index] = part.parse().ok().filter(|value| valid && *value <= MAX[index])?;
        }
        Some(Self { hour: values[0], minute: values[1], second: values[2] })
    }
}

/// Builds a message from a new time.
type TimeMessage<Msg> = Box<dyn Fn(TimeOfDay) -> Msg>;

/// A time of day edited segment by segment: hours and minutes, optionally seconds, on a 24-hour
/// clock written the same in every language.
///
/// The field takes focus as one control and one segment at a time is active: Left and Right
/// move between segments, Up and Down change the active segment and wrap around (23 → 00), and
/// typing digits fills it, moving on once the segment is complete. `:` moves on as well and
/// Backspace sets the segment to zero. A click activates the segment under the pointer. The wheel changes
/// the segment under the pointer, one step per notch, without moving focus or the active segment;
/// over a colon it changes the segment the pointer was last over inside the field, or the active
/// segment when there was none. The application owns the time.
///
/// Ctrl+A selects the whole time; then Ctrl+C copies it as `09:30` (`09:30:00` with seconds) and
/// Ctrl+X copies it and sets the time to zero. Pasting `9:30` or `09:30:15` sets the time;
/// other text is ignored. A right click (or Shift+F10 and the menu key) opens the edit menu of
/// [`TextInput`](super::TextInput) with the same actions; Cut and Copy need the whole time
/// selected.
///
/// Style keys: `time-input` (`bg`, `fg`) with states `hover`, `focus`, `invalid`, `disabled`;
/// `time-segment` (`bg`, `fg`, `bold`) with `selected` for the active segment and `active` while
/// a first digit waits for the second; `time-separator` (`fg`); `text-input-selection` (`bg`,
/// `fg`) for the whole time selected.
pub struct TimeInput<Msg> {
    value: TimeOfDay,
    seconds: bool,
    invalid: bool,
    disabled: bool,
    on_change: Option<TimeMessage<Msg>>,
}

#[derive(Debug, Default)]
struct TimeMemory {
    /// The active segment.
    segment: usize,
    /// A first digit typed into the active segment, waiting for the second.
    pending: Option<u8>,
    /// Whether the whole time is selected, for copying.
    all: bool,
    /// The segment the pointer was last over since it entered the field; the wheel over a colon
    /// changes it. Forgotten when the pointer leaves the field.
    hovered: Option<usize>,
}

impl<Msg> TimeInput<Msg> {
    /// A field showing `value` as hours and minutes.
    #[must_use]
    pub fn new(value: TimeOfDay) -> Self {
        Self { value, seconds: false, invalid: false, disabled: false, on_change: None }
    }

    /// Shows and edits seconds too.
    #[must_use]
    pub fn seconds(mut self, seconds: bool) -> Self {
        self.seconds = seconds;
        self
    }

    /// Marks the time as failing validation, such as an end before its start.
    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Greys the field out; it cannot be focused or changed.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message carrying the new time after every change.
    #[must_use]
    pub fn on_change(mut self, message: impl Fn(TimeOfDay) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(message));
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_change.is_some()
    }

    fn count(&self) -> usize {
        if self.seconds { 3 } else { 2 }
    }

    /// Left edge of segment `index`, relative to the field.
    fn offset(index: usize) -> u16 {
        // Segments are separated by one colon cell.
        u16::try_from(index).unwrap_or(0) * (SEGMENT + 1)
    }

    fn width(&self) -> u16 {
        Self::offset(self.count() - 1) + SEGMENT
    }

    fn change(&self, cx: &mut EventCx<'_, Msg>, value: TimeOfDay) {
        if value != self.value
            && let Some(message) = &self.on_change
        {
            cx.emit(message(value));
        }
    }

    /// Types `digit` into the active segment.
    fn type_digit(&self, cx: &mut EventCx<'_, Msg>, digit: u8) {
        let last = self.count() - 1;
        let memory = cx.memory::<TimeMemory>();
        let segment = memory.segment.min(last);
        let max = MAX[segment];
        let joined = memory.pending.map(|first| first * 10 + digit).filter(|joined| *joined <= max);
        let (value, complete) = match joined {
            Some(joined) => (joined, true),
            // A lone digit that cannot start a two-digit value in range is complete by itself.
            None => (digit, digit * 10 > max),
        };
        memory.pending = if complete { None } else { Some(digit) };
        if complete && segment < last {
            memory.segment = segment + 1;
        }
        self.change(cx, self.value.with_part(segment, value));
    }

    /// Moves segment `segment` one up or down, wrapping around (23 → 00).
    fn step(&self, cx: &mut EventCx<'_, Msg>, segment: usize, up: bool) {
        let last = self.count() - 1;
        let memory = cx.memory::<TimeMemory>();
        // A first digit waiting in the active segment stays when the wheel changes another one.
        if segment == memory.segment.min(last) {
            memory.pending = None;
        }
        let span = u16::from(MAX[segment]) + 1;
        let delta = if up { 1 } else { span - 1 };
        let value = (u16::from(self.value.part(segment)) + delta) % span;
        self.change(cx, self.value.with_part(segment, u8::try_from(value).unwrap_or(0)));
    }

    /// The segment whose cells include column `offset` of the field; `None` over a colon or past
    /// the end.
    fn segment_at(&self, offset: i32) -> Option<usize> {
        (0..self.count()).find(|index| {
            let start = i32::from(Self::offset(*index));
            (start..start + i32::from(SEGMENT)).contains(&offset)
        })
    }

    /// The segment the wheel changes at column `x`: the one under the pointer, else the one it
    /// was last over inside the field, else the active one.
    fn wheel_target(&self, cx: &mut EventCx<'_, Msg>, x: i32) -> usize {
        let last = self.count() - 1;
        let under = self.segment_at(x - cx.area().x);
        let memory = cx.memory::<TimeMemory>();
        if under.is_some() {
            memory.hovered = under;
        }
        under.or(memory.hovered).unwrap_or(memory.segment).min(last)
    }

    /// Activates the segment under column `x`.
    fn activate_at(&self, cx: &mut EventCx<'_, Msg>, x: i32) {
        let offset = x - cx.area().x;
        let index = (0..self.count()).rev().find(|index| offset >= i32::from(Self::offset(*index))).unwrap_or(0);
        let memory = cx.memory::<TimeMemory>();
        memory.segment = index;
        memory.pending = None;
    }

    /// Copies, cuts, pastes or selects the whole time.
    fn apply(&self, cx: &mut EventCx<'_, Msg>, action: EditAction) {
        let all = std::mem::take(&mut cx.memory::<TimeMemory>().all);
        match action {
            EditAction::Cut | EditAction::Copy if all => {
                cx.copy(self.value.write(self.seconds));
                if action == EditAction::Cut {
                    self.change(cx, TimeOfDay::default());
                }
            }
            EditAction::Cut | EditAction::Copy => {}
            EditAction::Paste => cx.run_action(Scope::Global, "paste"),
            EditAction::SelectAll => cx.memory::<TimeMemory>().all = true,
        }
    }

    /// The edit action of a Ctrl chord: A, C and X.
    fn chord_action(key: &KeyEvent) -> Option<EditAction> {
        let ctrl = Modifiers { ctrl: true, ..Modifiers::default() };
        match key.chord.key {
            Key::Char('a') if key.chord.mods == ctrl => Some(EditAction::SelectAll),
            Key::Char('c') if key.chord.mods == ctrl => Some(EditAction::Copy),
            Key::Char('x') if key.chord.mods == ctrl => Some(EditAction::Cut),
            _ => None,
        }
    }

    /// Offers `event` to the edit menu, which opens on a right press or its keys and takes every
    /// event while open. Returns whether the menu used the event.
    fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let open = edit_menu::is_open(cx);
        if !open && !edit_menu::asks(event) {
            return false;
        }
        if let Event::Mouse(mouse) = event
            && !open
            && !cx.memory::<TimeMemory>().all
        {
            self.activate_at(cx, mouse.x);
        }
        let all = cx.memory::<TimeMemory>().all;
        let (used, chosen) = TextMenu::edit(cx.env(), all, cx.can_paste()).event(cx, event);
        if used && !open {
            cx.probe_clipboard();
        }
        if let Some(action) = chosen {
            self.apply(cx, action);
        }
        used
    }
}

impl<Msg: 'static> Widget<Msg> for TimeInput<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        Size::new(self.width(), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let mut states = if self.active() { cx.states() } else { Vec::new() };
        if self.invalid {
            states.push(State::Invalid);
        }
        if self.disabled {
            states.push(State::Disabled);
        }
        let focused = states.contains(&State::Focus);
        // Painting follows every pointer move, so it keeps the last hovered segment: a move onto
        // a segment remembers it, a move off the field forgets it.
        let pointer = cx.pointer().map(|(x, _)| self.segment_at(x - area.x));
        let (segment, pending, all) = {
            let memory = cx.memory::<TimeMemory>();
            if !focused {
                *memory = TimeMemory { hovered: memory.hovered, ..TimeMemory::default() };
            }
            match pointer {
                None => memory.hovered = None,
                Some(Some(index)) => memory.hovered = Some(index),
                Some(None) => {}
            }
            (memory.segment, memory.pending, memory.all)
        };
        let selection = cx.style("text-input-selection", None, &states).text();
        let field_style = cx.style("time-input", None, &states);
        let surface = field_style.text();
        let field = Rect::new(area.x, area.y, self.width().min(area.width), 1);
        cx.clear(field, surface.bg.unwrap_or_else(|| cx.color("raised")));
        let pillar = field_style.color("pillar");
        if self.active() {
            cx.register_hit(field);
            edit_menu::request_overlay(cx, field);
        }
        let separator = cx.style("time-separator", None, &states).text();
        // The whole time selected reads as one run of selection, colons included, without the
        // active segment standing out.
        if let Some(bg) = selection.bg.filter(|_| all) {
            cx.fill(field, bg);
        }
        for index in 0..self.count() {
            let x = field.x + i32::from(Self::offset(index));
            if index > 0 {
                cx.text(x - 1, field.y, ":", CellStyle { bg: None, ..separator }, 1);
            }
            let mut segment_states = states.clone();
            if focused && index == segment && !all {
                segment_states.push(State::Selected);
                if pending.is_some() {
                    segment_states.push(State::Active);
                }
            }
            let mut style = cx.style("time-segment", None, &segment_states).text();
            if all {
                style.bg = None;
                style.fg = selection.fg.or(style.fg);
            }
            let rect = Rect::new(x, field.y, SEGMENT, 1).intersect(field);
            if let Some(bg) = style.bg {
                cx.clear(rect, bg);
            }
            let digits = format!("{:02}", self.value.part(index));
            let text_style = CellStyle { bg: None, fg: style.fg.or(surface.fg), ..style };
            cx.text(x + 1, field.y, &digits, text_style, text::width(&digits).min(rect.width.saturating_sub(1)));
        }
        // Drawn last so a selected first segment keeps it; the digits never slide.
        if let Some(color) = pillar {
            cx.pillar(field.x, field.y, color);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let all = cx.memory::<TimeMemory>().all;
        TextMenu::edit(cx.env(), all, cx.can_paste()).paint_overlay(cx, anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        if self.menu_event(cx, event) {
            return true;
        }
        let last = self.count() - 1;
        match event {
            Event::Key(key) if let Some(action) = Self::chord_action(key) => {
                self.apply(cx, action);
                true
            }
            Event::Paste(text) => {
                cx.memory::<TimeMemory>().all = false;
                let Some(time) = TimeOfDay::parse(text) else {
                    return false;
                };
                let time = if self.seconds { time } else { TimeOfDay { second: self.value.second, ..time } };
                self.change(cx, time);
                true
            }
            Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) => {
                let up = mouse.kind == MouseKind::ScrollUp;
                let segment = self.wheel_target(cx, mouse.x);
                self.step(cx, segment, up);
                true
            }
            Event::Key(key) => {
                cx.memory::<TimeMemory>().all = false;
                let segment = cx.memory::<TimeMemory>().segment.min(last);
                if key.is_plain(Key::Left) || key.is_plain(Key::Right) {
                    let memory = cx.memory::<TimeMemory>();
                    memory.segment =
                        if key.is_plain(Key::Left) { segment.saturating_sub(1) } else { (segment + 1).min(last) };
                    memory.pending = None;
                    return true;
                }
                if key.is_plain(Key::Up) || key.is_plain(Key::Down) {
                    self.step(cx, segment, key.is_plain(Key::Up));
                    return true;
                }
                if key.is_plain(Key::Backspace) {
                    cx.memory::<TimeMemory>().pending = None;
                    self.change(cx, self.value.with_part(segment, 0));
                    return true;
                }
                match key.text {
                    Some(':') => {
                        let memory = cx.memory::<TimeMemory>();
                        memory.segment = (segment + 1).min(last);
                        memory.pending = None;
                        true
                    }
                    Some(c) if c.is_ascii_digit() => {
                        self.type_digit(cx, u8::try_from(c.to_digit(10).unwrap_or(0)).unwrap_or(0));
                        true
                    }
                    _ => false,
                }
            }
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                cx.memory::<TimeMemory>().all = false;
                self.activate_at(cx, mouse.x);
                true
            }
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
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    struct Demo {
        time: TimeOfDay,
        seconds: bool,
        disabled: bool,
    }

    impl App for Demo {
        type Msg = TimeOfDay;
        fn update(&mut self, time: TimeOfDay) -> Command<TimeOfDay> {
            self.time = time;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, TimeOfDay>) {
            ui.add(TimeInput::new(self.time).seconds(self.seconds).disabled(self.disabled).on_change(|t| t))
                .id("starts");
        }
    }

    fn harness(hour: u8, minute: u8, seconds: bool) -> Harness<Demo> {
        Harness::new(Demo { time: TimeOfDay::new(hour, minute, 0), seconds, disabled: false }, 20, 1)
    }

    #[test]
    fn draws_segments_with_a_faint_colon_and_raises_the_active_one() {
        let mut h = harness(9, 5, false);
        assert_eq!(h.screen(), " 09 : 05\n");
        let theme = h.env().theme();
        assert_eq!(h.fg(4, 0), theme.color("muted"));
        let idle = h.bg(1, 0);
        h.press("tab");
        assert_ne!(h.bg(1, 0), h.bg(6, 0), "the active segment stands out");
        assert_ne!(h.bg(1, 0), idle);
        h.press("right");
        assert_eq!(h.bg(1, 0), h.bg(4, 0), "the first segment is back on the field surface");
        let mut h = harness(23, 59, true);
        assert_eq!(h.screen(), " 23 : 59 : 00\n");
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert!(h.screen().is_ascii());
    }

    #[test]
    fn arrows_move_between_segments_and_wrap_values() {
        let mut h = harness(23, 0, false);
        h.press("tab").press("up");
        assert_eq!(h.app().time, TimeOfDay::new(0, 0, 0));
        h.press("down");
        assert_eq!(h.app().time, TimeOfDay::new(23, 0, 0));
        h.press("right").press("down");
        assert_eq!(h.app().time, TimeOfDay::new(23, 59, 0));
        h.press("right").press("up");
        assert_eq!(h.app().time, TimeOfDay::new(23, 0, 0), "right stops at the last segment");
    }

    #[test]
    fn typing_fills_segments_and_moves_on() {
        let mut h = harness(0, 0, true);
        h.press("tab").type_text("0930");
        assert_eq!(h.app().time, TimeOfDay::new(9, 30, 0));
        h.press("left").press("left").type_text("7");
        assert_eq!(h.app().time, TimeOfDay::new(7, 30, 0), "7 cannot start an hour, so it completes it");
        h.type_text("45").type_text("2");
        assert_eq!(h.app().time, TimeOfDay::new(7, 45, 2));
        h.type_text("9");
        assert_eq!(h.app().time, TimeOfDay::new(7, 45, 29));
        h.press("left").press("left").type_text("24");
        assert_eq!(h.app().time, TimeOfDay::new(4, 45, 29), "24 is no hour; the 4 stands alone");
        h.press("backspace");
        assert_eq!(h.app().time, TimeOfDay::new(4, 0, 29), "the minute became active after the 4");
    }

    #[test]
    fn the_wheel_changes_the_segment_under_the_pointer() {
        let mut h = harness(8, 15, false);
        h.click(6, 0);
        h.mouse(MouseKind::ScrollUp, 6, 0).mouse(MouseKind::ScrollUp, 6, 0);
        assert_eq!(h.app().time, TimeOfDay::new(8, 17, 0), "one minute per notch");
        h.mouse(MouseKind::ScrollDown, 6, 0);
        assert_eq!(h.app().time, TimeOfDay::new(8, 16, 0));
    }

    /// The active segment's background, to show the wheel leaves it where it was.
    fn active_marks(h: &Harness<Demo>) -> (Option<crate::color::Rgb>, Option<crate::color::Rgb>) {
        (h.bg(1, 0), h.bg(6, 0))
    }

    #[test]
    fn the_wheel_over_a_segment_changes_it_whatever_is_active() {
        let mut h = Harness::new(Demo { time: TimeOfDay::new(8, 15, 0), seconds: false, disabled: false }, 20, 3);
        h.click(6, 0);
        let before = active_marks(&h);
        h.hover(1, 0).mouse(MouseKind::ScrollUp, 1, 0);
        assert_eq!(h.app().time, TimeOfDay::new(9, 15, 0), "hours change while the minutes are active");
        h.mouse(MouseKind::ScrollDown, 2, 0).mouse(MouseKind::ScrollDown, 2, 0);
        assert_eq!(h.app().time, TimeOfDay::new(7, 15, 0), "one hour per notch");
        h.press("up");
        assert_eq!(h.app().time, TimeOfDay::new(7, 16, 0), "the keys still change the active minutes");
        assert_eq!(active_marks(&h), before, "the active segment and focus stay");
        // Unfocused, the wheel over the minutes changes them without taking focus.
        h.click(1, 2);
        let idle = active_marks(&h);
        h.hover(7, 0).mouse(MouseKind::ScrollUp, 7, 0);
        assert_eq!(h.app().time, TimeOfDay::new(7, 17, 0));
        h.hover(0, 0).mouse(MouseKind::ScrollDown, 0, 0);
        assert_eq!(h.app().time, TimeOfDay::new(6, 17, 0), "the pillar cell belongs to the hours");
        h.hover(1, 2);
        assert_eq!(active_marks(&h), idle, "the field did not take focus");
    }

    #[test]
    fn the_wheel_over_a_colon_changes_the_segment_last_hovered() {
        let mut h = Harness::new(Demo { time: TimeOfDay::new(23, 59, 0), seconds: false, disabled: false }, 20, 3);
        h.click(1, 0);
        h.hover(6, 0).hover(4, 0).mouse(MouseKind::ScrollUp, 4, 0);
        assert_eq!(h.app().time, TimeOfDay::new(23, 0, 0), "the minutes wrap like the keys");
        h.hover(2, 0).hover(4, 0).mouse(MouseKind::ScrollUp, 4, 0);
        assert_eq!(h.app().time, TimeOfDay::new(0, 0, 0), "the hours wrap like the keys");
        // Leaving the field forgets the last hover: the colon then changes the active hours.
        h.hover(6, 0).hover(6, 2).hover(4, 0).mouse(MouseKind::ScrollDown, 4, 0);
        assert_eq!(h.app().time, TimeOfDay::new(23, 0, 0));
    }

    #[test]
    fn the_wheel_over_a_colon_without_a_hover_changes_the_active_segment() {
        let mut h = Harness::new(Demo { time: TimeOfDay::new(8, 15, 30), seconds: true, disabled: false }, 20, 3);
        h.press("tab").press("right").press("right");
        h.mouse(MouseKind::ScrollUp, 4, 0);
        assert_eq!(h.app().time, TimeOfDay::new(8, 15, 31), "straight onto the colon: the active seconds");
        h.hover(11, 0).hover(9, 0).mouse(MouseKind::ScrollUp, 9, 0);
        assert_eq!(h.app().time, TimeOfDay::new(8, 15, 32), "last over the seconds");
    }

    #[test]
    fn the_wheel_does_nothing_on_a_disabled_field() {
        let mut h = Harness::new(Demo { time: TimeOfDay::new(8, 15, 0), seconds: false, disabled: true }, 20, 1);
        h.hover(6, 0).mouse(MouseKind::ScrollUp, 6, 0).mouse(MouseKind::ScrollUp, 1, 0);
        assert_eq!(h.app().time, TimeOfDay::new(8, 15, 0));
    }

    #[test]
    fn select_all_copies_cuts_and_pastes_the_whole_time() {
        let mut h = Harness::new(Demo { time: TimeOfDay::new(9, 30, 0), seconds: false, disabled: false }, 30, 6);
        h.set_reduced_motion(true);
        h.press("tab").press("ctrl+c");
        assert!(h.copied().is_empty(), "nothing is selected yet");
        let idle = h.bg(6, 0);
        h.press("ctrl+a");
        let selection = h.env().theme().style("text-input-selection", None, &[]).paint("bg").map(|paint| paint.at(0.0));
        assert_ne!(h.bg(6, 0), idle, "every segment shows the selection");
        assert_eq!((h.bg(1, 0), h.bg(4, 0), h.bg(6, 0)), (selection, selection, selection), "colon included");
        h.press("ctrl+c");
        assert_eq!(h.clipboard(), Some("09:30"));
        h.press("ctrl+a").press("ctrl+x");
        assert_eq!(h.app().time, TimeOfDay::new(0, 0, 0));
        h.paste("7:45");
        assert_eq!(h.app().time, TimeOfDay::new(7, 45, 0));
        h.paste("25:00").paste("noon");
        assert_eq!(h.app().time, TimeOfDay::new(7, 45, 0), "text that is no time is ignored");
    }

    #[test]
    fn right_click_opens_the_edit_menu() {
        let mut h = Harness::new(Demo { time: TimeOfDay::new(9, 30, 0), seconds: true, disabled: false }, 30, 6);
        h.set_reduced_motion(true);
        h.mouse(MouseKind::Down(MouseButton::Right), 6, 0).mouse(MouseKind::Up(MouseButton::Right), 6, 0);
        assert!(h.screen().contains("Select all"), "{}", h.screen());
        let muted = h.env().theme().color("muted");
        assert_eq!(h.fg(8, 2), muted, "Copy needs the whole time selected: {}", h.screen());
        h.click_text("Select all");
        h.mouse(MouseKind::Down(MouseButton::Right), 6, 0).mouse(MouseKind::Up(MouseButton::Right), 6, 0);
        h.click_text("Copy");
        assert_eq!(h.clipboard(), Some("09:30:00"));
        h.set_system_clipboard(Some("18:05:30")).mouse(MouseKind::Down(MouseButton::Right), 1, 0);
        h.click_text("Paste");
        assert_eq!(h.app().time, TimeOfDay::new(18, 5, 30));
    }

    #[test]
    fn click_activates_a_segment_and_disabled_ignores_input() {
        let mut h = harness(8, 15, false);
        h.click(6, 0).press("up");
        assert_eq!(h.app().time, TimeOfDay::new(8, 16, 0));
        let mut h = Harness::new(Demo { time: TimeOfDay::new(8, 15, 0), seconds: false, disabled: true }, 20, 1);
        h.press("tab").press("up").click(1, 0).type_text("1");
        assert_eq!(h.app().time, TimeOfDay::new(8, 15, 0));
        assert_eq!(h.fg(1, 0), h.env().theme().color("muted"));
    }
}
