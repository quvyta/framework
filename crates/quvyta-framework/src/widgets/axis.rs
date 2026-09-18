//! Axes: a row of labels along the edge of a chart — hours of a day, weekdays, months, or names
//! of your own — thinned so they never collide.

use crate::date::{TimeOfDay, Weekday};
use crate::geometry::{Rect, Size};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Seconds in a day.
pub(crate) const DAY: u32 = 86_400;

/// Minutes between two hour labels, from the finest to the coarsest. Every step divides a day,
/// so the labels fall on the same clock times whatever the range starts at.
const HOUR_STEPS: [u32; 9] = [15, 30, 60, 120, 180, 240, 360, 720, 1_440];

/// The shortest step that writes whole hours only, where the short form of a label can stand.
const WHOLE_HOURS: u32 = 60;

/// Seconds from `from` forward to `to` on a clock: into the next day when `to` is not after
/// `from`, and a whole day when the two are the same moment.
pub(crate) fn span_between(from: TimeOfDay, to: TimeOfDay) -> u32 {
    let span = (to.seconds_since_midnight() + DAY - from.seconds_since_midnight()) % DAY;
    if span == 0 { DAY } else { span }
}

/// The cell, counted from the left edge of `width` cells, that `offset` seconds into a range of
/// `span` seconds falls in. A timeline and its axis both place time with this, so a label sits
/// over the cell its time is drawn in.
pub(crate) fn time_cell(offset: u32, span: u32, width: u16) -> i32 {
    let cell = u64::from(offset) * u64::from(width) / u64::from(span.max(1));
    i32::try_from(cell).unwrap_or(i32::MAX)
}

/// What the labels of an axis stand for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// A stretch of a day: `span` seconds from `from` seconds after midnight.
    Hours { from: u32, span: u32 },
    /// Consecutive days starting at `first`.
    Weekdays { first: Weekday, count: usize },
    /// Consecutive months starting at `first` (1 January to 12 December).
    Months { first: u8, count: usize },
    /// Names the caller gives, one per slot.
    Labels(Vec<String>),
}

/// A row of labels along the edge of a chart: hours of a day, weekdays, months, or names of your
/// own.
///
/// Hours are placed by time: a label starts in the cell its clock time falls in, the way a
/// [`Timeline`](super::Timeline) places a block, so an axis under a strip of the same range reads
/// straight down. Weekdays, months and names of your own are placed in slots: the width is shared
/// out into one slot per label, with [`gap`](Self::gap) empty cells between slots, and each label
/// is centred in its slot. A vertical [`BarChart`](super::BarChart) shares its width out the same
/// way, so an axis with the chart's gap stands under its bars.
///
/// Names come from the active language — `quvyta.date.weekday-long-*`, `weekday-*`, `month-*`,
/// `month-short-*`, and `quvyta.time.clock` and `hour` for times — never from the source.
///
/// Labels never collide and are never cut. Weekdays and months write every name in full while
/// all of them fit, then every name in its short form, then the short form of every second,
/// third, … name. Hours take the finest step whose labels fit — a quarter hour, half an hour, an
/// hour, then 2, 3, 4, 6, 12 hours and a day, always on round clock times — in the long form
/// (`09:30`), and fall back to the short form (`09`) only when the long one leaves fewer than two
/// labels, which gives a reader no scale. At least one cell stays empty between two labels, and a
/// label that would run past the right edge is left out rather than cut. An area too narrow for a
/// single label writes nothing.
///
/// An axis is text only, so it reads the same in every glyph mode; it takes no focus and sends
/// no messages.
///
/// Style keys: `axis` (`fg`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Axis {
    kind: Kind,
    gap: u16,
    faint: bool,
}

impl Axis {
    /// The hours from `from` to `to`: into the next day when `to` is not after `from`, a whole
    /// day when the two are the same.
    #[must_use]
    pub fn hours(from: TimeOfDay, to: TimeOfDay) -> Self {
        Self::of(Kind::Hours { from: from.seconds_since_midnight(), span: span_between(from, to) })
    }

