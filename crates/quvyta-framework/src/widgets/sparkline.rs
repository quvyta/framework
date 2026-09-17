//! Sparklines: a series of numbers as a row of small columns.

use super::eighths;
use crate::color::Rgb;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::icons::GlyphMode;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// A compact trend: one column per value, newest on the right, measured in eighths of a cell.
///
/// Columns scale from the lowest to the highest value shown, or over a fixed range. Every value
/// keeps at least one eighth so a quiet moment still reads as a sample. ASCII mode has no partial
/// blocks: whole cells fill with colour and the cell a column ends in takes the share of colour it
/// covers, so even the lowest sample tints one cell. When the series is wider than the area, the newest values are kept. Give the node a
/// height for taller columns.
///
/// Style keys: `sparkline` (`fg` for the columns, `peak` and `low` for the highlighted extremes,
/// `baseline` for the tone band of a reference value, `track` for the ground in ASCII mode).
#[derive(Debug, Clone, PartialEq)]
pub struct Sparkline {
    values: Vec<f32>,
    range: Option<(f32, f32)>,
    extremes: bool,
    baseline: Option<f32>,
}

impl Sparkline {
    /// A sparkline of `values`, oldest first.
    #[must_use]
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        Self { values: values.into_iter().collect(), range: None, extremes: false, baseline: None }
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

    fn scale(&self, shown: &[f32]) -> (f32, f32) {
        let (min, max) = self.range.unwrap_or_else(|| {
            let min = shown.iter().copied().fold(f32::INFINITY, f32::min);
            let max = shown.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            (min, max)
        });
        (min, if max > min { max } else { min + 1.0 })
    }
}

impl<Msg: 'static> Widget<Msg> for Sparkline {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let width = clamp_u16(i32::try_from(self.values.len()).unwrap_or(i32::MAX));
        Size::new(width, u16::from(width > 0)).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() || self.values.is_empty() {
            return;
        }
        let count = self.values.len().min(usize::from(area.width));
        let shown = &self.values[self.values.len() - count..];
        let (min, max) = self.scale(shown);
        let style = cx.style("sparkline", None, &[]);
        let fill = style.color("fg").unwrap_or_else(|| cx.color("accent"));
        let peak = style.color("peak").unwrap_or(fill);
        let low = style.color("low").unwrap_or(fill);
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

        let (peak_index, low_index) = if self.extremes { extremes(shown) } else { (None, None) };
        for (index, value) in shown.iter().copied().enumerate() {
            let color = if Some(index) == peak_index {
                peak
            } else if Some(index) == low_index {
                low
            } else {
                fill
            };
            let column = Rect::new(area.x + i32::try_from(index).unwrap_or(0), area.y, 1, cells);
            if ascii {
                paint_ascii_column(cx, column, level(value), color, |row| match band {
                    Some((band_row, band_color)) if band_row == row => band_color,
                    _ => track,
                });
            } else {
                eighths::vertical(cx, column, level(value), color);
            }
        }
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

    struct Demo(Sparkline, u16);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(self.0.clone()).height(Length::Cells(self.1));
        }
    }

    /// The column colour of the built-in theme.
    fn column(h: &Harness<Demo>) -> crate::color::Rgb {
        let theme = h.env().theme();
        theme.color("surface").expect("token").mix(theme.color("accent").expect("token"), 0.72)
    }

    const LOAD: [f32; 8] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];

    #[test]
    fn one_row_uses_eighth_blocks_and_keeps_newest() {
        let h = Harness::new(Demo(Sparkline::new(LOAD), 1), 8, 1);
        assert_eq!(h.screen(), "▁▂▃▄▅▆▇\n");
        assert_eq!(h.bg(7, 0), Some(column(&h)));
        let narrow = Harness::new(Demo(Sparkline::new(LOAD), 1), 4, 1);
        assert_eq!(narrow.screen(), "▁▃▆\n");
    }

    #[test]
    fn taller_columns_and_fixed_range() {
        let h = Harness::new(Demo(Sparkline::new([50.0, 100.0, 0.0]).range(0.0, 100.0), 2), 3, 2);
        assert_eq!(h.screen(), "▁\n  ▁\n");
        assert_eq!(h.bg(0, 1), Some(column(&h)));
        assert_eq!(h.bg(1, 0), Some(column(&h)));
    }

    #[test]
    fn extremes_and_baseline_are_coloured() {
        let spark = Sparkline::new([3.0, 9.0, 1.0, 5.0]).highlight_extremes();
        let h = Harness::new(Demo(spark.clone(), 1), 4, 1);
        assert_eq!(h.screen(), "▃ ▁▅\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(1, 0), theme.color("accent"), "the peak is the brightest column");
        assert_eq!(h.fg(2, 0), theme.color("muted"), "the low column is muted");
        assert_ne!(h.fg(0, 0), h.fg(2, 0));
        let lined = Harness::new(Demo(spark.baseline(5.0), 1), 4, 1);
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
        let mut h = Harness::new(Demo(Sparkline::new([0.0, 10.0]), 2), 2, 2);
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
        let mut h = Harness::new(Demo(Sparkline::new([0.0, 1.0, 50.0, 100.0]), 3), 4, 3);
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
        let mut h = Harness::new(Demo(Sparkline::new(LOAD), 1), 8, 1);
        h.set_glyph_mode(GlyphMode::Ascii);
        let brightness = |x: u16| h.bg(x, 0).map_or(0, |c| u32::from(c.r) + u32::from(c.g) + u32::from(c.b));
        let raised = h.env().theme().color("raised");
        assert!((0..8).all(|x| h.bg(x, 0) != raised), "every sample tints its cell");
        assert!((1..8).all(|x| brightness(x) != brightness(x - 1)), "rising values read as rising tones");
        assert_eq!(h.bg(7, 0), Some(column(&h)), "the highest is whole colour");
    }
}
