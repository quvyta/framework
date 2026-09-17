//! Bar charts: values compared side by side.

use super::cells;
use super::eighths;
use crate::geometry::{Rect, Size, clamp_u16};
use crate::text;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// Widest a vertical bar gets, in cells.
const MAX_BAR_WIDTH: u16 = 6;

/// Horizontal charts drop their labels below this width.
const LABEL_MIN_WIDTH: u16 = 24;

/// One bar of a [`BarChart`].
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    label: String,
    value: f32,
    value_text: Option<String>,
    variant: Option<String>,
}

impl Bar {
    /// A bar called `label` at `value`.
    #[must_use]
    pub fn new(label: impl Into<String>, value: f32) -> Self {
        Self { label: label.into(), value: value.max(0.0), value_text: None, variant: None }
    }

    /// Text shown for the value instead of the number, e.g. "1.2 GiB".
    #[must_use]
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.value_text = Some(text.into());
        self
    }

    /// Theme variant of this bar, e.g. `"danger"` for a service over its limit. A bar with a
    /// variant carries a marker before its value, so its meaning reads without colour.
    #[must_use]
    pub fn variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    fn shown_value(&self) -> String {
        self.value_text.clone().unwrap_or_else(|| {
            if self.value.fract() == 0.0 { format!("{:.0}", self.value) } else { format!("{:.1}", self.value) }
        })
    }
}

/// Bars measured in eighths of a cell, with labels and values.
///
/// Horizontal by default: one row per bar, labels on the left, values on the right. Vertical
/// charts stand bars side by side with the value above and the label below each. Bars scale to
/// the largest value unless a maximum is given. On narrow areas horizontal charts drop their
/// labels first; vertical bars get thinner and labels are cut with `…`. ASCII mode fills whole
/// cells.
///
/// Style keys: `bar-chart` and `bar-chart.<variant>` (`fill`), `bar-chart-label` (`fg`),
/// `bar-chart-value` and `bar-chart-value.<variant>` (`fg`, `bold`).
#[derive(Debug, Clone, PartialEq)]
pub struct BarChart {
    bars: Vec<Bar>,
    vertical: bool,
    max: Option<f32>,
    gap: u16,
}

impl BarChart {
    /// A horizontal chart of `bars`.
    #[must_use]
    pub fn new(bars: impl IntoIterator<Item = Bar>) -> Self {
        Self { bars: bars.into_iter().collect(), vertical: false, max: None, gap: 1 }
    }

    /// Stands the bars up side by side.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// The value of a full bar, e.g. `100.0` for percentages; the largest value by default.
    #[must_use]
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Empty rows (horizontal) or columns (vertical) between bars; 1 by default, so neighbouring
    /// bars never merge into one block. Vertical charts keep at least one column.
    #[must_use]
    pub fn gap(mut self, cells: u16) -> Self {
        self.gap = cells;
        self
    }

    fn scale(&self) -> f32 {
        let largest = self.bars.iter().map(|bar| bar.value).fold(0.0, f32::max);
        let max = self.max.unwrap_or(largest);
        if max > 0.0 { max } else { 1.0 }
    }

    fn vertical_gap(&self) -> u16 {
        self.gap.max(1)
    }

    fn count(&self) -> u16 {
        clamp_u16(i32::try_from(self.bars.len()).unwrap_or(i32::MAX))
    }

    /// The value text of `bar` with its marker, if it has a variant.
    fn value_label(&self, cx: &PaintCx<'_>, bar: &Bar) -> String {
        match &bar.variant {
            Some(_) => format!("{} {}", cx.env().icons().glyph("dot"), bar.shown_value()),
            None => bar.shown_value(),
        }
    }
}

impl<Msg: 'static> Widget<Msg> for BarChart {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let count = self.count();
        if count == 0 {
            return Size::default();
        }
        let size = if self.vertical {
            Size::new(available.width, 8)
        } else {
            Size::new(available.width, count.saturating_add(self.gap.saturating_mul(count - 1)))
        };
        size.min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() || self.bars.is_empty() {
            return;
        }
        if self.vertical {
            self.paint_vertical(cx, area);
        } else {
            self.paint_horizontal(cx, area);
        }
    }
}

impl BarChart {
    fn styles(&self, cx: &mut PaintCx<'_>, bar: &Bar) -> (crate::color::Rgb, crate::style::CellStyle) {
        let variant = bar.variant.as_deref();
        let fill = cx.style("bar-chart", variant, &[]).color("fill").unwrap_or_else(|| cx.color("accent"));
        let mut value = cx.style("bar-chart-value", variant, &[]).text();
        value.bg = None;
        (fill, value)
    }

    fn label_style(cx: &mut PaintCx<'_>) -> crate::style::CellStyle {
        let mut style = cx.style("bar-chart-label", None, &[]).text();
        style.bg = None;
        style
    }