    /// `count` weekdays from `first` on, wrapping after Sunday.
    #[must_use]
    pub fn weekdays(first: Weekday, count: usize) -> Self {
        Self::of(Kind::Weekdays { first, count })
    }

    /// `count` months from `first` on (1 for January to 12 for December), wrapping after
    /// December. A `first` outside 1 to 12 is taken as the nearest month.
    #[must_use]
    pub fn months(first: u8, count: usize) -> Self {
        Self::of(Kind::Months { first: first.clamp(1, 12), count })
    }

    /// Names of your own, one slot each, e.g. the weeks of a quarter. A name with no room in the
    /// active width is left out, never cut.
    #[must_use]
    pub fn labels(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::of(Kind::Labels(labels.into_iter().map(Into::into).collect()))
    }

    /// Empty cells between two slots; none by default. Give it the gap of the chart above so the
    /// labels stand under its bars. Hours are placed by time and have no slots, so they ignore it.
    #[must_use]
    pub fn gap(mut self, cells: u16) -> Self {
        self.gap = cells;
        self
    }

    fn of(kind: Kind) -> Self {
        Self { kind, gap: 0, faint: false }
    }

    /// Draws the labels in the faint tone, for the axis of a disabled chart.
    pub(crate) fn faint(mut self, faint: bool) -> Self {
        self.faint = faint;
        self
    }

    /// How many labels the axis could write before thinning.
    fn count(&self) -> usize {
        match &self.kind {
            Kind::Hours { .. } => 1,
            Kind::Weekdays { count, .. } | Kind::Months { count, .. } => *count,
            Kind::Labels(labels) => labels.len(),
        }
    }

    /// The labels written in `width` cells: where each starts, counted from the left edge, and
    /// its text. Reads names from the active language.
    fn place(&self, width: u16) -> Vec<(i32, String)> {
        match &self.kind {
            Kind::Hours { from, span } => hour_labels(*from, *span, width),
            Kind::Weekdays { first, count } => {
                let forms: Vec<Vec<String>> = (0..*count)
                    .map(|index| {
                        let day = (usize::from(first.number()) - 1 + index) % 7 + 1;
                        vec![
                            crate::t!(&format!("quvyta.date.weekday-long-{day}")),
                            crate::t!(&format!("quvyta.date.weekday-{day}")),
                        ]
                    })
                    .collect();
                self.slot_labels(&forms, width)
            }
            Kind::Months { first, count } => {
                let forms: Vec<Vec<String>> = (0..*count)
                    .map(|index| {
                        let month = (usize::from(*first) - 1 + index) % 12 + 1;
                        vec![
                            crate::t!(&format!("quvyta.date.month-{month}")),
                            crate::t!(&format!("quvyta.date.month-short-{month}")),
                        ]
                    })
                    .collect();
                self.slot_labels(&forms, width)
            }
            Kind::Labels(labels) => {
                let forms: Vec<Vec<String>> = labels.iter().map(|label| vec![label.clone()]).collect();
                self.slot_labels(&forms, width)
            }
        }
    }

    /// Labels centred in equal slots. Every label in its longest form while they all fit, then
    /// every label in each shorter form, then the shortest form of every second, third, … label.
    fn slot_labels(&self, forms: &[Vec<String>], width: u16) -> Vec<(i32, String)> {
        let count = forms.len();
        if count == 0 || width == 0 {
            return Vec::new();
        }
        let gaps = u16::try_from(count - 1).unwrap_or(u16::MAX).saturating_mul(self.gap);
        let slots = u16::try_from(count).unwrap_or(u16::MAX);
        let slot = (width.saturating_sub(gaps) / slots).max(1);
        let place = |index: usize, label: &str| {
            let start = i32::try_from(index).unwrap_or(i32::MAX).saturating_mul(i32::from(slot) + i32::from(self.gap));
            let label_width = i32::from(text::width(label));
            let x = start + (i32::from(slot) - label_width) / 2;
            // A label wider than its slot leans into its neighbours' room but stays inside the
            // axis; the collision check decides whether that room was free.
            (x.min(i32::from(width) - label_width).max(0), label.to_owned())
        };
        let longest = forms.iter().map(Vec::len).max().unwrap_or(0);
        for form in 0..longest {
            let labels: Vec<(i32, String)> =
                forms.iter().enumerate().map(|(index, f)| place(index, form_at(f, form))).collect();
            if fits(&labels, width) {
                return labels;
            }
        }
        let shortest = longest.saturating_sub(1);
        for step in 2..=count {
            let labels: Vec<(i32, String)> = forms
                .iter()
                .enumerate()
                .filter(|(index, _)| index % step == 0)
                .map(|(index, f)| place(index, form_at(f, shortest)))
                .collect();
            if fits(&labels, width) {
                return labels;
            }
        }
        Vec::new()
    }
}

