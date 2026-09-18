//! Date picker: a field that opens a month calendar as a layer.

use std::time::Duration;

use super::cells;
use super::layer::PointerGate;
use super::placement::{self, Placement};
use crate::date::{Date, Weekday};
use crate::event::{Event, KeyEvent, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::{Key, Modifiers};
use crate::motion::Easing;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// Width of one day column.
const CELL: u16 = 4;

/// Rows of weeks; always six so the calendar keeps its height from month to month.
const WEEKS: u16 = 6;

/// Title row, a spare row and the weekday row above the weeks.
const HEADER_ROWS: u16 = 3;

/// Cells of a month arrow beside the title: the glyph with a cell on each side, all of them the
/// press target.
const ARROW: u16 = 3;

/// A field showing a chosen date that opens a calendar to choose another.
///
/// Closed: Enter, Space, ↓ or a click opens it. Open: ←/→ move a day, ↑/↓ a week, PgUp/PgDn a
/// month, Shift+PgUp/PgDn a year, Home/End go to the ends of the week, Enter or Space chooses,
/// Esc or a click elsewhere closes; that click still reaches what it landed on, and a click on
/// the field while open only closes it. The arrows beside the title and the mouse wheel change the
/// month (each arrow is three cells that light up under the pointer). One day is highlighted:
/// the keyboard moves it and so does the pointer once it moves. Whatever the pointer is over, the
/// field, an arrow or a day, shows the pillar `▌` in its leftmost cell, a column that is always
/// blank, so nothing in the date picker slides whatever [`Env::slide`](crate::env::Env::slide)
/// says. The highlighted day's pillar breathes only while the keyboard moved it last. Month and
/// weekday names, the first day of the week and the field format come from the `quvyta.date`
/// locale keys.
///
/// The field uses the `select` styles. The calendar uses `calendar` (`bg`, `padding`),
/// `calendar-title`, `calendar-arrow` with `hover` (`bg` over its three cells, `pillar`),
/// `calendar-weekday`, and `calendar-day` with `hover` (the highlighted day, `pillar`), `focus` (added
/// to `hover` while the keyboard moved the highlight), `selected`, variants `outside` and `today`.
pub struct DatePicker<Msg> {
    value: Option<Date>,
    today: Option<Date>,
    placeholder: String,
    disabled: bool,
    on_change: Option<Box<dyn Fn(Date) -> Msg>>,
}

#[derive(Debug, Default)]
struct DatePickerMemory {
    open: bool,
    opened_at: Duration,
    cursor: Option<Date>,
    popup: Rect,
    /// The pointer moves the one highlighted day only when it moves.
    pointer: PointerGate,
    /// A day of a neighbouring month under the pointer: it is the highlighted day without
    /// turning the calendar to its month.
    pointed: Option<Date>,
    /// The pointer, not the keyboard, put the highlight where it is: its pillar stays calm.
    by_pointer: bool,
}

impl<Msg: 'static> DatePicker<Msg> {
    /// A date picker showing `value`.
    #[must_use]
    pub fn new(value: Option<Date>) -> Self {
        Self { value, today: None, placeholder: String::new(), disabled: false, on_change: None }
    }

    /// Faint text shown while no date is chosen.
    #[must_use]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// The day marked as today; by default [`Date::today_local`], the day on the machine's own
    /// clock and time zone, so the mark does not jump a day early or late around midnight.
    #[must_use]
    pub fn today(mut self, today: Date) -> Self {
        self.today = Some(today);
        self
    }

    /// Greys the field out; it cannot be opened.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for choosing a different date.
    #[must_use]
    pub fn on_change(mut self, message: impl Fn(Date) -> Msg + 'static) -> Self {
        self.on_change = Some(Box::new(message));
        self
    }

    fn today_or_clock(&self) -> Date {
        self.today.unwrap_or_else(Date::today_local)
    }

    fn open(&self, cx: &mut EventCx<'_, Msg>) {
        let now = cx.now();
        let cursor = self.value.unwrap_or_else(|| self.today_or_clock());
        let pointer = cx.interaction.pointer;
        let memory = cx.memory::<DatePickerMemory>();
        memory.open = true;
        memory.opened_at = now;
        memory.cursor = Some(cursor);
        memory.pointer = PointerGate::new(pointer);
        memory.pointed = None;
        memory.by_pointer = false;
        cx.capture_keys(true);
    }

    fn close(cx: &mut EventCx<'_, Msg>) {
        cx.memory::<DatePickerMemory>().open = false;
        cx.capture_keys(false);
    }

    fn choose(&self, cx: &mut EventCx<'_, Msg>, date: Date) {
        Self::close(cx);
        cx.flash();
        if Some(date) != self.value
            && let Some(message) = &self.on_change
        {
            cx.emit(message(date));
        }
    }

    fn cursor(&self, cx: &mut EventCx<'_, Msg>) -> Date {
        let fallback = self.value.unwrap_or_else(|| self.today_or_clock());
        *cx.memory::<DatePickerMemory>().cursor.get_or_insert(fallback)
    }

    fn key(&self, cx: &mut EventCx<'_, Msg>, key: &KeyEvent) -> bool {
        // Keys move on from the day the keyboard or the pointer last put the highlight on.
        let memory = cx.memory::<DatePickerMemory>();
        memory.pointed = None;
        memory.by_pointer = false;
        let cursor = self.cursor(cx);
        let shift = Modifiers { shift: true, ..Modifiers::default() };
        let first = first_weekday(cx.env().i18n());
        let moved = if key.is_plain(Key::Left) {
            Some(cursor.add_days(-1))
        } else if key.is_plain(Key::Right) {
            Some(cursor.add_days(1))
        } else if key.is_plain(Key::Up) {
            Some(cursor.add_days(-7))
        } else if key.is_plain(Key::Down) {
            Some(cursor.add_days(7))
        } else if key.is_plain(Key::PageUp) {
            Some(cursor.add_months(-1))
        } else if key.is_plain(Key::PageDown) {
            Some(cursor.add_months(1))
        } else if key.chord.mods == shift && key.chord.key == Key::PageUp {
            Some(cursor.add_months(-12))
        } else if key.chord.mods == shift && key.chord.key == Key::PageDown {
            Some(cursor.add_months(12))
        } else if key.is_plain(Key::Home) {
            Some(cursor.start_of_week(first))
        } else if key.is_plain(Key::End) {
            Some(cursor.start_of_week(first).add_days(6))
        } else {
            None
        };
        if let Some(date) = moved {
            cx.memory::<DatePickerMemory>().cursor = Some(date);
        } else if key.is_plain(Key::Enter) || key.is_plain(Key::Space) {
            self.choose(cx, cursor);
        } else if key.is_plain(Key::Esc) {
            Self::close(cx);
        } else if key.is_plain(Key::Tab) {
            Self::close(cx);
            return false;
        }
        true
    }

    fn click_calendar(&self, cx: &mut EventCx<'_, Msg>, popup: Rect, x: i32, y: i32) {
        let cursor = self.cursor(cx);
        let inner = popup.inset(calendar_padding(cx.env()));
        if y == inner.y {
            if x < inner.x + i32::from(ARROW) {
                cx.memory::<DatePickerMemory>().cursor = Some(cursor.add_months(-1));
            } else if x >= inner.right() - i32::from(ARROW) {
                cx.memory::<DatePickerMemory>().cursor = Some(cursor.add_months(1));
            }
            return;
        }
        let row = y - inner.y - i32::from(HEADER_ROWS);
        let column = (x - inner.x) / i32::from(CELL);
        if !(0..i32::from(WEEKS)).contains(&row) || !(0..7).contains(&column) || x < inner.x {
            return;
        }
        let date = grid_start(cursor, first_weekday(cx.env().i18n())).add_days(i64::from(row * 7 + column));
        self.choose(cx, date);
    }
}

