//! Level bars: a row of vertical columns for the levels of a spectrum or of several meters.

use super::eighths;
use crate::color::{ColorDepth, Rgb};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::GlyphMode;
use crate::style::CellStyle;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Columns for a spectrum or for a set of meters, one per value, measured in eighths of a cell.
///
/// Every value is a level between zero and one and the application computes it: the widget draws
/// columns and nothing else, so a value can come from a frequency band, a channel of a sound, a
/// queue depth or anything else, and it is the application that moves it from frame to frame.
/// Values outside the range are clamped and a value that is not a number stands for silence, so a
/// band that has not been measured yet draws nothing rather than a full column.
///
/// A column rises from the bottom of the area in the eighth blocks `▁▂▃▄▅▆▇█`, so three rows
/// give twenty-four levels. [`mirror`](Self::mirror) grows the columns up and down from the middle
/// row instead, which is how a level meter reads in one row's space, and
/// [`gradient`](Self::gradient) blends the colour from the theme's base tone at the bottom of a
/// column to the accent at its top, one step per cell.
///
/// [`peaks`](Self::peaks) puts a thin cap on each column at the level given, the way a meter holds
/// its high mark: the block that reaches it, or the thin `▔` line where the level is the top of a
/// cell, the one place a cap can be a line instead of a block. A cap at or below the level of the
/// column it belongs to is not drawn, and neither is one with no cell of its own.
///
/// Columns are [`bar_width`](Self::bar_width) cells wide with [`gap`](Self::gap) cells between
/// them. Values that do not fit are merged by averaging their neighbours, so a narrow area shows
/// the whole spectrum and a wide one shows every band; room to spare instead widens the columns,
/// the cells that cannot be shared out going to the outermost ones so the two ends of the row
/// match. A value list that is empty draws nothing, which leaves the caller room for an empty
/// state beside it.
///
/// In ASCII mode, which has no partial blocks, whole cells take the colour of the level and the
/// cell a column ends in takes the share of it it covers, so even a quiet band tints a cell. In
/// a terminal with the sixteen standard colours the gradient is left out and every column is one
/// tone, because a blend there would round to a palette entry per cell and speckle.
///
/// Style keys: `level-bars` (`base` for the columns and the bottom of the gradient, `peak` for a
/// cap and the top of the gradient, `track` for the ground the columns stand on in ASCII mode).
pub struct LevelBars {
    values: Vec<f32>,
    peaks: Option<Vec<f32>>,
    gap: u16,
    bar_width: u16,
    mirror: bool,
    gradient: bool,
}

impl LevelBars {
    /// Columns for `values`, each a level between zero and one, the lowest band first.
    #[must_use]
    pub fn new(values: impl Into<Vec<f32>>) -> Self {
        Self { values: values.into(), peaks: None, gap: 1, bar_width: 1, mirror: false, gradient: false }
    }

    /// Empty cells between the columns; one by default, so neighbouring columns never merge into
    /// one block. A gap as wide as the whole area leaves one column, which is drawn as wide as the
    /// area.
    #[must_use]
    pub fn gap(mut self, cells: u16) -> Self {
        self.gap = cells;
        self
    }

    /// Cells a column is drawn in; one by default, a thin bar beside its neighbours. Columns grow
    /// past it when the values leave room, and are cut to the area when it is narrower than one.
    #[must_use]
    pub fn bar_width(mut self, cells: u16) -> Self {
        self.bar_width = cells;
        self
    }

    /// A cap on each column at `peaks`, one value per value given, such as the highest a band
    /// reached since it was last read. A peak below the column it belongs to draws no cap, and a
    /// column with no room above it has none either; merged columns take the mean of the caps they
    /// cover. A list shorter than the values caps the columns it has.
    #[must_use]
    pub fn peaks(mut self, peaks: impl Into<Vec<f32>>) -> Self {
        self.peaks = Some(peaks.into());
        self
    }

