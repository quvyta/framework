//! Where a bar chart's categories and their bars sit, and how a stacked bar's whole cells are
//! shared out between its series. Painting and input both read the geometry from here, so the
//! category under the pointer is always the one drawn there.

use crate::geometry::Rect;

use super::{BarChart, MAX_BAR_WIDTH};

/// The rows of a horizontal chart.
pub(super) struct RowLayout {
    /// Rows of bars one category takes.
    pub(super) bars: u16,
    /// Rows from the top of one category to the top of the next, gap included.
    pub(super) step: usize,
}

/// The columns of a vertical chart.
pub(super) struct ColumnLayout {
    /// Cells one category takes.
    pub(super) slot: u16,
    /// Empty columns between categories.
    pub(super) gap: u16,
    /// Bars drawn per category.
    pub(super) bars: u16,
    /// Cells one bar of a category may use, air included.
    pub(super) inner: u16,
    /// Cells one bar is drawn wide.
    pub(super) bar: u16,
    /// Whether the series are drawn as one stacked bar, which a slot too thin for a group of
    /// bars falls back to rather than dropping series.
    pub(super) stacked: bool,
}

impl<Msg> BarChart<Msg> {
    pub(super) fn row_layout(&self) -> RowLayout {
        let bars = self.bars_per_category();
        RowLayout { bars, step: usize::from(bars).saturating_add(usize::from(self.gap)) }
    }

    pub(super) fn column_layout(&self, width: u16) -> ColumnLayout {
        let count = self.count().max(1);
        let gap = self.vertical_gap();
        let slot = (width.saturating_sub(gap.saturating_mul(count - 1)) / count).max(1);
        let wanted = self.bars_per_category();
        let inner = slot / wanted;
        if wanted > 1 && inner == 0 {
            let bar = slot.min(MAX_BAR_WIDTH);
            return ColumnLayout { slot, gap, bars: 1, inner: slot, bar, stacked: true };
        }
        let bar = inner.clamp(1, MAX_BAR_WIDTH);
        ColumnLayout { slot, gap, bars: wanted, inner, bar, stacked: self.stacked }
    }

    /// The cells one category owns: its rows across the whole width in a horizontal chart, its
    /// slot over the whole height in a vertical one. Empty once the category falls outside
    /// `area`.
    pub(super) fn category_rect(&self, area: Rect, category: usize) -> Rect {
        if self.vertical {
            let columns = self.column_layout(area.width);
            let step = usize::from(columns.slot).saturating_add(usize::from(columns.gap));
            let Ok(offset) = i32::try_from(step.saturating_mul(category)) else {
                return Rect::default();
            };
            Rect::new(area.x.saturating_add(offset), area.y, columns.slot, area.height).intersect(area)
        } else {
            let rows = self.row_layout();
            let Ok(offset) = i32::try_from(rows.step.saturating_mul(category)) else {
                return Rect::default();
            };
            Rect::new(area.x, area.y.saturating_add(offset), area.width, rows.bars).intersect(area)
        }
    }

    /// Where one bar of a category is drawn: the left edge of the cells it may use, and the left
    /// edge of the bar itself, which sits in the middle of them.
    pub(super) fn bar_column(&self, area: Rect, category: usize, bar: usize) -> (i32, i32) {
        let columns = self.column_layout(area.width);
        let slot = self.category_rect(area, category);
        let group = columns.bars.saturating_mul(columns.inner);
        let air = i32::from(columns.slot.saturating_sub(group) / 2);
        let Ok(offset) = i32::try_from(usize::from(columns.inner).saturating_mul(bar)) else {
            return (slot.right(), slot.right());
        };
        let left = slot.x + air + offset;
        (left, left + i32::from(columns.inner.saturating_sub(columns.bar) / 2))
    }

    /// The category the cell `(x, y)` belongs to, if any. The gaps between categories belong to
    /// nothing, so a click there changes no selection.
    pub(super) fn category_at(&self, area: Rect, x: i32, y: i32) -> Option<usize> {
        if !area.contains(x, y) {
            return None;
        }
        let (offset, own, step) = if self.vertical {
            let columns = self.column_layout(area.width);
            let step = usize::from(columns.slot).saturating_add(usize::from(columns.gap));
            (x - area.x, usize::from(columns.slot), step)
        } else {
            let rows = self.row_layout();
            (y - area.y, usize::from(rows.bars), rows.step)
        };
        let offset = usize::try_from(offset).ok()?;
        let category = offset / step;
        (offset % step < own && category < self.categories()).then_some(category)
    }
}

/// Shares `cells` whole cells out between `values`, largest remainder first, so the cells add up
/// to exactly `cells` and the biggest share of a short bar is the one that gets a cell.
pub(super) fn apportion(values: &[f32], cells: u16) -> Vec<u16> {
    let total: f32 = values.iter().sum();
    if total <= 0.0 || cells == 0 {
        return vec![0; values.len()];
    }
    let mut shares = Vec::with_capacity(values.len());
    let mut remainders = Vec::with_capacity(values.len());
    let mut given: u16 = 0;
    for (index, value) in values.iter().enumerate() {
        // `exact` is at most `cells`, so the floor fits the cell count.
        let exact = value / total * f32::from(cells);
        let whole = exact.floor();
        let share = whole.max(0.0).min(f32::from(cells)) as u16;
        shares.push(share);
        given = given.saturating_add(share);
        remainders.push((exact - whole, index));
    }
    remainders.sort_by(|left, right| right.0.total_cmp(&left.0).then(left.1.cmp(&right.1)));
    let mut left = cells.saturating_sub(given);
    for (remainder, index) in remainders {
        if left == 0 || remainder <= 0.0 {
            break;
        }
        shares[index] = shares[index].saturating_add(1);
        left -= 1;
    }
    shares
}

#[cfg(test)]
mod tests {
    use super::apportion;

    #[test]
    fn cells_add_up_and_go_to_the_largest_shares_first() {
        assert_eq!(apportion(&[1.0, 1.0, 1.0], 3), vec![1, 1, 1]);
        assert_eq!(apportion(&[3.0, 1.0], 8), vec![6, 2]);
        // 10 cells over 3/2/1: 5, 3.33, 1.66 — the last remainder wins the leftover cell.
        assert_eq!(apportion(&[3.0, 2.0, 1.0], 10), vec![5, 3, 2]);
        assert_eq!(apportion(&[9.0, 1.0], 1), vec![1, 0], "the one cell shows the larger share");
        assert_eq!(apportion(&[0.0, 0.0], 4), vec![0, 0], "nothing to share out");
        assert_eq!(apportion(&[1.0, 1.0], 0), vec![0, 0]);
        let shares = apportion(&[0.1, 0.0, 0.9], 5);
        assert_eq!(shares, vec![1, 0, 4]);
        assert_eq!(shares.iter().sum::<u16>(), 5);
    }
}
