//! Length of time entry: hours and minutes, optionally seconds.

mod parse;
#[cfg(test)]
mod tests;

use std::time::Duration;

pub use parse::{DurationError, DurationUnit, parse_duration};

use super::edit_menu::{self, EditAction, TextMenu};
use crate::event::{Event, KeyEvent, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::i18n::I18n;
use crate::keymap::{Key, Modifiers, Scope};
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};
use parse::LONGEST;

/// Builds a message from a new length of time.
type DurationMessage<Msg> = Box<dyn Fn(Duration) -> Msg>;

/// Builds a message from a paste that could not be read.
type RejectMessage<Msg> = Box<dyn Fn(DurationError) -> Msg>;

/// Hours, minutes and seconds of `total` seconds; hours are not capped.
fn parts(total: u64) -> [u64; 3] {
    [total / 3600, total / 60 % 60, total % 60]
}

/// `total` with the part at `index` set to `value`. Minutes and seconds past 59 carry into the
/// larger units, so typing 90 into the minutes gives an hour and a half; the result stops at the
/// longest length.
fn with_part(total: u64, index: usize, value: u64) -> u64 {
    let mut parts = parts(total);
    parts[index] = value;
    (parts[0] * 3600 + parts[1] * 60 + parts[2]).min(LONGEST)
}

/// Digits in `value`, at least one.
fn digits(value: u64) -> u16 {
    u16::try_from(value.checked_ilog10().unwrap_or(0) + 1).unwrap_or(u16::MAX)
}

/// Where the segments and the marks between them sit in a field, relative to its left edge.
#[derive(Debug, PartialEq, Eq)]
struct Layout {
    /// Left edge and width of every segment: a cell of room, the digits, a cell of room.
    segments: Vec<(u16, u16)>,
    /// The unit word after every segment, or a colon between segments in the compact form.
    marks: Vec<(u16, String)>,
    width: u16,
}

/// A length of time edited segment by segment: hours and minutes, optionally seconds, each
/// followed by its unit in the active language: `1 h 30 min`, `1 sa 30 dk`.
///
/// It is built like [`TimeInput`](super::TimeInput) and handled the same way. The field takes
/// focus as one control and one segment at a time is active: Left and Right move between
/// segments, Up and Down change the active segment by one of its unit, and typing digits fills
/// it, two digits a segment, moving on once the segment is complete. `:` and Space move on as
/// well and Backspace sets the segment to zero. A click activates the segment under the pointer.
/// The wheel changes the segment under the pointer, one unit per notch, without moving focus or
/// the active segment; over a unit word it changes the segment the pointer was last over inside
/// the field, or the active segment when there was none. The application owns the length.
///
/// Unlike a time of day a length does not wrap: stepping carries between the units (0 h 59 min
/// up is 1 h 00 min), stops at zero going down and at 99 h 59 min 59 s going up. Minutes and
/// seconds typed past 59 carry too, so `90` typed into the minutes reads `1 h 30 min`.
///
/// Ctrl+A selects the whole length; then Ctrl+C copies it as written in the active language
/// (`1 h 30 min`) and Ctrl+X copies it and sets the length to zero. Pasting sets the length from
/// anything [`parse_duration`] reads (`90 dk`, `1 h 30 min`, `2:15`); other text is ignored, or
/// sent to [`on_reject`](Self::on_reject) with the reason. A right click (or Shift+F10 and the
/// menu key) opens the edit menu of [`TextInput`](super::TextInput) with the same actions; Cut
/// and Copy need the whole length selected.
///
/// A zero length is the field's empty state: its digits are drawn as faint as the unit words
/// until the pointer or focus is on the field. Where the words do not fit, the field shows the
/// compact clock form, `1 : 30`, with the same segments and keys.
///
/// Style keys, shared with [`TimeInput`](super::TimeInput) so both fields look alike:
/// `time-input` (`bg`, `fg`) with states `hover`, `focus`, `invalid`, `disabled`; `time-segment`
/// (`bg`, `fg`, `bold`) with `selected` for the active segment and `active` while a first digit
/// waits for the second; `time-separator` (`fg`) for the unit words, the colons and the digits of
/// a zero length at rest; `text-input-selection` (`bg`, `fg`) for the whole length selected.
/// Language keys: `quvyta.duration.*` for the units and the reasons a paste was not read,
/// `quvyta.edit.*` for the menu.
pub struct DurationInput<Msg> {
    value: Duration,
    seconds: bool,
    invalid: bool,
    disabled: bool,
    on_change: Option<DurationMessage<Msg>>,
    on_reject: Option<RejectMessage<Msg>>,
}

