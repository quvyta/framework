//! Drawing a bar chart: rows of bars reaching right, or bars standing side by side.

use crate::color::Rgb;
use crate::geometry::{Rect, clamp_u16};
use crate::style::CellStyle;
use crate::text;
use crate::widget::PaintCx;

use super::super::cells;
use super::super::eighths;
use super::super::row::LEAD;
use super::layout::apportion;
use super::{BarChart, LABEL_MIN_WIDTH};

impl<Msg: 'static> BarChart<Msg> {
    /// One row per bar: the label on the left, the bar in the middle, the value on the right.
    pub(super) fn paint_horizontal(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let interactive = self.interactive();
        let lead = if self.marks_selection() { LEAD } else { 0 };
        let labels = area.width >= LABEL_MIN_WIDTH;
        let longest_label = self.bars.iter().map(|bar| text::width(&bar.label)).max().unwrap_or(0);
        let label_width = if labels { longest_label.min(area.width * 2 / 5) } else { 0 };
        let values: Vec<Vec<String>> = (0..self.categories()).map(|c| self.row_values(cx, c, labels)).collect();
        let value_width = values.iter().flatten().map(|value| text::width(value)).max().unwrap_or(0);
        let label_space = if labels { label_width + 1 } else { 0 };
        let bar_width = area.width.saturating_sub(cells::sum([lead, label_space, value_width, 1]));
        let focused = interactive && cx.is_focused();
        let pointer = if interactive { cx.pointer() } else { None };
        let label_style = self.label_style(cx);

        for (category, row_values) in values.iter().enumerate() {
            let slot = self.category_rect(area, category);
            if slot.is_empty() {
                break;
            }
            let hovered = pointer.is_some_and(|(x, y)| slot.contains(x, y));
            let states = self.states(hovered, category, focused);
            self.paint_ground(cx, slot, &states);
            let bar = &self.bars[category];
            if labels {
                let shown = text::truncate(&bar.label, label_width).into_owned();
                cx.text(slot.x + i32::from(lead), slot.y, &shown, label_style, label_width);
            }
            let value_style = self.value_style(cx, bar);
            for (index, value) in row_values.iter().enumerate() {
                let y = slot.y + i32::try_from(index).unwrap_or(i32::MAX);
                if y >= slot.bottom() {
                    break;
                }
                let row = Rect::new(slot.x + i32::from(cells::sum([lead, label_space])), y, bar_width, 1);
                if self.stacked && !self.series.is_empty() {
                    self.paint_stack(cx, row, category);
                } else {
                    let fill = self.fill(cx, bar, Some(index));
                    eighths::horizontal(cx, row, self.reached(self.value(category, index), bar_width), fill);
                }
                let x = area.right() - i32::from(text::width(value));
                cx.text(x, y, value, value_style, value_width);
            }
        }
    }

    /// The values written beside one category's bars: the bar's own text for a chart of plain
    /// bars, the total for a stack, one value per series otherwise, each after its series name
    /// while the chart is wide enough to name things.
    fn row_values(&self, cx: &PaintCx<'_>, category: usize, names: bool) -> Vec<String> {
        let bar = &self.bars[category];
        if self.series.is_empty() {
            return vec![self.bar_value(cx, bar)];
        }
        if self.stacked {
            return vec![self.format(self.total(category))];
        }
        if self.series.len() < 2 {
            return vec![self.format(self.value(category, 0))];
        }
        self.series
            .iter()
            .map(|series| {
                let value = self.format(series.value(category));
                if names { format!("{} {}", series.name, value) } else { value }
            })
            .collect()
    }

    /// Draws one category's series as segments of a single bar. Segment ends fall on cell edges,
    /// so two tones never share a cell; the eighth-cell tail at the end of the bar belongs to the
    /// last series in it.
    fn paint_stack(&self, cx: &mut PaintCx<'_>, row: Rect, category: usize) {
        let values = self.values(category);
        let reached = self.reached(values.iter().sum(), row.width);
        let full = clamp_u16(i32::try_from(reached / 8).unwrap_or(i32::MAX)).min(row.width);
        let bar = &self.bars[category];
        let mut x = row.x;
        for (index, share) in apportion(&values, full).into_iter().enumerate() {
            if share == 0 {
                continue;
            }
            let fill = self.fill(cx, bar, Some(index));
            let segment = Rect::new(x, row.y, share, row.height);
            cx.fill(segment, fill);
            self.paint_segment_name(cx, segment, index, fill);
            x = x.saturating_add(i32::from(share));
        }
        let partial = reached % 8;
        if partial > 0 && x < row.right() {
            let last = values.iter().rposition(|value| *value > 0.0).unwrap_or(0);
            let fill = self.fill(cx, bar, Some(last));
            let tail = Rect::new(x, row.y, clamp_u16(row.right() - x), row.height);
            eighths::horizontal(cx, tail, partial, fill);
        }
    }

    /// Writes a segment's series name inside it when the name fits with a cell of air on each
    /// side, so a stack is not read by colour alone. A segment too narrow for its name stays
    /// blank and a legend beside the chart carries the meaning.
    fn paint_segment_name(&self, cx: &mut PaintCx<'_>, segment: Rect, series: usize, fill: Rgb) {
        let Some(name) = self.series.get(series).map(|series| series.name.clone()) else {
            return;
        };
        let width = text::width(&name);
        if width == 0 || segment.width < width.saturating_add(2) {
            return;
        }
        cx.text(segment.x + 1, segment.y, &name, CellStyle::fg(readable_on(cx, fill)), width);
    }

    /// Bars standing side by side, the value above and the label below each.
    pub(super) fn paint_vertical(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let columns = self.column_layout(area.width);
        let interactive = self.interactive();
        let focused = interactive && cx.is_focused();
        let pointer = if interactive { cx.pointer() } else { None };
        let label_style = self.label_style(cx);
        // One row for the label below and one for the value above the tallest bar. An area too
        // short for both gives up the value row first and then the labels, keeping the bars.
        let label_row = area.height >= 2;
        let value_row = area.height >= 3;
        let bar_rows = area.height.saturating_sub(u16::from(label_row) + u16::from(value_row));
        let top = area.y + i32::from(u16::from(value_row));

        for category in 0..self.categories() {
            let slot = self.category_rect(area, category);
            if slot.is_empty() {
                break;
            }
            let hovered = pointer.is_some_and(|(x, y)| slot.contains(x, y));
            let states = self.states(hovered, category, focused);
            self.paint_ground(cx, slot, &states);
            let bar = &self.bars[category];
            let value_style = self.value_style(cx, bar);
            for index in 0..usize::from(columns.bars) {
                let (cells_x, bar_x) = self.bar_column(area, category, index);
                if cells_x >= slot.right() {
                    break;
                }
                let column = Rect::new(bar_x, top, columns.bar, bar_rows);
                let filled = if columns.stacked && !self.series.is_empty() {
                    self.paint_vertical_stack(cx, column, category)
                } else {
                    let filled = self.reached(self.value(category, index), bar_rows);
                    let fill = self.fill(cx, bar, Some(index));
                    eighths::vertical(cx, column, filled, fill);
                    filled
                };
                if !value_row {
                    continue;
                }
                let value = if columns.stacked && !self.series.is_empty() {
                    self.format(self.total(category))
                } else {
                    self.value_above(cx, category, index)
                };
                let shown = text::truncate(&value, columns.inner).into_owned();
                // A group of bars leaves a value out rather than cutting it to nothing.
                if columns.bars > 1 && text::width(&shown) < text::width(&value) {
                    continue;
                }
                let y = column.bottom() - i32::try_from(filled.div_ceil(8)).unwrap_or(0) - 1;
                let x = cells_x + i32::from((columns.inner.saturating_sub(text::width(&shown))) / 2);
                cx.text(x, y, &shown, value_style, columns.inner);
            }
            if label_row {
                let label = text::truncate(&bar.label, columns.slot).into_owned();
                let x = slot.x + i32::from((columns.slot.saturating_sub(text::width(&label))) / 2);
                cx.text(x, area.bottom() - 1, &label, label_style, columns.slot);
            }
        }
    }

    /// The value written above one bar of a vertical chart.
    fn value_above(&self, cx: &PaintCx<'_>, category: usize, index: usize) -> String {
        if self.series.is_empty() {
            self.bar_value(cx, &self.bars[category])
        } else {
            self.format(self.value(category, index))
        }
    }

    /// Draws one category's series as segments of a single standing bar and returns how many
    /// eighths of a cell the whole bar reaches.
    fn paint_vertical_stack(&self, cx: &mut PaintCx<'_>, column: Rect, category: usize) -> u32 {
        let values = self.values(category);
        let reached = self.reached(values.iter().sum(), column.height);
        let full = clamp_u16(i32::try_from(reached / 8).unwrap_or(i32::MAX)).min(column.height);
        let bar = &self.bars[category];
        let mut bottom = column.bottom();
        for (index, share) in apportion(&values, full).into_iter().enumerate() {
            if share == 0 {
                continue;
            }
            let fill = self.fill(cx, bar, Some(index));
            bottom -= i32::from(share);
            cx.fill(Rect::new(column.x, bottom, column.width, share), fill);
        }
        let partial = reached % 8;
        if partial > 0 && bottom > column.y {
            let last = values.iter().rposition(|value| *value > 0.0).unwrap_or(0);
            let fill = self.fill(cx, bar, Some(last));
            let tail = Rect::new(column.x, column.y, column.width, clamp_u16(bottom - column.y));
            eighths::vertical(cx, tail, partial, fill);
        }
        reached
    }
}

/// The theme colour that reads best on `fill`: the ink meant for the accent, or the text colour
/// when the fill is dark enough for it.
fn readable_on(cx: &PaintCx<'_>, fill: Rgb) -> Rgb {
    let ink = cx.color("ink");
    let text = cx.color("text");
    if fill.contrast_ratio(ink) >= fill.contrast_ratio(text) { ink } else { text }
}
