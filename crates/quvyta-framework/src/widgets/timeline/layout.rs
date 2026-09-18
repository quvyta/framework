//! Where a timeline's day, blocks and lanes sit. Painting and input both read the geometry from
//! here, so the block under the pointer is always the one drawn there.

use crate::date::TimeOfDay;
use crate::geometry::Rect;

use super::super::axis::{DAY, span_between, time_cell};
use super::{TimeBlock, Timeline};

/// Cells an open edge fades over: two give a gradient that reads as running on, and leave most
/// of a short block in its own tone.
const FADE: u16 = 2;

/// A block placed on the strip: which one, where it lies in the day and its lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Placed {
    /// Index into the blocks given.
    pub(super) index: usize,
    /// Seconds from the day's start to the block's start.
    pub(super) start: u32,
    /// Seconds from the day's start to the block's end, cut at the day's end.
    pub(super) end: u32,
    /// The lane, 0 on top.
    pub(super) lane: u16,
}

/// The rows of a timeline in an area: strip rows from the top, then the axis, then the readout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Rows {
    /// Rows the lanes are drawn in, at least one.
    pub(super) strip: u16,
    /// The row of the axis, counted from the top, when it is shown.
    pub(super) axis: Option<u16>,
    /// The row of the readout, counted from the top, when it is shown.
    pub(super) readout: Option<u16>,
}