    /// Grows the columns up and down from the middle row instead of only up from the bottom, so a
    /// level reads as loud on both sides of the middle. A level that cannot be shared evenly
    /// grows upwards first, and a mirrored column needs two rows: in a one-row area both halves
    /// would be thinner than a cell, so it draws as a plain column.
    #[must_use]
    pub fn mirror(mut self, on: bool) -> Self {
        self.mirror = on;
        self
    }

    /// Blends the colour of a column from the theme's base tone where it starts to the accent
    /// where it ends, one step per cell, so a column reads as deep or loud. Both ends are theme
    /// tones, so the blend changes with the theme, and a terminal with the sixteen standard colours
    /// keeps one tone instead, which every depth can draw.
    #[must_use]
    pub fn gradient(mut self, on: bool) -> Self {
        self.gradient = on;
        self
    }

    /// The columns drawn in an area `width` cells wide.
    ///
    /// A column is [`bar_width`](Self::bar_width) cells with [`gap`](Self::gap) between them, the
    /// last one carrying no trailing gap; values that would not fit are merged into the columns
    /// that do, and the cells left over widen every column by the same share, the ones that cannot
    /// be shared out going to the outermost columns.
    fn slots(&self, width: u16) -> Vec<Slot> {
        let count = self.values.len();
        let room = usize::from(width);
        if count == 0 || room == 0 {
            return Vec::new();
        }
        // A column wider than the area would be cut by the frame, so it takes the whole area
        // instead: the bar is as wide as there is room for, never wider.
        let bar = usize::from(self.bar_width.max(1)).min(room);
        let gap = usize::from(self.gap);
        let columns = count.min((room + gap) / (bar + gap));
        let spare = room - columns * bar - (columns - 1) * gap;
        let cells = clamp_u16(i32::try_from(bar + spare / columns).unwrap_or(i32::MAX));
        let outer = spare % columns;
        let mut slots = Vec::with_capacity(columns);
        let mut x = 0u16;
        for index in 0..columns {
            let taken = cells + u16::from(turn(index, columns) < outer);
            slots.push(Slot { x, cells: taken });
            x = x.saturating_add(taken).saturating_add(self.gap);
        }
        slots
    }

    /// The level a column of `columns` stands for and the level of its cap, one pair for each
    /// column. Values that do not fit are averaged into the columns that do, so a narrow area
    /// shows the whole spectrum and a wide one every band of it, and a merged column's cap is the
    /// mean of the caps it covers. Every value is read as a level first, so one band out of range
    /// or not a number cannot pull a merged column past the area.
    fn levels(&self, columns: usize) -> Vec<(f32, Option<f32>)> {
        let count = self.values.len();
        let peaks = self.peaks.as_deref().unwrap_or_default();
        (0..columns)
            .map(|column| {
                let start = column * count / columns;
                let end = (column + 1) * count / columns;
                let capped = &peaks[start.min(peaks.len())..end.min(peaks.len())];
                (mean(&self.values[start..end]), (!capped.is_empty()).then(|| mean(capped)))
            })
            .collect()
    }

    /// The two halves a column in `area` is drawn in: one for a plain column, which grows up from
    /// the bottom of the area, and two for a mirrored one, which grow up and down from the middle
    /// row. A mirrored column needs two rows, so in a one-row area it draws as a plain column.
    fn halves(&self, area: Rect) -> (Half, Option<Half>) {
        let plain = (Half { area, up: true }, None);
        if !self.mirror || area.height < 2 {
            return plain;
        }
        let down = area.height / 2;
        let axis = i32::from(area.height - down);
        (
            Half { area: Rect::new(area.x, area.y, area.width, area.height - down), up: true },
            Some(Half { area: Rect::new(area.x, area.y + axis, area.width, down), up: false }),
        )
    }
}