    fn paint_horizontal(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let scale = self.scale();
        let labels = area.width >= LABEL_MIN_WIDTH;
        let longest_label = self.bars.iter().map(|bar| text::width(&bar.label)).max().unwrap_or(0);
        let label_width = if labels { longest_label.min(area.width * 2 / 5) } else { 0 };
        let values: Vec<String> = self.bars.iter().map(|bar| self.value_label(cx, bar)).collect();
        let value_width = values.iter().map(|value| text::width(value)).max().unwrap_or(0);
        let label_space = if labels { label_width + 1 } else { 0 };
        let bar_width = area.width.saturating_sub(cells::sum([label_space, value_width, 1]));
        let label_style = Self::label_style(cx);

        for (index, (bar, value)) in self.bars.iter().zip(&values).enumerate() {
            let offset = u16::try_from(index).unwrap_or(u16::MAX).saturating_mul(self.gap.saturating_add(1));
            if offset >= area.height {
                break;
            }
            let y = area.y + i32::from(offset);
            if labels {
                let shown = text::truncate(&bar.label, label_width).into_owned();
                cx.text(area.x, y, &shown, label_style, label_width);
            }
            let (fill, value_style) = self.styles(cx, bar);
            let row = Rect::new(area.x + i32::from(label_space), y, bar_width, 1);
            eighths::horizontal(cx, row, eighths::eighths(bar.value / scale, bar_width), fill);
            let x = area.right() - i32::from(text::width(value));
            cx.text(x, y, value, value_style, value_width);
        }
    }

    fn paint_vertical(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let scale = self.scale();
        let count = self.count();
        let gap = self.vertical_gap();
        let slot = (area.width.saturating_sub(gap.saturating_mul(count - 1)) / count).max(1);
        let bar_width = slot.min(MAX_BAR_WIDTH);
        let label_style = Self::label_style(cx);
        // One row for the label below and one for the value above the tallest bar.
        let bar_rows = area.height.saturating_sub(2);

        for (index, bar) in self.bars.iter().enumerate() {
            let column = u16::try_from(index).unwrap_or(u16::MAX);
            let x = area.x + i32::from(column) * (i32::from(slot) + i32::from(gap));
            if x >= area.right() {
                break;
            }
            let (fill, value_style) = self.styles(cx, bar);
            let filled = eighths::eighths(bar.value / scale, bar_rows);
            let bar_x = x + i32::from((slot - bar_width) / 2);
            let column_rect = Rect::new(bar_x, area.y + 1, bar_width, bar_rows);
            eighths::vertical(cx, column_rect, filled, fill);

            let top = column_rect.bottom() - i32::try_from(filled.div_ceil(8)).unwrap_or(0) - 1;
            let value = self.value_label(cx, bar);
            let shown = text::truncate(&value, slot).into_owned();
            let value_x = x + i32::from((slot - text::width(&shown)) / 2);
            cx.text(value_x, top, &shown, value_style, slot);

            let label = text::truncate(&bar.label, slot).into_owned();
            let label_x = x + i32::from((slot - text::width(&label)) / 2);
            cx.text(label_x, area.bottom() - 1, &label, label_style, slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};

    struct Demo(BarChart, u16);

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, _: ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(self.0.clone()).fill_width().height(Length::Cells(self.1));
        }
    }

    fn services() -> BarChart {
        BarChart::new([Bar::new("api", 40.0), Bar::new("postgres", 25.0), Bar::new("worker", 90.0).variant("danger")])
    }

    #[test]
    fn horizontal_bars_with_labels_and_values() {
        let h = Harness::new(Demo(services(), 5), 30, 5);
        assert_eq!(
            h.screen(),
            "api             ▏           40\n\npostgres     ▌              25\n\nworker                    ● 90\n"
        );
        let theme = h.env().theme();
        assert_eq!(h.bg(9, 4), theme.color("danger"));
        assert_eq!(h.fg(26, 4), theme.color("danger"));
        assert_ne!(h.bg(9, 3), theme.color("danger"));
    }

    #[test]
    fn narrow_horizontal_drops_labels() {
        let h = Harness::new(Demo(services().gap(0), 3), 16, 3);
        assert_eq!(h.screen(), "    ▉         40\n              25\n            ● 90\n");
        assert_eq!(h.bg(2, 1), h.env().theme().color("accent"));
    }

    #[test]
    fn vertical_bars_stand_with_value_above_and_label_below() {
        let h = Harness::new(Demo(services().vertical().max(100.0), 6), 24, 6);
        assert_eq!(
            h.screen(),
            "                 ● 90\n                ▅▅▅▅▅▅\n  40\n▅▅▅▅▅▅    25\n\n  api   postgr… worker\n"
        );
        let theme = h.env().theme();
        assert_eq!(h.bg(0, 4), theme.color("accent"));
        assert_eq!(h.bg(8, 4), theme.color("accent"));
        assert_eq!(h.bg(16, 2), theme.color("danger"));
    }

    #[test]
    fn ascii_rounds_to_cells() {
        let mut h = Harness::new(Demo(BarChart::new([Bar::new("a", 1.0), Bar::new("b", 0.55)]).gap(0), 2), 30, 2);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert_eq!(h.screen(), "a                            1\nb                          0.6\n");
        let theme = h.env().theme();
        assert_eq!(h.bg(25, 0), theme.color("accent"));
        assert_eq!(h.bg(14, 1), theme.color("accent"));
        assert_ne!(h.bg(15, 1), theme.color("accent"));
    }

    #[test]
    fn a_huge_gap_leaves_room_for_the_first_bar_only() {
        let h = Harness::new(Demo(services().gap(u16::MAX), 5), 30, 5);
        assert!(h.screen().starts_with("api "), "{}", h.screen());
        assert!(!h.screen().contains("postgres"), "{}", h.screen());
        let h = Harness::new(Demo(services().vertical().gap(u16::MAX), 6), 24, 6);
        assert!(h.screen().ends_with("\n…\n"), "one thin bar with its label cut:\n{}", h.screen());
    }
}