/// The `form`-th form of a label, or its last one when it has fewer.
fn form_at(forms: &[String], form: usize) -> &str {
    forms.get(form).or_else(|| forms.last()).map_or("", String::as_str)
}

/// Whether `labels`, left to right, stay inside `width` cells with at least one empty cell
/// between two of them. No labels at all do not fit: there is nothing to show.
fn fits(labels: &[(i32, String)], width: u16) -> bool {
    if labels.is_empty() {
        return false;
    }
    let mut free_from = 0;
    for (x, label) in labels {
        let end = x + i32::from(text::width(label));
        if *x < free_from || end > i32::from(width) {
            return false;
        }
        free_from = end + 1;
    }
    true
}

/// The hour labels of `span` seconds from `from`: the finest step whose labels fit with the long
/// form (`09:30`). When that leaves fewer than two labels, which gives a reader no scale, the
/// finest whole-hour step of the short form (`09`) is taken instead if it writes more.
fn hour_labels(from: u32, span: u32, width: u16) -> Vec<(i32, String)> {
    let finest = |short: bool| {
        HOUR_STEPS
            .into_iter()
            .filter(|step| !short || *step >= WHOLE_HOURS)
            .map(|step| {
                ticks(from, span, step)
                    .map(|tick| (time_cell(tick - from, span, width), clock_label(tick % DAY, short)))
                    // A label that would run past the right edge is left out, not cut.
                    .filter(|(x, label)| x + i32::from(text::width(label)) <= i32::from(width))
                    .collect::<Vec<_>>()
            })
            .find(|labels| fits(labels, width))
            .unwrap_or_default()
    };
    let long = finest(false);
    if long.len() >= 2 {
        return long;
    }
    let short = finest(true);
    if short.len() > long.len() { short } else { long }
}

/// The clock times a `step` minutes apart that fall inside `span` seconds from `from`, in seconds
/// on a clock that runs on past midnight.
fn ticks(from: u32, span: u32, step: u32) -> impl Iterator<Item = u32> {
    let step = step * 60;
    let first = from.div_ceil(step) * step;
    (first..from + span).step_by(usize::try_from(step).unwrap_or(usize::MAX))
}

/// The label of a clock time: `09:30` in the long form, `09` in the short one.
pub(crate) fn clock_label(seconds: u32, short: bool) -> String {
    let time = TimeOfDay::from_seconds_since_midnight(seconds);
    let hour = format!("{:02}", time.hour);
    if short {
        crate::t!("quvyta.time.hour", hour = hour)
    } else {
        crate::t!("quvyta.time.clock", hour = hour, minute = format!("{:02}", time.minute))
    }
}