/// Where one column sits in the area and how wide it is.
struct Slot {
    /// Columns from the left of the area.
    x: u16,
    /// Cells the column takes.
    cells: u16,
}

/// One half of a column: the cells it is drawn in, and the end it grows from.
struct Half {
    /// The cells available.
    area: Rect,
    /// Whether the half grows up from the bottom of `area`; otherwise it hangs from the top.
    up: bool,
}

/// The tones the columns are drawn in, and what the terminal can show them in.
struct Inks {
    /// The tone where a column starts.
    base: Rgb,
    /// The tone where it ends, and the tone of a cap.
    peak: Rgb,
    /// The ground the columns stand on in ASCII mode.
    track: Rgb,
    /// Whether a column blends from the base tone to the peak one; sixteen colours cannot.
    blend: bool,
    /// Whether the terminal has no block glyphs to measure a level with.
    ascii: bool,
}

/// How far into the cells left over a column of `columns` stands. They are shared out from the
/// left edge and the right edge in turn, so the two ends of a row match as far as an odd remainder
/// allows.
fn turn(index: usize, columns: usize) -> usize {
    let from_right = columns - 1 - index;
    if index <= from_right { index * 2 } else { from_right * 2 + 1 }
}

/// A value as a level of a column: outside zero to one clamped, and a value that is not a number
/// stands for silence rather than for a full column.
fn level(value: f32) -> f32 {
    if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) }
}

/// The mean of the levels of `values`, which is never empty: a column always covers at least one
/// value.
fn mean(values: &[f32]) -> f32 {
    let total: f32 = values.iter().copied().map(level).sum();
    total / values.len() as f32
}

impl Half {
    /// The row of the cell `index` cells from the end the half grows from, or nothing when the
    /// half is not that tall.
    fn cell(&self, index: u32) -> Option<i32> {
        let row = if self.up {
            self.area.bottom() - 1 - i32::try_from(index).unwrap_or(i32::MAX)
        } else {
            self.area.y.saturating_add(i32::try_from(index).unwrap_or(i32::MAX))
        };
        (self.area.y..self.area.bottom()).contains(&row).then_some(row)
    }

    /// The colour of the cell at `row`: the base tone, or a step of the blend from the tone at the
    /// end the half grows from to the tone at its other end, so a mirrored column deepens away
    /// from the middle on both sides.
    fn tone(&self, row: i32, inks: &Inks) -> Rgb {
        if !inks.blend || self.area.height < 2 {
            return inks.base;
        }
        let from_base = if self.up { self.area.bottom() - 1 - row } else { row - self.area.y };
        let steps = self.area.height - 1;
        let step = u16::try_from(from_base).unwrap_or(steps).min(steps);
        inks.base.mix(inks.peak, f32::from(step) / f32::from(steps))
    }

    /// Writes `block` across the cell at `row` in `tone`.
    fn block(&self, cx: &mut PaintCx<'_>, row: i32, block: &str, tone: Rgb) {
        for x in self.area.x..self.area.right() {
            cx.text(x, row, block, CellStyle::fg(tone), 1);
        }
    }

    /// Draws `level` eighths of a cell into the half: whole cells as the full block, and the block
    /// that ends the level in the cell above them.
    ///
    /// ASCII mode has no blocks at all: whole cells take the tone of the level and the cell it ends
    /// in takes the share of it it covers over the ground, as a sparkline reads in that mode.
    fn paint(&self, cx: &mut PaintCx<'_>, level: u32, inks: &Inks) {
        let whole = level / 8;
        for index in 0..whole {
            let Some(row) = self.cell(index) else { break };
            let tone = self.tone(row, inks);
            if inks.ascii {
                cx.fill(Rect::new(self.area.x, row, self.area.width, 1), tone);
            } else {
                self.block(cx, row, eighths::FULL, tone);
            }
        }
        let partial = level % 8;
        if partial == 0 {
            return;
        }
        let Some(row) = self.cell(whole) else { return };
        let tone = self.tone(row, inks);
        if inks.ascii {
            // `partial` is below eight, so the share is exact in f32.
            let cell = Rect::new(self.area.x, row, self.area.width, 1);
            cx.fill(cell, inks.track.mix(tone, partial as f32 / 8.0));
            return;
        }
        let partial = usize::try_from(partial).unwrap_or(0);
        let block = if self.up { eighths::lower_block(partial) } else { eighths::upper_block(partial) };
        self.block(cx, row, block, tone);
    }