#[derive(Debug, Default)]
struct DurationMemory {
    /// The active segment.
    segment: usize,
    /// A first digit typed into the active segment, waiting for the second.
    pending: Option<u8>,
    /// Whether the whole length is selected, for copying.
    all: bool,
    /// The segment the pointer was last over since it entered the field; the wheel over a unit
    /// word changes it. Forgotten when the pointer leaves the field.
    hovered: Option<usize>,
}

impl<Msg> DurationInput<Msg> {
    /// A field showing `value` as hours and minutes. Parts of a second are not shown and are
    /// dropped by the first change.
    #[must_use]
    pub fn new(value: Duration) -> Self {
        Self { value, seconds: false, invalid: false, disabled: false, on_change: None, on_reject: None }
    }

    /// Shows and edits seconds too.
    #[must_use]
    pub fn seconds(mut self, seconds: bool) -> Self {
        self.seconds = seconds;
        self
    }

    /// Marks the length as failing validation, such as a break longer than the session.
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

    /// Message carrying the new length after every change.
    #[must_use]
    pub fn on_change(mut self, message: impl Fn(Duration) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(message));
        self
    }

    /// Message carrying why a pasted text could not be read, so the application can say so
    /// next to the field with [`DurationError::message`]. Without it such a paste is ignored and
    /// passed on, as in [`TimeInput`](super::TimeInput).
    #[must_use]
    pub fn on_reject(mut self, message: impl Fn(DurationError) -> Msg + 'static) -> Self {
        self.on_reject = Some(Box::new(message));
        self
    }

    fn active(&self) -> bool {
        !self.disabled && self.on_change.is_some()
    }

    fn count(&self) -> usize {
        if self.seconds { 3 } else { 2 }
    }

    fn total(&self) -> u64 {
        self.value.as_secs()
    }

    /// The layout with unit words from `i18n`, or the compact clock form without.
    fn layout(&self, words: Option<&I18n>) -> Layout {
        let parts = parts(self.total());
        let mut segments = Vec::new();
        let mut marks = Vec::new();
        let mut x: u16 = 0;
        for (index, unit) in DurationUnit::ALL[..self.count()].iter().enumerate() {
            if words.is_none() && index > 0 {
                marks.push((x, ":".to_owned()));
                x = x.saturating_add(1);
            }
            // Hours take two digits, more only for a length handed in beyond the longest.
            let width = if index == 0 { digits(parts[0]).max(2) } else { 2 };
            segments.push((x, width + 2));
            x = x.saturating_add(width + 2);
            if let Some(i18n) = words {
                let word = unit.short(i18n);
                let width = text::width(&word);
                marks.push((x, word));
                x = x.saturating_add(width);
            }
        }
        Layout { segments, marks, width: x }
    }

    /// The layout the field has in `width` cells: with unit words when they fit.
    fn fitted(&self, i18n: &I18n, width: u16) -> Layout {
        let full = self.layout(Some(i18n));
        if full.width <= width { full } else { self.layout(None) }
    }

    fn change(&self, cx: &mut EventCx<'_, Msg>, total: u64) {
        if total != self.total()
            && let Some(message) = &self.on_change
        {
            cx.emit(message(Duration::from_secs(total)));
        }
    }

    /// Types `digit` into the active segment: two digits make a segment.
    fn type_digit(&self, cx: &mut EventCx<'_, Msg>, digit: u8) {
        let last = self.count() - 1;
        let memory = cx.memory::<DurationMemory>();
        let segment = memory.segment.min(last);
        let (value, complete) = match memory.pending {
            Some(first) => (first * 10 + digit, true),
            None => (digit, false),
        };
        memory.pending = if complete { None } else { Some(digit) };
        if complete && segment < last {
            memory.segment = segment + 1;
        }
        self.change(cx, with_part(self.total(), segment, u64::from(value)));
    }

    /// Moves the length one unit of segment `segment` up or down, carrying between units and
    /// stopping at zero and at the longest length.
    fn step(&self, cx: &mut EventCx<'_, Msg>, segment: usize, up: bool) {
        let last = self.count() - 1;
        let memory = cx.memory::<DurationMemory>();
        // A first digit waiting in the active segment stays when the wheel changes another one.
        if segment == memory.segment.min(last) {
            memory.pending = None;
        }
        let unit = DurationUnit::ALL[segment].seconds();
        let total = self.total();
        let next = if up { (total + unit).min(LONGEST.max(total)) } else { total.saturating_sub(unit) };
        self.change(cx, next);
    }

    /// The segment whose cells include column `offset` of the field; `None` over a unit word, a
    /// colon or past the end.
    fn segment_at(layout: &Layout, offset: i32) -> Option<usize> {
        layout.segments.iter().position(|(start, width)| {
            let start = i32::from(*start);
            (start..start + i32::from(*width)).contains(&offset)
        })
    }

    /// The layout of the field handling an event.
    fn event_layout(&self, cx: &EventCx<'_, Msg>) -> Layout {
        self.fitted(cx.env().i18n(), cx.area().width)
    }

    /// The segment the wheel changes at column `x`: the one under the pointer, else the one it
    /// was last over inside the field, else the active one.
    fn wheel_target(&self, cx: &mut EventCx<'_, Msg>, x: i32) -> usize {
        let last = self.count() - 1;
        let under = Self::segment_at(&self.event_layout(cx), x - cx.area().x);
        let memory = cx.memory::<DurationMemory>();
        if under.is_some() {
            memory.hovered = under;
        }
        under.or(memory.hovered).unwrap_or(memory.segment).min(last)
    }

    /// Activates the segment under column `x`; a unit word belongs to the segment before it.
    fn activate_at(&self, cx: &mut EventCx<'_, Msg>, x: i32) {
        let offset = x - cx.area().x;
        let layout = self.event_layout(cx);
        let index = layout.segments.iter().rposition(|(start, _)| offset >= i32::from(*start)).unwrap_or(0);
        let memory = cx.memory::<DurationMemory>();
        memory.segment = index;
        memory.pending = None;
    }

    /// Copies, cuts, pastes or selects the whole length.
    fn apply(&self, cx: &mut EventCx<'_, Msg>, action: EditAction) {
        let all = std::mem::take(&mut cx.memory::<DurationMemory>().all);
        match action {
            EditAction::Cut | EditAction::Copy if all => {
                cx.copy(parse::write(self.value, self.seconds, cx.env().i18n()));
                if action == EditAction::Cut {
                    self.change(cx, 0);
                }
            }
            EditAction::Cut | EditAction::Copy => {}
            EditAction::Paste => cx.run_action(Scope::Global, "paste"),
            EditAction::SelectAll => cx.memory::<DurationMemory>().all = true,
        }
    }

    /// Sets the length from pasted `text`. Without seconds on screen the pasted seconds are
    /// dropped and the length keeps its own, as the time field does.
    fn paste(&self, cx: &mut EventCx<'_, Msg>, text: &str) -> bool {
        cx.memory::<DurationMemory>().all = false;
        match parse_duration(text, cx.env().i18n()) {
            Ok(length) => {
                let pasted = length.as_secs();
                let total = if self.seconds { pasted } else { with_part(pasted, 2, parts(self.total())[2]) };
                self.change(cx, total);
                true
            }
            Err(error) => match &self.on_reject {
                Some(message) => {
                    cx.emit(message(error));
                    true
                }
                None => false,
            },
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
            && !cx.memory::<DurationMemory>().all
        {
            self.activate_at(cx, mouse.x);
        }
        let all = cx.memory::<DurationMemory>().all;
        let (used, chosen) = TextMenu::edit(cx.env(), all, cx.can_paste()).event(cx, event);
        if used && !open {
            cx.probe_clipboard();
        }
        if let Some(action) = chosen {
            self.apply(cx, action);
        }
        used
    }

    /// Handles a key while focused.
    fn key(&self, cx: &mut EventCx<'_, Msg>, key: &KeyEvent) -> bool {
        let last = self.count() - 1;
        cx.memory::<DurationMemory>().all = false;
        let segment = cx.memory::<DurationMemory>().segment.min(last);
        if key.is_plain(Key::Left) || key.is_plain(Key::Right) {
            let memory = cx.memory::<DurationMemory>();
            memory.segment = if key.is_plain(Key::Left) { segment.saturating_sub(1) } else { (segment + 1).min(last) };
            memory.pending = None;
            return true;
        }
        if key.is_plain(Key::Up) || key.is_plain(Key::Down) {
            self.step(cx, segment, key.is_plain(Key::Up));
            return true;
        }
        if key.is_plain(Key::Backspace) {
            cx.memory::<DurationMemory>().pending = None;
            self.change(cx, with_part(self.total(), segment, 0));
            return true;
        }
        if key.is_plain(Key::Space) || key.text == Some(':') {
            let memory = cx.memory::<DurationMemory>();
            memory.segment = (segment + 1).min(last);
            memory.pending = None;
            return true;
        }
        match key.text {
            Some(c) if c.is_ascii_digit() => {
                self.type_digit(cx, u8::try_from(c.to_digit(10).unwrap_or(0)).unwrap_or(0));
                true
            }
            _ => false,
        }
    }
}