impl<Msg: 'static> Widget<Msg> for Axis {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        if self.count() == 0 {
            return Size::default();
        }
        Size::new(available.width, 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        let color = if self.faint {
            cx.color("muted")
        } else {
            cx.style("axis", None, &[]).color("fg").unwrap_or_else(|| cx.color("dim"))
        };
        let style = CellStyle::fg(color);
        for (x, label) in self.place(area.width) {
            let width = text::width(&label);
            cx.text(area.x + x, area.y, &label, style, width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};
    use crate::widgets::{Bar, BarChart};

    struct Demo(Axis);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(self.0.clone()).width(Length::Fill(1)).height(Length::Fill(1));
        }
    }

    fn screen(axis: Axis, width: u16) -> String {
        Harness::new(Demo(axis), width, 1).screen().trim_end_matches('\n').to_owned()
    }

    fn time(hour: u8, minute: u8) -> TimeOfDay {
        TimeOfDay::new(hour, minute, 0)
    }

    /// The words of an axis row, in order.
    fn words(row: &str) -> Vec<&str> {
        row.split_whitespace().collect()
    }

    #[test]
    fn a_day_writes_round_hours_that_fit() {
        let day = screen(Axis::hours(time(0, 0), time(0, 0)), 72);
        assert_eq!(
            words(&day),
            [
                "00:00", "02:00", "04:00", "06:00", "08:00", "10:00", "12:00", "14:00", "16:00", "18:00", "20:00",
                "22:00"
            ]
        );
        assert!(day.starts_with("00:00 "), "{day}");
        assert_eq!(day.find("12:00"), Some(36), "noon sits in the middle of a day: {day}");
    }

    #[test]
    fn narrower_days_thin_the_hours_and_then_shorten_them() {
        let wide = words(&screen(Axis::hours(time(0, 0), time(0, 0)), 48)).len();
        let narrow = screen(Axis::hours(time(0, 0), time(0, 0)), 24);
        assert!(wide < 12 && wide > 4, "48 cells thin the labels to a coarser step: {wide}");
        assert_eq!(words(&narrow), ["00:00", "06:00", "12:00", "18:00"]);
        let tiny = screen(Axis::hours(time(0, 0), time(0, 0)), 9);
        assert_eq!(words(&tiny), ["00", "12"], "too narrow for two long labels, so the hours go short");
        assert_eq!(words(&screen(Axis::hours(time(0, 0), time(0, 0)), 5)), ["00:00"], "one label is still a label");
        assert_eq!(screen(Axis::hours(time(0, 0), time(0, 0)), 1), "", "no room for a label writes nothing");
    }

    #[test]
    fn a_short_range_goes_down_to_the_quarter_hour() {
        let morning = screen(Axis::hours(time(9, 0), time(10, 0)), 40);
        assert_eq!(words(&morning), ["09:00", "09:15", "09:30", "09:45"]);
        let off_round = screen(Axis::hours(time(9, 10), time(12, 10)), 36);
        assert!(off_round.starts_with("  "), "a range that starts between labels leaves the start blank: {off_round}");
        assert_eq!(words(&off_round), ["09:30", "10:00", "10:30", "11:00", "11:30"], "12:00 would run past the edge");
    }

    #[test]
    fn a_range_across_midnight_counts_on_into_the_next_day() {
        let night = screen(Axis::hours(time(22, 0), time(6, 0)), 48);
        assert_eq!(words(&night), ["22:00", "23:00", "00:00", "01:00", "02:00", "03:00", "04:00", "05:00"]);
    }

    #[test]
    fn labels_never_collide_or_get_cut_at_any_width() {
        let axes = [
            Axis::hours(time(0, 0), time(0, 0)),
            Axis::hours(time(9, 10), time(11, 50)),
            Axis::weekdays(Weekday::Monday, 7),
            Axis::months(1, 12),
            Axis::labels(["first week", "second week", "third week"]),
        ];
        for axis in axes {
            for width in 0..90 {
                let h = Harness::new(Demo(axis.clone()), width.max(1), 1);
                let i18n = std::sync::Arc::new(crate::i18n::I18n::builtin());
                let labels = crate::i18n::scope(i18n, || axis.place(width));
                let row = h.screen();
                for (x, label) in &labels {
                    assert!(*x >= 0 && *x + i32::from(text::width(label)) <= i32::from(width), "{axis:?} at {width}");
                    if width > 0 {
                        assert!(row.contains(label.as_str()), "{label} is written whole at {width}: {row}");
                    }
                }
                for pair in labels.windows(2) {
                    let end = pair[0].0 + i32::from(text::width(&pair[0].1));
                    assert!(pair[1].0 > end, "{axis:?} at {width}: {pair:?} touch");
                }
                assert!(!row.contains('…'), "an axis never cuts a label: {row}");
            }
        }
    }

    #[test]
    fn weekdays_shorten_before_they_thin() {
        let long = screen(Axis::weekdays(Weekday::Monday, 7), 70);
        assert_eq!(words(&long), ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]);
        let short = screen(Axis::weekdays(Weekday::Monday, 7), 28);
        assert_eq!(words(&short), ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]);
        let thin = screen(Axis::weekdays(Weekday::Monday, 7), 14);
        assert_eq!(words(&thin), ["Mo", "We", "Fr", "Su"], "every second day when even the short names crowd");
        let thinner = screen(Axis::weekdays(Weekday::Monday, 7), 12);
        assert_eq!(words(&thinner), ["Mo", "Th", "Su"], "every third day when every second still touches");
        let from_sunday = screen(Axis::weekdays(Weekday::Sunday, 2), 20);
        assert_eq!(words(&from_sunday), ["Sunday", "Monday"], "the week wraps");
    }

    #[test]
    fn months_come_from_the_active_language() {
        let mut h = Harness::new(Demo(Axis::months(11, 3)), 40, 1);
        assert_eq!(words(h.screen().trim_end()), ["November", "December", "January"]);
        h.set_locale("tr");
        assert_eq!(words(h.screen().trim_end()), ["Kasım", "Aralık", "Ocak"]);
        let mut year = Harness::new(Demo(Axis::months(1, 12)), 60, 1);
        assert_eq!(
            words(year.screen().trim_end()),
            ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        );
        year.set_locale("tr");
        assert_eq!(words(year.screen().trim_end())[..3], ["Oca", "Şub", "Mar"]);
        let mut days = Harness::new(Demo(Axis::weekdays(Weekday::Monday, 2)), 30, 1);
        days.set_locale("tr");
        assert_eq!(words(days.screen().trim_end()), ["Pazartesi", "Salı"]);
    }

    #[test]
    fn slots_with_the_chart_gap_stand_under_the_bars() {
        struct Chart;
        impl App for Chart {
            type Msg = ();
            fn update(&mut self, _: ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                let bars = ["Mo", "Tu", "We", "Th", "Fr"].map(|day| Bar::new(day, 3.0));
                ui.add(BarChart::new(bars).vertical().gap(2)).width(Length::Fill(1)).height(Length::Cells(4));
                let days = ["Mo", "Tu", "We", "Th", "Fr"];
                ui.add(Axis::labels(days).gap(2)).width(Length::Fill(1)).height(Length::Cells(1));
            }
        }
        for width in [23, 30, 41, 56] {
            let h = Harness::new(Chart, width, 5);
            let screen = h.screen();
            let rows: Vec<&str> = screen.lines().collect();
            assert_eq!(rows[3], rows[4], "the axis writes the chart's own labels in the same cells at {width}");
        }
    }

    #[test]
    fn an_axis_is_text_in_every_glyph_mode() {
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            let mut h = Harness::new(Demo(Axis::hours(time(0, 0), time(0, 0))), 24, 1);
            h.set_glyph_mode(mode);
            assert_eq!(words(h.screen().trim_end()), ["00:00", "06:00", "12:00", "18:00"], "{mode:?}");
            assert_eq!(h.fg(0, 0), h.env().theme().color("dim"), "{mode:?}");
        }
    }

    #[test]
    fn nothing_to_label_measures_nothing() {
        let h = Harness::new(Demo(Axis::labels(Vec::<String>::new())), 10, 1);
        assert_eq!(h.screen(), "\n");
        let axis = Axis::labels(["a"]);
        assert_eq!(axis.count(), 1);
        assert_eq!(Axis::weekdays(Weekday::Monday, 0).count(), 0);
    }
}