    /// Draws the cap of a half at `level`, the high mark the level reached before it came back
    /// down: the block that reaches it, or the thin `▔` line where the level is the top of a cell,
    /// which is the one place a cap can be a line without covering a cell the column already fills.
    /// A level with no cell of its own has no cap.
    ///
    /// ASCII mode has no partial blocks, so a cap is a whole cell of the peak tone in the cell the
    /// level falls in.
    fn paint_cap(&self, cx: &mut PaintCx<'_>, level: u32, inks: &Inks) {
        let Some(row) = self.cell(level / 8) else { return };
        if inks.ascii {
            cx.fill(Rect::new(self.area.x, row, self.area.width, 1), inks.peak);
            return;
        }
        let partial = usize::try_from(level % 8).unwrap_or(0);
        let block = match (partial, self.up) {
            (0, true) => eighths::CAP,
            (0, false) => eighths::FLOOR,
            (partial, true) => eighths::lower_block(partial),
            (partial, false) => eighths::upper_block(partial),
        };
        self.block(cx, row, block, inks.peak);
    }
}

impl<Msg: 'static> Widget<Msg> for LevelBars {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        // The columns at the width asked for: a wider column is something the area has room for,
        // which a measure cannot know, and one row is enough for a bar.
        let count = self.values.len();
        let cells = count
            .saturating_mul(usize::from(self.bar_width.max(1)))
            .saturating_add(count.saturating_sub(1).saturating_mul(usize::from(self.gap)));
        Size::new(clamp_u16(i32::try_from(cells).unwrap_or(i32::MAX)), 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() || self.values.is_empty() {
            return;
        }
        let slots = self.slots(area.width);
        if slots.is_empty() {
            return;
        }
        let style = cx.style("level-bars", None, &[]);
        let accent = cx.color("accent");
        let inks = Inks {
            // What the theme writes as `mix($accent, $surface, 60%)`: a share of the first colour
            // blended into the second.
            base: style.color("base").unwrap_or_else(|| cx.color("surface").mix(accent, 0.6)),
            peak: style.color("peak").unwrap_or(accent),
            track: style.color("track").unwrap_or_else(|| cx.color("raised")),
            // Sixteen colours cannot hold a blend: every cell would round to its own palette entry
            // and a column would speckle instead of shading, so there the whole column is one tone.
            blend: self.gradient && cx.env().depth() != ColorDepth::Ansi16,
            ascii: cx.env().glyph_mode() == GlyphMode::Ascii,
        };
        if inks.ascii {
            cx.clear(area, inks.track);
        }
        for (slot, (value, cap)) in slots.iter().zip(self.levels(slots.len())) {
            let column = Rect::new(area.x + i32::from(slot.x), area.y, slot.cells, area.height);
            let (up, down) = self.halves(column);
            let level = eighths::eighths(value, column.height);
            let cap = cap.map_or(0, |cap| eighths::eighths(cap, column.height));
            // A level that cannot be shared evenly between two halves grows upwards first, so a
            // mirrored column reaches the level a plain one would.
            let (up_level, down_level) = match down {
                Some(_) => (level.div_ceil(2), level / 2),
                None => (level, 0),
            };
            let (up_cap, down_cap) = match down {
                Some(_) => (cap.div_ceil(2), cap / 2),
                None => (cap, 0),
            };
            for (half, level, cap) in [(Some(up), up_level, up_cap), (down, down_level, down_cap)] {
                let Some(half) = half else { continue };
                half.paint(cx, level, &inks);
                if cap > level {
                    half.paint_cap(cx, cap, &inks);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    /// A row of level bars with the capabilities each test turns on.
    struct Demo {
        values: Vec<f32>,
        peaks: Option<Vec<f32>>,
        rows: u16,
        gap: u16,
        bar_width: u16,
        mirror: bool,
        gradient: bool,
    }

    impl Demo {
        fn new(values: impl Into<Vec<f32>>, rows: u16) -> Self {
            Self { values: values.into(), peaks: None, rows, gap: 1, bar_width: 1, mirror: false, gradient: false }
        }

        fn peaks(mut self, peaks: impl Into<Vec<f32>>) -> Self {
            self.peaks = Some(peaks.into());
            self
        }

        fn gap(mut self, cells: u16) -> Self {
            self.gap = cells;
            self
        }

        fn bar_width(mut self, cells: u16) -> Self {
            self.bar_width = cells;
            self
        }

        fn mirrored(mut self) -> Self {
            self.mirror = true;
            self
        }

        fn gradient(mut self) -> Self {
            self.gradient = true;
            self
        }
    }

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            let mut bars = LevelBars::new(self.values.clone())
                .gap(self.gap)
                .bar_width(self.bar_width)
                .mirror(self.mirror)
                .gradient(self.gradient);
            if let Some(peaks) = &self.peaks {
                bars = bars.peaks(peaks.clone());
            }
            ui.add(bars).height(Length::Cells(self.rows)).fill_width();
        }
    }

    /// A row `width` cells wide of the demo's bars.
    fn harness(demo: Demo, width: u16) -> Harness<Demo> {
        let height = demo.rows;
        Harness::new(demo, width, height)
    }

    /// The two tones a column is drawn in, as the built-in theme gives them: the theme writes the
    /// base as `mix($accent, $surface, 60%)`, a share of the first colour blended into the second.
    fn tones(h: &Harness<Demo>) -> (Rgb, Rgb) {
        let theme = h.env().theme();
        let accent = theme.color("accent").expect("token");
        (theme.color("surface").expect("token").mix(accent, 0.6), accent)
    }

    #[test]
    fn a_level_is_measured_in_eighths_of_a_cell() {
        let h = harness(Demo::new([0.5], 4), 1);
        assert_eq!(h.screen(), "\n\n█\n█\n", "half a level in four rows is exactly two whole cells");
        let eighths = harness(Demo::new([0.3], 1), 1);
        assert_eq!(eighths.screen(), "▂\n", "three tenths of a cell is two eighths");
        assert_eq!(eighths.fg(0, 0), Some(tones(&eighths).0), "the block carries the column's tone");
        let whole = harness(Demo::new([1.0], 3), 1);
        assert_eq!(whole.screen(), "█\n█\n█\n", "a full level fills every row of the area");
    }

    #[test]
    fn levels_outside_the_range_and_nonsense_are_read_as_silence() {
        let h = harness(Demo::new([-0.5, f32::NAN, 1.4, 0.5], 2), 7);
        assert_eq!(h.screen(), "    █\n    █ █\n", "below zero and not a number are silence, over one is a full level");
        assert_eq!(h.fg(4, 1), Some(tones(&h).0));
    }

    #[test]
    fn gaps_and_widths_lay_the_columns_out() {
        let gapped = harness(Demo::new([1.0, 1.0, 1.0], 1).gap(3), 9);
        assert_eq!(gapped.screen(), "█   █   █\n", "three empty cells between the columns");
        let wide = harness(Demo::new([1.0, 1.0], 1).bar_width(3), 8);
        assert_eq!(wide.screen(), "████ ███\n", "the cell that is left over goes to the outermost column");
        let narrow = harness(Demo::new([1.0, 0.0], 2), 1);
        assert_eq!(narrow.screen(), "\n█\n", "a column with no room for a gap stands for both values");
    }

    #[test]
    fn a_merged_column_stands_for_the_average_of_its_neighbours() {
        let hundred: Vec<f32> = (0..100u16).map(|value| f32::from(value) / 100.0).collect();
        let h = harness(Demo::new(hundred, 4), 19);
        // Ten columns of ten values each, every column a tenth louder than the one before it.
        assert_eq!(h.screen(), "                ▃ ▆\n          ▁ ▅ █ █ █\n      ▃ ▆ █ █ █ █ █\n▁ ▅ █ █ █ █ █ █ █ █\n");
    }

    #[test]
    fn a_mirrored_column_grows_from_the_middle_row() {
        let h = harness(Demo::new([0.5], 4).mirrored(), 1);
        assert_eq!(h.screen(), "\n█\n█\n\n", "half up and half down, one cell on each side of the middle");
        let eighth = harness(Demo::new([0.25], 4).mirrored(), 1);
        assert_eq!(eighth.screen(), "\n▄\n▄\n\n", "a quarter of the level is a quarter cell on each side");
        let one_row = harness(Demo::new([1.0], 1).mirrored(), 1);
        assert_eq!(one_row.screen(), "█\n", "a mirrored column needs two rows, so it draws as a plain one");
    }

    #[test]
    fn a_peak_cap_sits_at_the_level_the_column_reached() {
        let h = harness(Demo::new([0.25], 4).peaks([0.375]), 1);
        assert_eq!(h.screen(), "\n\n▄\n█\n", "the cap is the block that reaches the peak level");
        let whole = harness(Demo::new([0.25], 4).peaks([0.5]), 1);
        assert_eq!(whole.screen(), "\n▔\n\n█\n", "a peak on the top of a cell is the thin line above it");
        let quiet = harness(Demo::new([0.75], 4).peaks([0.5]), 1);
        assert_eq!(quiet.screen(), "\n█\n█\n█\n", "a peak below the column draws no cap");
        let full = harness(Demo::new([0.5], 2).peaks([1.0]), 1);
        assert_eq!(full.screen(), "\n█\n", "a cap with no cell of its own is left out");
        assert_eq!(h.fg(0, 2), Some(tones(&h).1), "the cap is the theme's peak tone");
    }

    #[test]
    fn a_mirrored_column_carries_a_cap_on_each_of_its_two_halves() {
        let h = harness(Demo::new([0.25], 6).peaks([0.5]).mirrored(), 1);
        assert_eq!(h.screen(), "\n▄\n▆\n▂\n▄\n\n", "the same peak level on both sides of the middle");
    }

    #[test]
    fn a_merged_column_takes_the_mean_of_the_caps_it_covers() {
        let h = harness(Demo::new([0.0, 0.0], 4).peaks([0.5, 1.0]), 1);
        assert_eq!(h.screen(), "▔\n\n\n\n", "one cap at the mean of the two peaks, in a column of its own");
    }

    #[test]
    fn a_gradient_deepens_a_column_from_its_base_to_its_top() {
        let h = harness(Demo::new([1.0], 4).gradient(), 1);
        let (base, peak) = tones(&h);
        assert_eq!(h.fg(0, 3), Some(base), "the column starts at the base tone");
        assert_eq!(h.fg(0, 0), Some(peak), "and ends at the accent");
        assert_ne!(h.fg(0, 1), h.fg(0, 2), "one step per cell");
        let flat = harness(Demo::new([1.0], 4), 1);
        assert_eq!(flat.fg(0, 0), Some(base), "without the gradient a column is one tone");
        assert_eq!(flat.fg(0, 3), Some(base));
    }

    #[test]
    fn a_mirrored_column_deepens_away_from_the_middle_on_both_sides() {
        let h = harness(Demo::new([1.0], 4).gradient().mirrored(), 1);
        let (base, peak) = tones(&h);
        assert_eq!(h.fg(0, 1), Some(base), "the row above the middle starts at the base tone");
        assert_eq!(h.fg(0, 2), Some(base), "and the row below it");
        assert_eq!(h.fg(0, 0), Some(peak), "the top of the column is the accent");
        assert_eq!(h.fg(0, 3), Some(peak), "and so is the bottom of a mirrored one");
    }

    #[test]
    fn sixteen_colours_keep_one_tone_for_the_whole_column() {
        let mut h = harness(Demo::new([1.0], 4).gradient(), 1);
        let (base, peak) = tones(&h);
        assert_ne!(h.fg(0, 0), h.fg(0, 3), "with colours the blend steps down the column");
        assert_eq!(h.fg(0, 0), Some(peak));
        h.set_depth(ColorDepth::Ansi16);
        let flat = base.to_ansi16();
        for row in 0..4 {
            assert_eq!(h.buffer()[(0, row)].fg, ratatui_core::style::Color::Indexed(flat), "row {row}");
        }
    }

    #[test]
    fn ascii_mode_prints_only_ascii_and_carries_the_last_cell() {
        let mut h = harness(Demo::new([1.0, 0.3], 3), 3);
        h.set_glyph_mode(GlyphMode::Ascii);
        let screen = h.screen();
        assert!(screen.is_ascii(), "ASCII mode draws no block:\n{screen}");
        let (base, _) = tones(&h);
        let track = h.env().theme().color("raised").expect("token");
        assert_eq!(h.bg(0, 0), Some(base), "whole cells take the tone of the level");
        assert_eq!(h.bg(2, 0), Some(track), "the ground a column stands on");
        let partial = h.bg(2, 2);
        assert!(partial.is_some_and(|tone| tone != base && tone != track), "the last cell carries the share");
    }

    #[test]
    fn a_level_bar_without_values_draws_nothing() {
        let empty = harness(Demo::new(Vec::new(), 3), 6);
        assert_eq!(empty.screen(), "\n\n\n");
        let thin = harness(Demo::new([1.0], 3), 1);
        assert_eq!(thin.screen(), "█\n█\n█\n", "a one-column area still has one column");
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        // A bar wider than the area, a gap wider than it, and a hundred values for four columns:
        // every one of them is laid out again rather than drawn past the last column.
        for (demo, screen) in [
            (Demo::new([1.0, 1.0], 3).bar_width(9), "████\n████\n████\n"),
            (Demo::new([1.0, 0.0, 1.0], 3), "██\n██ ▄\n██ █\n"),
            (Demo::new(vec![1.0; 100], 3), "██ █\n██ █\n██ █\n"),
        ] {
            assert_eq!(harness(demo, 4).screen(), screen);
        }
    }

    #[test]
    fn every_builtin_theme_gives_the_columns_and_their_caps_a_tone() {
        let registry = crate::theme::ThemeRegistry::builtin();
        for (id, _) in registry.list() {
            let theme = registry.resolve(&id).theme.expect("resolves");
            let style = theme.style("level-bars", None, &[]);
            let tone = |key: &str| style.paint(key).map(|paint| paint.at(0.0));
            let (Some(base), Some(peak)) = (tone("base"), tone("peak")) else {
                panic!("theme {id} names no `base` or `peak` tone");
            };
            assert!(
                base.perceptual_distance(peak) > 0.05,
                "theme {id}: a cap in the peak tone would be lost on the column: {base} against {peak}"
            );
        }
    }

    #[test]
    fn a_turn_hands_the_leftover_cells_to_the_ends_in_turn() {
        assert_eq!((0..3).map(|i| turn(i, 3)).collect::<Vec<_>>(), vec![0, 2, 1]);
        assert_eq!((0..4).map(|i| turn(i, 4)).collect::<Vec<_>>(), vec![0, 2, 3, 1]);
    }
}