impl<Msg: 'static> Timeline<Msg> {
    /// Seconds from the day's start to `time`.
    pub(super) fn offset(&self, time: TimeOfDay) -> u32 {
        (time.seconds_since_midnight() + DAY - self.day_start.seconds_since_midnight()) % DAY
    }

    /// The clock time `offset` seconds after the day's start.
    pub(super) fn clock(&self, offset: u32) -> TimeOfDay {
        TimeOfDay::from_seconds_since_midnight((self.day_start.seconds_since_midnight() + offset) % DAY)
    }

    /// Where `block` lies in the day, as seconds from the day's start to its start and to its
    /// end, the end cut at the day's end.
    pub(super) fn extent(&self, block: &TimeBlock) -> (u32, u32) {
        let start = self.offset(block.start);
        let length = (block.end.seconds_since_midnight() + DAY - block.start.seconds_since_midnight()) % DAY;
        (start, (start + length).min(DAY))
    }

    /// The visible stretch: seconds from the day's start to its left edge, and how many seconds
    /// it spans. The whole day without a range; a range is kept inside the day.
    pub(super) fn visible(&self) -> (u32, u32) {
        match self.range {
            None => (0, DAY),
            Some((from, to)) => {
                let start = self.offset(from);
                (start, span_between(from, to).min(DAY - start))
            }
        }
    }

    /// Every block with its place in the day and its lane: blocks taken in time order, each in
    /// the first lane from the top where nothing overlaps it. A block that ends where another
    /// starts does not overlap it.
    pub(super) fn placed(&self) -> Vec<Placed> {
        let mut placed: Vec<Placed> = self
            .blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                let (start, end) = self.extent(block);
                Placed { index, start, end, lane: 0 }
            })
            .collect();
        placed.sort_by_key(|p| (p.start, p.end, p.index));
        // The end of the last block in each lane; a moment still takes a second, so two moments
        // at the same time stand in two lanes.
        let mut lane_ends: Vec<u32> = Vec::new();
        for block in &mut placed {
            let end = block.end.max(block.start + 1);
            let lane = lane_ends.iter().position(|lane_end| *lane_end <= block.start);
            let lane = lane.unwrap_or_else(|| {
                lane_ends.push(0);
                lane_ends.len() - 1
            });
            lane_ends[lane] = end;
            block.lane = u16::try_from(lane).unwrap_or(u16::MAX);
        }
        placed
    }

    /// Lanes the blocks need; one for an empty day, which still shows its track.
    pub(super) fn lanes(&self) -> u16 {
        self.placed().iter().map(|p| p.lane.saturating_add(1)).max().unwrap_or(1)
    }

    /// The blocks in the order the keyboard walks them: by start, then from the top lane down.
    pub(super) fn order(&self) -> Vec<usize> {
        let mut placed = self.placed();
        placed.sort_by_key(|p| (p.start, p.lane, p.index));
        placed.into_iter().map(|p| p.index).collect()
    }

    /// The rows an area `height` cells tall gives each part. The strip keeps at least one row;
    /// the axis goes first when rows run short, then the readout, then lanes.
    pub(super) fn rows(&self, height: u16) -> Rows {
        let lanes = self.lanes();
        let mut axis = self.axis;
        let mut readout = self.readout;
        let wanted =
            |axis: bool, readout: bool| lanes.saturating_add(u16::from(axis)).saturating_add(u16::from(readout));
        if height < wanted(axis, readout) {
            axis = false;
        }
        if height < wanted(axis, readout) {
            readout = false;
        }
        let strip = lanes.min(height).max(1);
        Rows { strip, axis: axis.then_some(strip), readout: readout.then_some(strip + u16::from(axis)) }
    }

    /// The cells a placed block covers in a strip `width` cells wide, as its first column from
    /// the left edge and its width; `None` when the block lies outside the visible range. A block
    /// shorter than a cell still takes one.
    pub(super) fn columns(&self, placed: &Placed, width: u16) -> Option<(i32, u16)> {
        let (from, span) = self.visible();
        let to = from + span;
        let inside = if placed.end > placed.start {
            placed.end > from && placed.start < to
        } else {
            placed.start >= from && placed.start < to
        };
        if !inside || width == 0 {
            return None;
        }
        let first = time_cell(placed.start.max(from) - from, span, width);
        let last = time_cell(placed.end.min(to) - from, span, width);
        let first = first.min(i32::from(width) - 1);
        let cells = u16::try_from((last - first).max(1))
            .unwrap_or(1)
            .min(width.saturating_sub(u16::try_from(first).unwrap_or(0)));
        Some((first, cells))
    }

    /// How many cells fade at the start and at the end of a placed block `cells` wide: up to
    /// [`FADE`] at each open edge that is the block's own and in view, shared out so that at least
    /// one cell keeps the block's tone. An edge where a zoomed range cuts the block is not its own
    /// and stays square.
    pub(super) fn fades(&self, placed: &Placed, cells: u16) -> (u16, u16) {
        let block = &self.blocks[placed.index];
        let (from, span) = self.visible();
        let lead = block.open_start && placed.start >= from;
        let trail = block.open_end && placed.end <= from + span;
        let ends = u16::from(lead) + u16::from(trail);
        if ends == 0 {
            return (0, 0);
        }
        let each = (cells.saturating_sub(1) / ends).min(FADE);
        (if lead { each } else { 0 }, if trail { each } else { 0 })
    }

    /// The strip row a lane is drawn in: its own, or the last row when the strip has fewer.
    pub(super) fn row_of(lane: u16, strip: u16) -> u16 {
        lane.min(strip.saturating_sub(1))
    }

    /// The blocks in the order they are painted: as given, with the selected one last so it is
    /// never hidden under another.
    pub(super) fn paint_order(&self) -> Vec<Placed> {
        let mut placed = self.placed();
        placed.sort_by_key(|p| (self.selected == Some(p.index), p.index));
        placed
    }

    /// The block drawn at the cell `(x, y)`: the one painted last there, as it is the one seen.
    pub(super) fn block_at(&self, area: Rect, x: i32, y: i32) -> Option<usize> {
        if !area.contains(x, y) {
            return None;
        }
        let rows = self.rows(area.height);
        let row = u16::try_from(y - area.y).ok()?;
        if row >= rows.strip {
            return None;
        }
        let column = x - area.x;
        self.paint_order()
            .into_iter()
            .rev()
            .filter(|p| Self::row_of(p.lane, rows.strip) == row)
            .find(|p| {
                self.columns(p, area.width)
                    .is_some_and(|(first, cells)| column >= first && column < first + i32::from(cells))
            })
            .map(|p| p.index)
    }

    /// The time under column `x`, in seconds from the day's start.
    pub(super) fn time_at(&self, area: Rect, x: i32) -> u32 {
        let (from, span) = self.visible();
        let column = u64::try_from((x - area.x).max(0)).unwrap_or(0);
        let offset = column * u64::from(span) / u64::from(area.width.max(1));
        from + u32::try_from(offset).unwrap_or(span).min(span)
    }
}