/// The first day of the week for the active language. Reads the environment's translator
/// directly because events are handled outside the translation scope of painting.
fn first_weekday(i18n: &crate::i18n::I18n) -> Weekday {
    let number = i18n.translate("quvyta.date.first-weekday", &[]);
    number.trim().parse::<u8>().ok().and_then(Weekday::from_number).unwrap_or(Weekday::Monday)
}

/// The first day shown for the month of `cursor`.
fn grid_start(cursor: Date, first: Weekday) -> Date {
    cursor.first_of_month().start_of_week(first)
}

fn month_name(month: u8) -> String {
    crate::t!(&format!("quvyta.date.month-{month}"))
}

/// `date` written the way the active language writes dates.
fn format_date(date: Date) -> String {
    crate::t!("quvyta.date.format", day = u32::from(date.day()), month = month_name(date.month()), year = date.year())
}

fn calendar_padding(env: &crate::env::Env) -> crate::geometry::Padding {
    crate::style::WidgetStyle::new(env.theme().style("calendar", None, &[]), 0.0).padding()
}

impl<Msg: 'static> Widget<Msg> for DatePicker<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let style = cx.env().theme().style("select", None, &[]);
        let (vertical, horizontal) = style.pair("padding").unwrap_or((0, 1));
        let sample = self.value.map_or(0, |date| text::width(&format_date(date)));
        let longest = sample.max(text::width(&self.placeholder)).max(12);
        Size::new(cells::sum([longest, 3, horizontal.saturating_mul(2)]), vertical.saturating_mul(2).saturating_add(1))
            .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let open = cx.memory::<DatePickerMemory>().open;
        let mut states = if self.disabled { vec![State::Disabled] } else { cx.pressable_states() };
        if open {
            states.push(State::Active);
        }
        let label = self.value.map(format_date);
        super::select::paint_field(cx, area, &states, label.as_deref(), &self.placeholder);
        if !self.disabled {
            cx.register_hit(area);
        }
        if open && !self.disabled {
            cx.request_overlay(area);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        let style = cx.style("calendar", None, &[]);
        let padding = style.padding();
        let background = style.text().bg.unwrap_or_else(|| cx.color("overlay"));
        let size = Size::new(
            (CELL * 7).saturating_add(padding.horizontal()),
            (HEADER_ROWS + WEEKS).saturating_add(padding.vertical()),
        );
        let (full, side) = placement::place(anchor, size, cx.clip(), Placement::Below);
        let (opened_at, remembered) = {
            let memory = cx.memory::<DatePickerMemory>();
            memory.popup = full;
            (memory.opened_at, memory.cursor)
        };
        let enter = cx.env().theme().motion().enter;
        let progress = cx.progress_since(opened_at, enter, Easing::EaseOut);
        let shown = placement::unfold(full, side, progress);
        let grounds = cx.grounds_around(shown);
        cx.clear(shown, background);
        cx.register_hit(shown);
        let mut cursor = remembered.or(self.value).unwrap_or_else(|| self.today_or_clock());
        let today = self.today_or_clock();
        let first = first_weekday(cx.env().i18n());
        let inner = full.inset(padding);
        let pointer = cx.pointer();
        let weekday_row = inner.y + 2;
        let day_rect = |index: i64| {
            let (row, column) = (u16::try_from(index / 7).unwrap_or(0), u16::try_from(index % 7).unwrap_or(0));
            Rect::new(inner.x + i32::from(column * CELL), weekday_row + 1 + i32::from(row), CELL, 1)
        };
        let start = grid_start(cursor, first);
        let under_pointer = pointer
            .and_then(|(x, y)| (0..i64::from(WEEKS * 7)).find(|index| day_rect(*index).contains(x, y)))
            .map(|index| start.add_days(index));
        let anywhere = cx.pointer_anywhere();
        let highlighted = {
            let memory = cx.memory::<DatePickerMemory>();
            if memory.pointer.moved(anywhere) {
                memory.pointed = None;
                memory.by_pointer |= under_pointer.is_some();
                match under_pointer {
                    Some(date) if date.month() == cursor.month() => {
                        cursor = date;
                        memory.cursor = Some(date);
                    }
                    other => memory.pointed = other,
                }
            }
            (memory.pointed.unwrap_or(cursor), memory.by_pointer)
        };
        let (highlighted, by_pointer) = highlighted;
        // Only a highlight the keyboard moved counts as visible focus, so its pillar breathes.
        let keyboard_focus = !by_pointer && cx.is_focus_visible();
        cx.with_clip(shown, |cx| {
            // Title with the month arrows at both ends.
            let title = crate::t!("quvyta.date.title", month = month_name(cursor.month()), year = cursor.year());
            let title_style = cx.style("calendar-title", None, &[]).text();
            let title_width = text::width(&title);
            let title_x = inner.x + i32::from(inner.width.saturating_sub(title_width) / 2);
            cx.text(title_x, inner.y, &title, title_style, title_width);
            for (glyph, x) in [("chevron-left", inner.x), ("chevron-right", inner.right() - i32::from(ARROW))] {
                let area = Rect::new(x, inner.y, ARROW, 1);
                let arrow = cx.env().icons().glyph(glyph).into_owned();
                let hovered = pointer.is_some_and(|(px, py)| area.contains(px, py));
                let states = if hovered { vec![State::Hover] } else { Vec::new() };
                let arrow_style = cx.style("calendar-arrow", None, &states);
                let pillar = arrow_style.color("pillar");
                let arrow_style = arrow_style.text();
                if let Some(bg) = arrow_style.bg {
                    cx.fill(area, bg);
                }
                // The arrow's leftmost cell is blank padding around the glyph, so the pillar fits.
                if let Some(color) = pillar {
                    cx.pillar(x, inner.y, color);
                }
                cx.text(x + 1, inner.y, &arrow, CellStyle { bg: None, ..arrow_style }, text::width(&arrow));
            }

            let weekday_style = cx.style("calendar-weekday", None, &[]).text();
            for column in 0..7u8 {
                let weekday = Weekday::from_number((first.number() - 1 + column) % 7 + 1).unwrap_or(Weekday::Monday);
                let name = crate::t!(&format!("quvyta.date.weekday-{}", weekday.number()));
                let x = inner.x + i32::from(u16::from(column) * CELL) + 1;
                cx.text(x, weekday_row, &name, weekday_style, CELL - 1);
            }

            let start = grid_start(cursor, first);
            for index in 0..i64::from(WEEKS * 7) {
                let date = start.add_days(index);
                let cell = day_rect(index);
                let mut states = Vec::new();
                if date == highlighted {
                    states.push(State::Hover);
                    if keyboard_focus {
                        states.push(State::Focus);
                    }
                }
                if Some(date) == self.value {
                    states.push(State::Selected);
                }
                let variant = if date.month() != cursor.month() {
                    Some("outside")
                } else if date == today {
                    Some("today")
                } else {
                    None
                };
                let day_style = cx.style("calendar-day", variant, &states);
                let pillar = day_style.color("pillar");
                let day_style = day_style.text();
                if let Some(bg) = day_style.bg {
                    cx.fill(cell, bg);
                }
                // Every day cell keeps its first column blank for the pillar: the number never moves.
                if let Some(color) = pillar {
                    cx.pillar(cell.x, cell.y, color);
                }
                let label = format!("{:>2}", date.day());
                cx.text(cell.x + 1, cell.y, &label, CellStyle { bg: None, ..day_style }, 2);
            }
        });
        cx.stand_apart(shown, &grounds, Some(background));
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.disabled {
            return false;
        }
        let open = cx.memory::<DatePickerMemory>().open;
        match event {
            Event::PointerOutside => {
                Self::close(cx);
                true
            }
            Event::Key(key) if !open => {
                let opens = key.is_plain(Key::Enter) || key.is_plain(Key::Space) || key.is_plain(Key::Down);
                if opens {
                    self.open(cx);
                }
                opens
            }
            Event::Key(key) => self.key(cx, key),
            Event::Mouse(mouse) => {
                let popup = cx.memory::<DatePickerMemory>().popup;
                let in_popup = open && popup.contains(mouse.x, mouse.y);
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) if in_popup => {
                        self.click_calendar(cx, popup, mouse.x, mouse.y);
                        true
                    }
                    MouseKind::Down(MouseButton::Left) => {
                        if open {
                            Self::close(cx);
                        } else {
                            self.open(cx);
                        }
                        true
                    }
                    MouseKind::ScrollUp | MouseKind::ScrollDown if in_popup => {
                        let months = if mouse.kind == MouseKind::ScrollUp { -1 } else { 1 };
                        let cursor = self.cursor(cx);
                        cx.memory::<DatePickerMemory>().cursor = Some(cursor.add_months(months));
                        true
                    }
                    _ => false,
                }
            }
            Event::Paste(_) => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    struct Demo {
        date: Option<Date>,
    }

    impl App for Demo {
        type Msg = Date;
        fn update(&mut self, date: Date) -> Command<Date> {
            self.date = Some(date);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Date>) {
            let today = Date::new(2026, 9, 16).expect("valid");
            ui.add(DatePicker::new(self.date).today(today).placeholder("Release date").on_change(|date| date))
                .width(Length::Cells(24))
                .id("date");
        }
    }

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).expect("valid")
    }

    #[test]
    fn opens_a_calendar_with_weeks_from_the_locale() {
        let mut h = Harness::new(Demo { date: Some(date(2026, 9, 3)) }, 40, 14);
        h.set_reduced_motion(true);
        assert!(h.screen().starts_with("  September 3, 2026"), "{}", h.screen());
        h.press("tab").press("enter");
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(lines[2], "   ◀     September 2026     ▶", "{screen}");
        assert_eq!(lines[4], "   Su  Mo  Tu  We  Th  Fr  Sa");
        // The highlighted day (the chosen 3rd) shows the pillar in its blank first column.
        assert_eq!(lines[5], "   30  31   1   2 ▌ 3   4   5");
        assert_eq!(lines[10], "    4   5   6   7   8   9  10");
        let theme = h.env().theme();
        // The chosen day is filled with the accent; days of other months are faint.
        assert_eq!(h.bg(18, 5), theme.color("accent"));
        assert_eq!(h.fg(20, 5), theme.color("ink"));
        assert_eq!(h.fg(4, 5), theme.color("muted"));
    }

    #[test]
    fn today_is_marked_and_turkish_weeks_start_on_monday_with_names() {
        let mut h = Harness::new(Demo { date: None }, 40, 14);
        h.set_reduced_motion(true).set_locale("tr").press("tab").press("enter");
        let screen = h.screen();
        assert!(screen.contains("Eylül 2026"), "{screen}");
        assert!(screen.contains("Pt  Sa  Ça  Pe  Cu  Ct  Pz"), "{screen}");
        let (x, y) = h.find("16").expect("today shown");
        let theme = h.env().theme();
        assert_eq!(h.fg(u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0)), theme.color("accent"));
    }

    #[test]
    fn today_is_the_local_day_unless_given() {
        // Read the local day on both sides, so a midnight passing in between cannot fail the test.
        let before = Date::today_local();
        let marked = DatePicker::<Date>::new(None).today_or_clock();
        let after = Date::today_local();
        assert!(marked == before || marked == after, "{marked} is not the local day {before}");
        let given = date(2026, 9, 16);
        assert_eq!(DatePicker::<Date>::new(None).today(given).today_or_clock(), given);
    }

    #[test]
    fn english_weeks_start_on_sunday() {
        let mut h = Harness::new(Demo { date: None }, 40, 14);
        h.set_reduced_motion(true).press("tab").press("enter");
        assert!(h.screen().contains("Su  Mo  Tu"), "{}", h.screen());
    }

    #[test]
    fn keyboard_moves_by_day_week_month_and_week_ends() {
        let mut h = Harness::new(Demo { date: Some(date(2026, 1, 31)) }, 40, 14);
        h.set_reduced_motion(true).press("tab").press("enter");
        h.press("pgdn").press("enter");
        assert_eq!(h.app().date, Some(date(2026, 2, 28)));
        h.press("enter").press("right").press("down").press("enter");
        assert_eq!(h.app().date, Some(date(2026, 3, 8)));
        h.press("enter").press("right").press("right").press("home").press("enter");
        assert_eq!(h.app().date, Some(date(2026, 3, 8)), "English weeks start on Sunday");
        h.press("enter").press("end").press("left").press("up").press("enter");
        assert_eq!(h.app().date, Some(date(2026, 3, 6)));
        h.press("enter").press("shift+pgup").press("enter");
        assert_eq!(h.app().date, Some(date(2025, 3, 6)));
        h.press("enter").press("esc");
        assert!(!h.screen().contains("Mo  Tu"));
    }

    #[test]
    fn clicks_choose_days_and_arrows_change_the_month() {
        let mut h = Harness::new(Demo { date: Some(date(2026, 9, 3)) }, 40, 14);
        h.set_reduced_motion(true);
        h.click(3, 0);
        let (x, y) = h.find("▶").expect("next month arrow");
        h.click(x, y);
        assert!(h.screen().contains("October 2026"), "{}", h.screen());
        h.click_text("15");
        assert_eq!(h.app().date, Some(date(2026, 10, 15)));
        assert!(h.screen().contains("October 15, 2026"), "{}", h.screen());
    }

    #[test]
    fn unfolds_over_motion_enter() {
        let mut h = Harness::new(Demo { date: None }, 40, 14);
        h.press("tab").press("enter");
        assert!(!h.screen().contains("Mo"));
        h.advance(Duration::from_millis(300));
        assert!(h.screen().contains("Mo"));
    }

    // One highlight, and the mouse does what the keys do.

    /// Days whose cell carries the highlight surface.
    fn lit_days(h: &Harness<Demo>) -> Vec<String> {
        let active = h.env().theme().color("active");
        let screen = h.screen();
        let mut days = Vec::new();
        for (y, line) in screen.lines().enumerate().skip(5) {
            // Day cells are four wide from the calendar's inner edge, two cells in.
            for (column, day) in line.chars().skip(2).collect::<Vec<_>>().chunks(4).enumerate() {
                let x = u16::try_from(column * 4 + 3).unwrap_or(0);
                if u16::try_from(y).is_ok_and(|y| h.bg(x, y) == active) {
                    days.push(day.iter().filter(|c| **c != '▌').collect::<String>().trim().to_owned());
                }
            }
        }
        days
    }

    #[test]
    fn the_pointer_moves_the_one_highlighted_day() {
        let mut h = Harness::new(Demo { date: None }, 40, 14);
        h.set_reduced_motion(true).press("tab").press("enter");
        assert_eq!(lit_days(&h), ["16"], "today has the keyboard highlight:\n{}", h.screen());
        let (x, y) = h.find("22").expect("a day");
        h.hover(x, y);
        assert_eq!(lit_days(&h), ["22"], "only the hovered day is lit:\n{}", h.screen());
        h.press("right");
        assert_eq!(lit_days(&h), ["23"], "the keyboard goes on from it and the resting pointer waits");
        h.press("enter");
        assert_eq!(h.app().date, Some(date(2026, 9, 23)));
    }

    #[test]
    fn a_neighbouring_months_day_under_the_pointer_is_lit_without_turning_the_month() {
        let mut h = Harness::new(Demo { date: Some(date(2026, 9, 3)) }, 40, 14);
        h.set_reduced_motion(true).press("tab").press("enter");
        let (x, y) = h.find("30").expect("August 30");
        h.hover(x, y);
        assert!(h.screen().contains("September 2026"), "{}", h.screen());
        assert_eq!(lit_days(&h), ["30"], "{}", h.screen());
        h.press("right");
        assert_eq!(lit_days(&h), ["4"], "keys continue from the September day");
    }

    #[test]
    fn month_arrows_are_three_cell_buttons_and_the_wheel_turns_the_month() {
        let mut h = Harness::new(Demo { date: Some(date(2026, 9, 3)) }, 40, 14);
        h.set_reduced_motion(true).press("tab").press("enter");
        let (x, y) = h.find("▶").expect("next month arrow");
        let (column, row) = (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"));
        let resting = h.bg(column, row);
        h.hover(x + 1, y);
        let lit = h.env().theme().color("active");
        assert_ne!(resting, lit);
        assert_eq!([h.bg(column - 1, row), h.bg(column, row), h.bg(column + 1, row)], [lit, lit, lit]);
        h.click(x - 1, y);
        assert!(h.screen().contains("October 2026"), "{}", h.screen());
        let (x, y) = h.find("◀").expect("previous month arrow");
        h.click(x + 1, y);
        assert!(h.screen().contains("September 2026"), "{}", h.screen());
        let (x, y) = h.find("15").expect("a day");
        h.mouse(MouseKind::ScrollDown, x, y);
        assert!(h.screen().contains("October 2026"), "{}", h.screen());
        h.mouse(MouseKind::ScrollUp, x, y).mouse(MouseKind::ScrollUp, x, y);
        assert!(h.screen().contains("August 2026"), "{}", h.screen());
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
        assert!(!h.screen().contains('▶'), "{}", h.screen());
    }

    // Every hovered part shows the pillar in its leftmost cell and nothing slides, whether the
    // slide setting is on or off.

    /// A harness with the slide setting forced `on` or off, reduced motion so layers open at once.
    fn harness(value: Option<Date>, slide: bool, width: u16) -> Harness<Demo> {
        let mut env = crate::env::Env::builtin();
        env.set_slide(slide);
        let mut h = Harness::with_env(Demo { date: value }, env, width, 14);
        h.set_reduced_motion(true);
        h
    }

    /// The pillar colour a calendar style gives in `states` at pulse phase zero.
    fn pillar_of(h: &Harness<Demo>, widget: &str, states: &[State]) -> Option<crate::color::Rgb> {
        crate::style::WidgetStyle::new(h.env().theme().style(widget, None, states), 0.0).color("pillar")
    }

    fn cell(x: i32, y: i32) -> (u16, u16) {
        (u16::try_from(x).expect("x"), u16::try_from(y).expect("y"))
    }

    /// The open calendar of September 2026 as the tests below see it, with `pillars` drawn over
    /// the resting screen at `(column, row)`.
    fn calendar_with(pillars: &[(usize, usize)]) -> String {
        let mut lines: Vec<Vec<char>> = [
            "▌ September 3, 2026  ▾",
            "",
            "   ◀     September 2026     ▶",
            "",
            "   Su  Mo  Tu  We  Th  Fr  Sa",
            "   30  31   1   2   3   4   5",
            "    6   7   8   9  10  11  12",
            "   13  14  15  16  17  18  19",
            "   20  21  22  23  24  25  26",
            "   27  28  29  30   1   2   3",
            "    4   5   6   7   8   9  10",
            "",
            "",
            "",
        ]
        .iter()
        .map(|line| line.chars().collect())
        .collect();
        for &(column, row) in pillars {
            let line = &mut lines[row];
            assert_eq!(line.get(column), Some(&' '), "a pillar only takes a blank cell");
            line[column] = '▌';
        }
        lines.iter().map(|line| line.iter().collect::<String>()).collect::<Vec<_>>().join("\n") + "\n"
    }

    #[test]
    fn a_hovered_day_shows_the_pillar_in_its_blank_first_column_and_never_slides() {
        for slide in [false, true] {
            let mut h = harness(Some(date(2026, 9, 3)), slide, 40);
            h.click(3, 0);
            // Opened with a click: the chosen day carries the one highlight and its calm pillar.
            assert_eq!(h.screen(), calendar_with(&[(18, 5)]), "slide {slide}");
            let (x, y) = h.find("22").expect("a day");
            h.hover(x + 1, y);
            // The pillar moves to the hovered day's first column; every number stays in place.
            assert_eq!(h.screen(), calendar_with(&[(10, 8)]), "slide {slide}");
            let soft = pillar_of(&h, "calendar-day", &[State::Hover]);
            assert!(soft.is_some());
            assert_eq!(h.fg(10, 8), soft, "a pointer highlight has the soft pillar");
            assert_eq!(h.bg(10, 8), h.bg(11, 8), "the pillar sits on the day's lit surface");
            // Hovering the chosen day keeps its accent fill and adds an ink pillar.
            h.hover(20, 5);
            assert_eq!(h.screen(), calendar_with(&[(18, 5)]), "slide {slide}");
            assert_eq!(h.bg(19, 5), h.env().theme().color("accent"));
            assert_eq!(h.fg(18, 5), pillar_of(&h, "calendar-day", &[State::Hover, State::Selected]));
        }
    }

    #[test]
    fn a_hovered_month_arrow_shows_the_pillar_in_its_first_cell() {
        for slide in [false, true] {
            let mut h = harness(Some(date(2026, 9, 3)), slide, 40);
            h.click(3, 0);
            let (x, y) = h.find("◀").expect("previous month arrow");
            h.hover(x, y);
            assert_eq!(h.screen(), calendar_with(&[(2, 2), (18, 5)]), "slide {slide}");
            let soft = pillar_of(&h, "calendar-arrow", &[State::Hover]);
            assert!(soft.is_some());
            assert_eq!(h.fg(2, 2), soft);
            let (x, y) = h.find("▶").expect("next month arrow");
            h.hover(x + 1, y);
            assert_eq!(h.screen(), calendar_with(&[(27, 2), (18, 5)]), "slide {slide}");
            assert_eq!(h.fg(27, 2), soft);
        }
    }

    #[test]
    fn a_hovered_field_shows_the_pillar_and_its_label_stays() {
        for slide in [false, true] {
            let mut h = harness(Some(date(2026, 9, 3)), slide, 40);
            assert_eq!(h.screen().lines().next(), Some("  September 3, 2026  ▾"), "slide {slide}");
            h.hover(10, 0);
            assert_eq!(h.screen().lines().next(), Some("▌ September 3, 2026  ▾"), "slide {slide}");
        }
    }

    #[test]
    fn only_a_highlight_the_keyboard_moved_breathes() {
        let mut h = harness(Some(date(2026, 9, 3)), true, 40);
        h.press("tab").press("enter").press("right");
        assert_eq!(h.screen(), calendar_with(&[(22, 5)]));
        let breathing = pillar_of(&h, "calendar-day", &[State::Hover, State::Focus]);
        let soft = pillar_of(&h, "calendar-day", &[State::Hover]);
        assert_ne!(breathing, soft);
        assert_eq!(h.fg(22, 5), breathing, "keyboard focus breathes");
        let breathes = h.env().theme().style("calendar-day", None, &[State::Hover, State::Focus]);
        assert!(crate::style::WidgetStyle::new(breathes, 0.0).is_animated());
        // The pointer takes the highlight over: its day stays calm although focus came from keys.
        let (x, y) = h.find("22").expect("a day");
        h.hover(x, y);
        assert_eq!(h.screen(), calendar_with(&[(10, 8)]));
        assert_eq!(h.fg(10, 8), soft);
        h.press("left");
        assert_eq!(h.screen(), calendar_with(&[(6, 8)]));
        assert_eq!(h.fg(6, 8), breathing);
    }

    #[test]
    fn narrow_screens_keep_the_pillar_in_the_first_column() {
        let mut h = harness(Some(date(2026, 9, 3)), true, 26);
        h.click(3, 0);
        let (x, y) = h.find("22").expect("a day");
        h.hover(x, y);
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(lines[0], "▌ September 3, 2026  ▾", "{screen}");
        assert_eq!(lines[7], "   13  14  15  16  17  18", "{screen}");
        assert_eq!(lines[8], "   20  21 ▌22  23  24  25", "{screen}");
        let (x, y) = h.find("◀").expect("previous month arrow");
        h.hover(x, y);
        assert_eq!(h.screen().lines().nth(2), Some("  ▌◀  September 2026  ▶"), "{}", h.screen());
    }

    #[test]
    fn ascii_mode_draws_the_pillar_as_a_filled_cell() {
        let mut h = harness(Some(date(2026, 9, 3)), false, 40);
        h.set_glyph_mode(crate::icons::GlyphMode::Ascii).click(3, 0);
        let (x, y) = h.find("22").expect("a day");
        h.hover(x, y);
        let (column, row) = cell(x - 1, y);
        assert_eq!(h.bg(column, row), pillar_of(&h, "calendar-day", &[State::Hover]));
        assert_eq!(h.screen().lines().nth(8), Some("   20  21  22  23  24  25  26"), "{}", h.screen());
    }
}
