//! Sparklines: a series of numbers as a row of small columns.

use super::eighths;
use crate::color::Rgb;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::GlyphMode;
use crate::keymap::Key;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

/// How much a hovered tone is lifted towards the text colour, the same step the theme uses for
/// hovered surfaces.
const HOVER_LIFT: f32 = 0.08;

/// Builds a message from the point being read, or from no point at all.
type ReadMessage<Msg> = Box<dyn Fn(Option<usize>) -> Msg>;

/// A compact trend: one column per value, newest on the right, measured in eighths of a cell.
///
/// Columns scale from the lowest to the highest value shown, or over a fixed range. Every value
/// keeps at least one eighth so a quiet moment still reads as a sample. ASCII mode has no partial
/// blocks: whole cells fill with colour and the cell a column ends in takes the share of colour it
/// covers, so even the lowest sample tints one cell. When the series is wider than the area, the newest values are kept. Give the node a
/// height for taller columns.
///
/// With [`Sparkline::on_read`] a point can be read off the chart: pressing or dragging over the
/// columns reads the one under the pointer, and while focused ←/→ move one point, Home and End
/// jump to the oldest and the newest shown, and Esc stops reading. The message carries the
/// position in the values given, so the application writes the value itself; the application owns
/// which point is being read ([`Sparkline::reading`]). The read column rises to a tone band of its
/// own and is drawn in the accent, the hovered column's band is one hover step brighter than the
/// ground, and nothing moves: bands and columns stay in their cells.
///
/// Style keys: `sparkline` (`fg` for the columns, `peak` and `low` for the highlighted extremes,
/// `reading` for the column being read, `baseline` for the tone band of a reference value,
/// `track` for the ground in ASCII mode).
pub struct Sparkline<Msg> {
    values: Vec<f32>,
    range: Option<(f32, f32)>,
    extremes: bool,
    baseline: Option<f32>,
    reading: Option<usize>,
    on_read: Option<ReadMessage<Msg>>,
}

