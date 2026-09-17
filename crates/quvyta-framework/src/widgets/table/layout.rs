//! Column widths of a [`Table`] and which columns show when they scroll sideways.

use std::sync::Arc;

use crate::geometry::clamp_u16;

use super::{COLUMN_GAP, ColumnWidth, Table, TableMemory};
/// Where a visible column was drawn in the last frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Placed {
    pub(super) column: usize,
    pub(super) x: i32,
    pub(super) width: u16,
}

impl<Msg: 'static> Table<Msg> {
    pub(super) fn widest_cells(&self, memory: &mut TableMemory) -> Vec<u16> {
        if let Some((rows, widths)) = &memory.fit
            && Arc::ptr_eq(rows, &self.rows)
            && widths.len() == self.columns.len()
        {
            return widths.clone();
        }
        let mut widths = vec![0u16; self.columns.len()];
        for row in self.rows.iter() {
            for (width, cell) in widths.iter_mut().zip(&row.cells) {
                *width = (*width).max(cell.width());
            }
        }
        memory.fit = Some((Arc::clone(&self.rows), widths.clone()));
        widths
    }

    /// Column widths for `room` cells, and whether they had to overflow it.
    pub(super) fn widths(&self, widest: &[u16], room: u16) -> (Vec<u16>, bool) {
        let mut widths: Vec<u16> = self
            .columns
            .iter()
            .zip(widest)
            .map(|(column, widest)| match column.width {
                ColumnWidth::Fixed(cells) => cells,
                // The first column gives one cell to the selection slide.
                ColumnWidth::Fit => widest.saturating_add(1).max(column.title_width()).max(column.min.unwrap_or(0)),
                ColumnWidth::Fill(_) => column.min.unwrap_or_else(|| column.title_width().max(3)),
            })
            .collect();
        let gaps = COLUMN_GAP.saturating_mul(clamp_u16(i32::try_from(widths.len().saturating_sub(1)).unwrap_or(0)));
        let used = widths.iter().fold(0u16, |sum, width| sum.saturating_add(*width)).saturating_add(gaps);
        if used > room {
            return (widths, true);
        }
        let mut free = room - used;
        let weights: Vec<(usize, u16)> = self
            .columns
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c.width {
                ColumnWidth::Fill(weight) => Some((i, weight.max(1))),
                _ => None,
            })
            .collect();
        let total_weight: u32 = weights.iter().map(|(_, w)| u32::from(*w)).sum();
        let share = free;
        for (position, (index, weight)) in weights.iter().enumerate() {
            let extra = if position + 1 == weights.len() {
                free
            } else {
                clamp_u16(i32::try_from(u32::from(share) * u32::from(*weight) / total_weight).unwrap_or(0)).min(free)
            };
            widths[*index] += extra;
            free -= extra;
        }
        (widths, false)
    }

    /// Visible columns from `offset`, positioned from `x` within `room` cells.
    pub(super) fn place(widths: &[u16], offset: usize, x: i32, room: u16) -> Vec<Placed> {
        let mut placed = Vec::new();
        let mut left = x;
        let right = x + i32::from(room);
        for (column, width) in widths.iter().enumerate().skip(offset) {
            let space = right - left;
            if space < 3 {
                break;
            }
            let width = (*width).min(clamp_u16(space));
            placed.push(Placed { column, x: left, width });
            left += i32::from(width) + i32::from(COLUMN_GAP);
        }
        placed
    }

    /// The largest column offset that still shows the last column as fully as possible.
    pub(super) fn max_offset(widths: &[u16], room: u16) -> usize {
        let mut used = 0u16;
        for (index, width) in widths.iter().enumerate().rev() {
            let next = used.saturating_add(*width).saturating_add(if used == 0 { 0 } else { COLUMN_GAP });
            if next > room {
                return (index + 1).min(widths.len().saturating_sub(1));
            }
            used = next;
        }
        0
    }

    pub(super) fn spans(place: &Placed, x: i32) -> bool {
        x >= place.x && x < place.x + i32::from(place.width)
    }
}
