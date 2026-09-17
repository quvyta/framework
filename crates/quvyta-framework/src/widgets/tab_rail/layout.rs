//! Where the rows of a [`TabRail`] sit: tabs, the add row, the collapsed name card, and what the
//! pointer is over.
//!
//! Every row is a block [`row_height`](TabRail::row_height) lines tall with
//! [`spacing`](TabRail::spacing) lines of canvas after it. Scrolling, hit testing and the scrollbar count
//! whole blocks, so a one-line rail with no gap is laid out exactly as a list of rows.

use crate::geometry::{Rect, clamp_u16};
use crate::text;

use super::super::cells;
use super::super::close_mark;
use super::super::edge_scroll::{Edge, Zone};
use super::super::scrollbar::ScrollMetrics;
use super::super::tab_model::TabHit;
use super::TabRail;

impl<Msg: 'static> TabRail<Msg> {
    /// Rows of the rail: the tabs and the add row.
    pub(super) fn rows(&self) -> usize {
        self.tabs.len() + usize::from(self.on_add.is_some())
    }

    /// Canvas lines between rows: the [`gap`](TabRail::gap) when set, otherwise one line between
    /// blocks taller than a line (tall blocks without it read as one column) and none between
    /// one-line rows.
    pub(super) fn spacing(&self) -> u16 {
        self.gap.unwrap_or(u16::from(self.row_height > 1))
    }

    /// The height of every block in an area `height` lines tall: the row height, cut to the area
    /// so a rail shorter than one block still shows a whole tab.
    fn block_height(&self, height: u16) -> u16 {
        self.row_height.min(height).max(1)
    }

    /// How many whole blocks fit in an area `height` lines tall; at least one when there is any
    /// room.
    pub(super) fn visible_blocks(&self, height: u16) -> usize {
        let gap = usize::from(self.spacing());
        let pitch = usize::from(self.block_height(height)) + gap;
        let fit = (usize::from(height) + gap) / pitch;
        fit.max(usize::from(height > 0))
    }

    /// The block of the `row`th visible row of `content`.
    fn block(&self, content: Rect, row: usize) -> Rect {
        let height = self.block_height(content.height);
        let pitch = i32::from(height) + i32::from(self.spacing());
        let y = content.y + i32::try_from(row).unwrap_or(0).saturating_mul(pitch);
        Rect::new(content.x, y, content.width, height)
    }

    /// The line of `block` that holds its content: the middle one, or the upper of the two middle
    /// lines of an even block.
    pub(super) fn middle(block: Rect) -> i32 {
        block.y + i32::from(block.height.saturating_sub(1) / 2)
    }

    /// The height a rail of `rows` rows asks for: its blocks and the gaps between them.
    pub(super) fn natural_height(&self, rows: usize) -> u16 {
        let rows = u16::try_from(rows.max(1)).unwrap_or(u16::MAX);
        rows.saturating_mul(self.row_height).saturating_add((rows - 1).saturating_mul(self.spacing()))
    }

    /// Where the add row sits on screen, if it is shown and scrolled into view.
    pub(super) fn add_rect(&self, content: Rect, offset: usize) -> Option<Rect> {
        self.on_add.as_ref()?;
        let row = self.tabs.len().checked_sub(offset)?;
        (row < self.visible_blocks(content.height)).then(|| self.block(content, row))
    }

    /// Cells right of the name in a block `height` lines tall: status dot and badge, each with a
    /// space before it, and the close mark when it shares the name's line. It does in blocks of
    /// one or two lines, whose first line is their middle line.
    pub(super) fn trailing_width(&self, index: usize, height: u16) -> u16 {
        let tab = &self.tabs[index];
        let status = if tab.status.is_some() { 2 } else { 0 };
        let badge = tab.badge.as_deref().map_or(0, |badge| text::width(badge).saturating_add(1));
        let close = if self.model.closable(index) && height <= 2 { close_mark::WIDTH } else { 0 };
        cells::sum([status, badge, close])
    }

    /// The close mark of tab `index` in its wide `block`: flush right on the block's first line,
    /// which in a one-line row is the row itself and in a taller block its top right corner, like
    /// the close mark of a dialog. None when the tab cannot close or the block has no room for the
    /// mark beside the pillar.
    pub(super) fn close_rect(&self, index: usize, block: Rect) -> Option<Rect> {
        (self.model.closable(index) && block.width > close_mark::WIDTH)
            .then(|| close_mark::rect(block.right() - i32::from(close_mark::WIDTH), block.y))
    }

    /// Tab blocks on screen, from the scroll offset, in `order`.
    pub(super) fn slots(&self, order: &[usize], offset: usize, area: Rect) -> Vec<(usize, Rect)> {
        order
            .iter()
            .skip(offset)
            .take(self.visible_blocks(area.height))
            .enumerate()
            .map(|(row, index)| (*index, self.block(area, row)))
            .collect()
    }

    /// The part of `area` the rows take, and the scroll position counted in rows.
    pub(super) fn content(&self, area: Rect, offset: usize) -> (Rect, ScrollMetrics) {
        let metrics = ScrollMetrics { total: self.rows(), visible: self.visible_blocks(area.height), offset };
        // The collapsed strip always keeps its scrollbar column, so it never changes width.
        let width = if self.collapsed || metrics.overflows() { area.width.saturating_sub(1) } else { area.width };
        (Rect::new(area.x, area.y, width, area.height), metrics)
    }

    /// The end of the rail a tab dragged to line `y` is held against, if the rows overflow: the
    /// first or last visible block (with the lines after it), or past the rail's top or bottom.
    /// A rail showing a single block has no inner blocks to spare, so only past its edges counts.
    /// Only the line matters, so a pointer that strays off the rail's side still scrolls.
    pub(super) fn drag_zone(&self, area: Rect, content: Rect, metrics: ScrollMetrics, y: i32) -> Option<Zone> {
        if !metrics.overflows() {
            return None;
        }
        let (top_end, bottom_start) = if metrics.visible >= 2 {
            (self.block(content, 0).bottom(), self.block(content, metrics.visible - 1).y)
        } else {
            (area.y, area.bottom())
        };
        if y < top_end {
            Some(Zone { edge: Edge::Back, beyond: clamp_u16(area.y - y) })
        } else if y >= bottom_start {
            Some(Zone { edge: Edge::Forward, beyond: clamp_u16(y + 1 - area.bottom()) })
        } else {
            None
        }
    }

    pub(super) fn identity(&self) -> Vec<usize> {
        (0..self.tabs.len()).collect()
    }

    pub(super) fn hit(&self, slots: &[(usize, Rect)], x: i32, y: i32) -> Option<TabHit> {
        let (index, rect) = slots.iter().find(|(_, rect)| rect.contains(x, y)).copied()?;
        if !self.collapsed && self.close_rect(index, rect).is_some_and(|close| close.contains(x, y)) {
            return Some(TabHit::Close(index));
        }
        Some(TabHit::Tab(index, rect))
    }

    /// Where the name card of the row at `anchor` sits on `screen`: right of the row and as tall as
    /// it, or pulled left when the screen ends first. A space, the name, two spaces and the badge,
    /// then the close mark or a closing space, on the row's middle line.
    pub(super) fn card(screen: Rect, anchor: Rect, name: &str, badge: Option<&str>, closable: bool) -> Rect {
        let badge_width = badge.map_or(0, |badge| text::width(badge).saturating_add(2));
        let end = if closable { close_mark::WIDTH } else { 1 };
        let width = cells::sum([1, text::width(name), badge_width, end]).min(screen.width);
        let x = anchor.right().min(screen.right() - i32::from(width)).max(screen.x);
        Rect::new(x, anchor.y, width, anchor.height)
    }

    /// What a press at `(x, y)` on the name card `rect` of tab `index` is on.
    pub(super) fn card_hit(rect: Rect, index: usize, closable: bool, x: i32, y: i32) -> TabHit {
        let close = close_mark::rect(rect.right() - i32::from(close_mark::WIDTH), Self::middle(rect));
        if closable && close.contains(x, y) { TabHit::Close(index) } else { TabHit::Tab(index, rect) }
    }
}