impl<Msg: 'static> Sparkline<Msg> {
    /// A sparkline of `values`, oldest first.
    #[must_use]
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        Self {
            values: values.into_iter().collect(),
            range: None,
            extremes: false,
            baseline: None,
            reading: None,
            on_read: None,
        }
    }

    /// Scales columns over `min..max` instead of the values shown, e.g. `0.0..100.0` for percent.
    #[must_use]
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.range = Some((min, max));
        self
    }

    /// Colours the highest and the lowest column shown.
    #[must_use]
    pub fn highlight_extremes(mut self) -> Self {
        self.extremes = true;
        self
    }

    /// Tints the cells at the level of `value`, such as a limit or an average, with a quiet band.
    #[must_use]
    pub fn baseline(mut self, value: f32) -> Self {
        self.baseline = Some(value);
        self
    }

    /// The point being read, as a position in the values given. A point that is not shown, because
    /// the area is narrower than the series, is not marked.
    #[must_use]
    pub fn reading(mut self, index: Option<usize>) -> Self {
        self.reading = index;
        self
    }

    /// Lets a point be read off the chart with the pointer and the keyboard; the message carries
    /// the position in the values given, or nothing when reading stops.
    #[must_use]
    pub fn on_read(mut self, message: impl Fn(Option<usize>) -> Msg + 'static) -> Self {
        self.on_read = Some(Box::new(message));
        self
    }

    fn scale(&self, shown: &[f32]) -> (f32, f32) {
        let (min, max) = self.range.unwrap_or_else(|| {
            let min = shown.iter().copied().fold(f32::INFINITY, f32::min);
            let max = shown.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            (min, max)
        });
        (min, if max > min { max } else { min + 1.0 })
    }

    /// Where the values shown in an area `width` cells wide start, and how many there are.
    fn window(&self, width: u16) -> (usize, usize) {
        let count = self.values.len().min(usize::from(width));
        (self.values.len() - count, count)
    }

    /// The point a pointer at column `x` reads: the nearest one, so a drag that leaves the
    /// columns keeps reading the end of the series.
    fn point_at(&self, area: Rect, x: i32) -> Option<usize> {
        let (start, count) = self.window(area.width);
        let last = count.checked_sub(1)?;
        let column = usize::try_from((x - area.x).max(0)).unwrap_or(0).min(last);
        Some(start + column)
    }

    /// Reads `target` when it is another point than the one being read.
    fn read(&self, cx: &mut EventCx<'_, Msg>, target: Option<usize>) {
        if target != self.reading
            && let Some(message) = &self.on_read
        {
            cx.emit(message(target));
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Sparkline<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let width = clamp_u16(i32::try_from(self.values.len()).unwrap_or(i32::MAX));
        Size::new(width, u16::from(width > 0)).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() || self.values.is_empty() {
            return;
        }
        let readable = self.on_read.is_some();
        if readable {
            cx.register_hit(area);
        }
        let (start, count) = self.window(area.width);
        let shown = &self.values[start..];
        let (min, max) = self.scale(shown);
        let style = cx.style("sparkline", None, &[]);
        let fill = style.color("fg").unwrap_or_else(|| cx.color("accent"));
        let peak = style.color("peak").unwrap_or(fill);
        let low = style.color("low").unwrap_or(fill);
        let marked = style.color("reading").unwrap_or_else(|| cx.color("accent"));
        let cells = area.height;
        let total = u32::from(cells) * 8;
        // The lowest value keeps one eighth and the highest fills the column.
        let level = |value: f32| 1 + eighths::scaled((value - min) / (max - min), total - 1);

        let ascii = cx.env().glyph_mode() == GlyphMode::Ascii;
        let track = style.color("track").unwrap_or_else(|| cx.color("raised"));
        if ascii {
            cx.clear(Rect::new(area.x, area.y, clamp_u16(i32::try_from(count).unwrap_or(0)), area.height), track);
        }
        // The row of the baseline band, counted from the bottom, and its colour.
        let band = self.baseline.map(|value| {
            let row = u16::try_from(level(value).saturating_sub(1) / 8).unwrap_or(0).min(cells - 1);
            (row, style.color("baseline").unwrap_or_else(|| cx.color("raised")))
        });
        if let Some((row, color)) = band {
            let width = clamp_u16(i32::try_from(count).unwrap_or(0));
            cx.fill(Rect::new(area.x, area.bottom() - 1 - i32::from(row), width, 1), color);
        }

        let hovered = if readable { cx.pointer().and_then(|(x, _)| self.point_at(area, x)) } else { None };
        let read = self.reading.filter(|index| (start..start + count).contains(index));
        let ground = (cx.color("raised"), cx.color("active"), cx.color("text"));
        let (peak_index, low_index) = if self.extremes { extremes(shown) } else { (None, None) };
        for (offset, value) in shown.iter().copied().enumerate() {
            let index = start + offset;
            let color = if Some(index) == read {
                marked
            } else if Some(offset) == peak_index {
                peak
            } else if Some(offset) == low_index {
                low
            } else {
                fill
            };
            let column = Rect::new(area.x + i32::try_from(offset).unwrap_or(0), area.y, 1, cells);
            let lit = column_band(ground, Some(index) == read, Some(index) == hovered);
            if let Some(tone) = lit {
                cx.fill(column, tone);
            }
            if ascii {
                let ground = lit.unwrap_or(track);
                paint_ascii_column(cx, column, level(value), color, |row| match band {
                    Some((band_row, band_color)) if band_row == row && lit.is_none() => band_color,
                    _ => ground,
                });
            } else {
                eighths::vertical(cx, column, level(value), color);
            }
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.on_read.is_none() || self.values.is_empty() {
            return false;
        }
        let area = cx.area();
        let (start, count) = self.window(area.width);
        let Some(last) = count.checked_sub(1).map(|last| start + last) else {
            return false;
        };
        match event {
            Event::Key(key) => {
                // A first key reads the newest point, which is the one a reader wants first.
                let current = self.reading.filter(|index| (start..=last).contains(index));
                let target = if key.is_plain(Key::Left) {
                    current.map_or(last, |index| index.saturating_sub(1).max(start))
                } else if key.is_plain(Key::Right) {
                    current.map_or(last, |index| (index + 1).min(last))
                } else if key.is_plain(Key::Home) {
                    start
                } else if key.is_plain(Key::End) {
                    last
                } else if key.is_plain(Key::Esc) {
                    if self.reading.is_none() {
                        return false;
                    }
                    self.read(cx, None);
                    return true;
                } else {
                    return false;
                };
                self.read(cx, Some(target));
                true
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseKind::Down(MouseButton::Left) => {
                    // The pointer is captured so a drag that leaves the columns keeps reading.
                    cx.capture_pointer();
                    self.read(cx, self.point_at(area, mouse.x));
                    true
                }
                MouseKind::Drag(MouseButton::Left) => {
                    self.read(cx, self.point_at(area, mouse.x));
                    true
                }
                MouseKind::Up(MouseButton::Left) => true,
                _ => false,
            },
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.on_read.is_some() && !self.values.is_empty()
    }
}

/// The tone behind a column: the read one sits on the active surface, a hovered one is lifted a
/// hover step above the ground it stands on, and a plain column has no band of its own.
/// `ground` is `(raised, active, text)`.
fn column_band(ground: (Rgb, Rgb, Rgb), read: bool, hovered: bool) -> Option<Rgb> {
    let (raised, active, text) = ground;
    match (read, hovered) {
        (false, false) => None,
        (true, false) => Some(active),
        (read, true) => Some(if read { active } else { raised }.mix(text, HOVER_LIFT)),
    }
}

/// Draws a column of `eighths` in ASCII mode, which has no partial blocks: whole cells take
/// `color`, and the cell the column ends in takes the share of `color` it covers over its ground
/// (`ground(row)`, rows counted from the bottom). Rounding to whole cells instead would hide the
/// lowest samples, or, one row tall, draw every sample as the same full cell; blending keeps every
/// sample visible and the trend readable, as cell-stepped colour does everywhere.
fn paint_ascii_column(cx: &mut PaintCx<'_>, column: Rect, eighths: u32, color: Rgb, ground: impl Fn(u16) -> Rgb) {
    let full = u16::try_from(eighths / 8).unwrap_or(u16::MAX).min(column.height);
    cx.fill(Rect::new(column.x, column.bottom() - i32::from(full), 1, full), color);
    let partial = eighths % 8;
    if partial > 0 && full < column.height {
        // `partial` is below eight, so the share is exact in f32.
        let tone = ground(full).mix(color, partial as f32 / 8.0);
        cx.fill(Rect::new(column.x, column.bottom() - i32::from(full) - 1, 1, 1), tone);
    }
}

/// The positions of the highest and the lowest value, or `None` for each that is not marked.
///
/// Only the latest occurrence of each extreme is marked, so a flat top is not a stripe, and a flat
/// series has a peak but no low.
fn extremes(shown: &[f32]) -> (Option<usize>, Option<usize>) {
    let highest = shown.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let lowest = shown.iter().copied().fold(f32::INFINITY, f32::min);
    let peak = shown.iter().rposition(|value| *value >= highest);
    let low = shown.iter().rposition(|value| *value <= lowest).filter(|_| highest > lowest);
    (peak, low)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    /// A sparkline of `values`, with the capabilities each test turns on.
    struct Demo {
        values: Vec<f32>,
        rows: u16,
        range: Option<(f32, f32)>,
        extremes: bool,
        baseline: Option<f32>,
        reading: Option<usize>,
        readable: bool,
    }

    impl Demo {
        fn new(values: impl IntoIterator<Item = f32>, rows: u16) -> Self {
            Self {
                values: values.into_iter().collect(),
                rows,
                range: None,
                extremes: false,
                baseline: None,
                reading: None,
                readable: false,
            }
        }

        fn range(mut self, min: f32, max: f32) -> Self {
            self.range = Some((min, max));
            self
        }

        fn extremes(mut self) -> Self {
            self.extremes = true;
            self
        }

        fn baseline(mut self, value: f32) -> Self {
            self.baseline = Some(value);
            self
        }

        fn readable(mut self) -> Self {
            self.readable = true;
            self
        }
    }

    impl App for Demo {
        type Msg = Option<usize>;
        fn update(&mut self, reading: Option<usize>) -> Command<Option<usize>> {
            self.reading = reading;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Option<usize>>) {
            let mut spark = Sparkline::new(self.values.iter().copied());
            if let Some((min, max)) = self.range {
                spark = spark.range(min, max);
            }
            if self.extremes {
                spark = spark.highlight_extremes();
            }
            if let Some(value) = self.baseline {
                spark = spark.baseline(value);
            }
            if self.readable {
                spark = spark.reading(self.reading).on_read(|index| index);
            }
            ui.add(spark).height(Length::Cells(self.rows)).id("trend");
        }
    }

    fn harness(demo: Demo, width: u16) -> Harness<Demo> {
        let height = demo.rows;
        Harness::new(demo, width, height)
    }

    /// The column colour of the built-in theme.
    fn column(h: &Harness<Demo>) -> Rgb {
        let theme = h.env().theme();
        theme.color("surface").expect("token").mix(theme.color("accent").expect("token"), 0.72)
    }

    const LOAD: [f32; 8] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];

    #[test]
    fn one_row_uses_eighth_blocks_and_keeps_newest() {
        let h = harness(Demo::new(LOAD, 1), 8);
        assert_eq!(h.screen(), "▁▂▃▄▅▆▇\n");
        assert_eq!(h.bg(7, 0), Some(column(&h)));
        let narrow = harness(Demo::new(LOAD, 1), 4);
        assert_eq!(narrow.screen(), "▁▃▆\n");
    }

    #[test]
    fn taller_columns_and_fixed_range() {
        let h = harness(Demo::new([50.0, 100.0, 0.0], 2).range(0.0, 100.0), 3);
        assert_eq!(h.screen(), "▁\n  ▁\n");
        assert_eq!(h.bg(0, 1), Some(column(&h)));
        assert_eq!(h.bg(1, 0), Some(column(&h)));
    }

    #[test]
    fn extremes_and_baseline_are_coloured() {
        let h = harness(Demo::new([3.0, 9.0, 1.0, 5.0], 1).extremes(), 4);
        assert_eq!(h.screen(), "▃ ▁▅\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(1, 0), theme.color("accent"), "the peak is the brightest column");
        assert_eq!(h.fg(2, 0), theme.color("muted"), "the low column is muted");
        assert_ne!(h.fg(0, 0), h.fg(2, 0));
        let lined = harness(Demo::new([3.0, 9.0, 1.0, 5.0], 1).extremes().baseline(5.0), 4);
        assert_ne!(lined.bg(0, 0), h.bg(0, 0), "the baseline row is tinted");
        assert_eq!(lined.bg(1, 0), theme.color("accent"), "columns are drawn over the band");
    }

    #[test]
    fn extremes_mark_the_latest_highest_and_lowest() {
        assert_eq!(extremes(&[3.0, 9.0, 1.0, 5.0]), (Some(1), Some(2)));
        assert_eq!(extremes(&[9.0, 1.0, 9.0, 1.0, 4.0]), (Some(2), Some(3)), "latest occurrence of each");
        assert_eq!(extremes(&[2.0, 2.0, 2.0]), (Some(2), None), "a flat series has no low");
        assert_eq!(extremes(&[]), (None, None));
    }

    #[test]
    fn extremes_take_linear_time_on_the_widest_series() {
        // A falling series is the worst case for a quadratic scan: about two billion comparisons
        // at this width, which takes seconds in a debug build.
        let falling: Vec<f32> = (0..u16::MAX).rev().map(f32::from).collect();
        let started = std::time::Instant::now();
        assert_eq!(extremes(&falling), (Some(0), Some(falling.len() - 1)));
        assert!(started.elapsed() < std::time::Duration::from_millis(500), "took {:?}", started.elapsed());
    }

    #[test]
    fn ascii_fills_cells_on_a_track() {
        let mut h = harness(Demo::new([0.0, 10.0], 2), 2);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "\n\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(0, 0), theme.color("raised"));
        let lowest = h.bg(0, 1);
        assert!(lowest.is_some() && lowest != theme.color("raised"), "the lowest value still tints one cell");
        assert_eq!(h.bg(1, 0), Some(column(&h)));
    }

    #[test]
    fn ascii_shows_every_sample_as_at_least_one_cell() {
        let mut h = harness(Demo::new([0.0, 1.0, 50.0, 100.0], 3), 4);
        h.set_glyph_mode(GlyphMode::Ascii);
        let raised = h.env().theme().color("raised");
        for x in 0..4 {
            assert_ne!(h.bg(x, 2), raised, "sample {x} is visible");
        }
        assert_eq!(h.bg(0, 1), raised, "low samples do not grow past one cell");
        assert_eq!(h.bg(3, 0), Some(column(&h)), "the highest fills the column");
        assert_eq!(h.bg(2, 2), Some(column(&h)), "cells a column passes are whole colour");
    }

    #[test]
    fn one_row_ascii_keeps_the_trend_in_tone() {
        let mut h = harness(Demo::new(LOAD, 1), 8);
        h.set_glyph_mode(GlyphMode::Ascii);
        let brightness = |x: u16| h.bg(x, 0).map_or(0, |c| u32::from(c.r) + u32::from(c.g) + u32::from(c.b));
        let raised = h.env().theme().color("raised");
        assert!((0..8).all(|x| h.bg(x, 0) != raised), "every sample tints its cell");
        assert!((1..8).all(|x| brightness(x) != brightness(x - 1)), "rising values read as rising tones");
        assert_eq!(h.bg(7, 0), Some(column(&h)), "the highest is whole colour");
    }

    /// Four samples on a two-row sparkline: 20, 90, 40 and 60 percent.
    fn readable() -> Harness<Demo> {
        harness(Demo::new([20.0, 90.0, 40.0, 60.0], 2).range(0.0, 100.0).readable(), 4)
    }

    #[test]
    fn a_plain_sparkline_neither_takes_focus_nor_sends_messages() {
        let mut h = harness(Demo::new([20.0, 90.0, 40.0, 60.0], 2).range(0.0, 100.0), 4);
        let before = h.screen();
        h.press("tab").press("right").press("end").click(1, 1).hover(2, 0);
        assert_eq!(h.app().reading, None, "nothing was read");
        assert_eq!(h.screen(), before, "and nothing changed on screen");
    }

    #[test]
    fn a_press_reads_the_point_under_the_pointer_and_a_drag_scrubs() {
        let mut h = readable();
        h.click(2, 1);
        assert_eq!(h.app().reading, Some(2), "the third column was read");
        h.drag((2, 1), (0, 1));
        assert_eq!(h.app().reading, Some(0), "the drag read the oldest point");
        h.drag((0, 1), (40, 1));
        assert_eq!(h.app().reading, Some(3), "a drag past the columns keeps the newest point");
    }

    #[test]
    fn keys_move_the_reading_and_esc_stops_it() {
        let mut h = readable();
        h.press("tab");
        assert_eq!(h.app().reading, None, "focus alone reads nothing");
        h.press("left");
        assert_eq!(h.app().reading, Some(3), "the first key reads the newest point");
        h.press("left");
        assert_eq!(h.app().reading, Some(2));
        h.press("home");
        assert_eq!(h.app().reading, Some(0));
        h.press("left");
        assert_eq!(h.app().reading, Some(0), "the oldest shown point is the end of the way");
        h.press("right");
        assert_eq!(h.app().reading, Some(1));
        h.press("end");
        assert_eq!(h.app().reading, Some(3));
        h.press("right");
        assert_eq!(h.app().reading, Some(3), "and so is the newest");
        h.press("esc");
        assert_eq!(h.app().reading, None, "esc stops reading");
    }

    #[test]
    fn the_read_column_rises_and_the_hovered_one_lightens_without_moving() {
        let mut h = readable();
        let quiet = h.screen();
        let ground = h.bg(1, 0);
        h.hover(1, 0);
        let hovered = h.bg(1, 0);
        assert_ne!(hovered, ground, "the hovered column stands on a lighter ground");
        assert_eq!(h.screen(), quiet, "hovering moves no cell");
        assert_eq!(h.bg(2, 0), ground, "only the hovered column changes");

        h.send(Some(1));
        let (accent, active) = {
            let theme = h.env().theme();
            (theme.color("accent"), theme.color("active"))
        };
        assert_eq!(h.bg(1, 1), accent, "the read column is drawn in the accent");
        assert_ne!(h.bg(1, 0), hovered, "and its band is the active surface, lifted while hovered");
        assert_eq!(h.screen(), quiet, "reading moves no cell either");
        h.hover(30, 30);
        assert_eq!(h.bg(1, 0), active, "unhovered, the read column keeps the active band");
    }

    #[test]
    fn a_reading_outside_the_shown_window_is_not_marked() {
        let mut h = harness(Demo::new(LOAD, 1).readable(), 3);
        h.send(Some(0));
        let theme = h.env().theme();
        assert_eq!(h.app().reading, Some(0));
        assert!((0..3).all(|x| h.bg(x, 0) != theme.color("active")), "no column is marked:\n{}", h.screen());
        // Only the three newest values are shown, so the keys work inside that window.
        h.press("tab").press("home");
        assert_eq!(h.app().reading, Some(5), "home reads the oldest point shown");
    }

    #[test]
    fn reading_works_in_every_glyph_mode_and_with_one_value() {
        for mode in [GlyphMode::Unicode, GlyphMode::Nerd, GlyphMode::Ascii] {
            let mut h = readable();
            h.set_glyph_mode(mode);
            let before = h.screen();
            h.click(3, 1);
            assert_eq!(h.app().reading, Some(3), "{mode:?}");
            assert_eq!(h.screen(), before, "{mode:?} moves no cell");
            assert_eq!(h.bg(3, 1), h.env().theme().color("accent"), "{mode:?} marks the read column");

            let mut single = harness(Demo::new([42.0], 1).readable(), 6);
            single.set_glyph_mode(mode);
            single.press("tab").press("right");
            assert_eq!(single.app().reading, Some(0), "{mode:?} reads the only value");
        }
    }

    #[test]
    fn reading_holds_in_every_theme() {
        for theme in ["monochrome", "nordic", "amber", "iris"] {
            let mut h = readable();
            h.set_theme(theme);
            h.click(3, 1);
            assert_eq!(h.app().reading, Some(3), "{theme}");
            let accent = h.env().theme().color("accent");
            assert_eq!(h.bg(3, 1), accent, "{theme} marks the read column in its accent");
            assert_ne!(h.bg(3, 0), h.bg(0, 0), "{theme} raises the read column's band above the ground");
        }
    }

    #[test]
    fn an_empty_readable_sparkline_stays_quiet() {
        let mut h = harness(Demo::new([], 1).readable(), 6);
        assert_eq!(h.screen(), "\n");
        h.press("tab").press("right").press("home").click(0, 0);
        assert_eq!(h.app().reading, None, "there is nothing to read");
        assert_eq!(h.screen(), "\n");
    }

    #[test]
    fn equal_values_are_all_readable() {
        let mut h = harness(Demo::new([50.0, 50.0, 50.0], 2).range(0.0, 100.0).readable(), 3);
        assert_eq!(h.screen(), "▁▁▁\n\n", "equal values stand at one flat level");
        h.click(1, 1);
        assert_eq!(h.app().reading, Some(1));
        assert_eq!(h.bg(1, 1), h.env().theme().color("accent"));
        assert_eq!(h.bg(0, 1), Some(column(&h)), "its neighbours keep the column colour");
    }

    #[test]
    fn every_builtin_theme_gives_the_read_column_its_own_tone() {
        let registry = crate::theme::ThemeRegistry::builtin();
        for (id, _) in registry.list() {
            let theme = registry.resolve(&id).theme.expect("resolves");
            let style = theme.style("sparkline", None, &[]);
            let tone = |key: &str| style.paint(key).map(|paint| paint.at(0.0));
            let (Some(reading), Some(line)) = (tone("reading"), tone("fg")) else {
                panic!("theme {id} names no `reading` or `fg` tone");
            };
            // The distance a theme keeps between colours of different meaning.
            let distance = reading.perceptual_distance(line);
            assert!(distance >= 0.10, "theme {id}: the read column {reading} is too close to the line {line}");
            assert_eq!(Some(reading), theme.color("accent"), "theme {id} reads in its accent, as it selects");
        }
    }

    #[test]
    fn reading_survives_a_one_cell_area() {
        let mut h = harness(Demo::new(LOAD, 1).readable(), 1);
        h.press("tab").press("left");
        assert_eq!(h.app().reading, Some(7), "the only column shown is the newest value");
        h.press("left");
        assert_eq!(h.app().reading, Some(7));
        h.click(0, 0);
        assert_eq!(h.app().reading, Some(7));
    }
}