impl<Msg: 'static> Widget<Msg> for DurationInput<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let full = self.layout(Some(cx.env().i18n()));
        Size::new(full.width, 1).min(available)
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
        let layout = self.fitted(cx.env().i18n(), area.width);
        // Painting follows every pointer move, so it keeps the last hovered segment: a move onto
        // a segment remembers it, a move off the field forgets it.
        let pointer = cx.pointer().map(|(x, _)| Self::segment_at(&layout, x - area.x));
        let (segment, pending, all) = {
            let memory = cx.memory::<DurationMemory>();
            if !focused {
                *memory = DurationMemory { hovered: memory.hovered, ..DurationMemory::default() };
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
        let field = Rect::new(area.x, area.y, layout.width.min(area.width), 1);
        cx.clear(field, surface.bg.unwrap_or_else(|| cx.color("raised")));
        let pillar = field_style.color("pillar");
        if self.active() {
            cx.register_hit(field);
            edit_menu::request_overlay(cx, field);
        }
        let separator = cx.style("time-separator", None, &states).text();
        // The whole length selected reads as one run of selection, unit words included, without
        // the active segment standing out.
        if let Some(bg) = selection.bg.filter(|_| all) {
            cx.fill(field, bg);
        }
        for (x, mark) in &layout.marks {
            let x = field.x + i32::from(*x);
            let room = u16::try_from(field.right() - x).unwrap_or(0);
            let fg = if all { selection.fg.or(separator.fg) } else { separator.fg };
            cx.text(x, field.y, mark, CellStyle { bg: None, fg, ..separator }, room);
        }
        // A zero length rests faint, like an empty field; hover and focus bring the digits back.
        let resting = !states.iter().any(|state| matches!(state, State::Hover | State::Focus));
        let empty = self.total() == 0 && resting && !all;
        let parts = parts(self.total());
        for (index, (offset, width)) in layout.segments.iter().enumerate() {
            let x = field.x + i32::from(*offset);
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
            let rect = Rect::new(x, field.y, *width, 1).intersect(field);
            if let Some(bg) = style.bg {
                cx.clear(rect, bg);
            }
            let digits = if index == 0 {
                format!("{:>width$}", parts[0], width = usize::from(width - 2))
            } else {
                format!("{:02}", parts[index])
            };
            let fg = if empty { separator.fg } else { style.fg.or(surface.fg) };
            let text_style = CellStyle { bg: None, fg, ..style };
            cx.text(x + 1, field.y, &digits, text_style, text::width(&digits).min(rect.width.saturating_sub(1)));
        }
        // Drawn last so a selected first segment keeps it; the digits never slide.
        if let Some(color) = pillar {
            cx.pillar(field.x, field.y, color);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let all = cx.memory::<DurationMemory>().all;
        TextMenu::edit(cx.env(), all, cx.can_paste()).paint_overlay(cx, anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.active() {
            return false;
        }
        if self.menu_event(cx, event) {
            return true;
        }
        match event {
            Event::Key(key) if let Some(action) = Self::chord_action(key) => {
                self.apply(cx, action);
                true
            }
            Event::Paste(text) => self.paste(cx, text),
            Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) => {
                let up = mouse.kind == MouseKind::ScrollUp;
                let segment = self.wheel_target(cx, mouse.x);
                self.step(cx, segment, up);
                true
            }
            Event::Key(key) => self.key(cx, key),
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Left) => {
                cx.memory::<DurationMemory>().all = false;
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
