//! Where the cards of a grid go: how many columns fit, how wide a card is, which card is under a
//! cell and where a key moves the selection. Pure arithmetic, shared by painting and input.

use crate::geometry::{Rect, clamp_u16};

/// The sizes a grid is asked for, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Sizing {
    /// Narrowest a card gets while more than one column fits.
    pub(super) min_width: u16,
    /// Widest a card gets; room beyond it stays free on the right.
    pub(super) max_width: u16,
    /// Rows of a whole card, its padding included.
    pub(super) height: u16,
    /// Cells between two cards of a row.
    pub(super) column_gap: u16,
    /// Rows between two rows of cards.
    pub(super) row_gap: u16,
}

/// A grid of `count` cards laid out in an area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct Layout {
    /// The cards' part of the area: all of it, less the scrollbar column when rows overflow.
    pub(super) body: Rect,
    pub(super) columns: usize,
    pub(super) card_width: u16,
    pub(super) card_height: u16,
    pub(super) column_gap: u16,
    pub(super) row_gap: u16,
    pub(super) count: usize,
    /// Rows of cards.
    pub(super) rows: usize,
    /// Rows of cards that fit whole; at least one, so a short area still shows the top of a row.
    pub(super) visible: usize,
    /// Whether there are more rows than fit, so the scrollbar is drawn.
    pub(super) overflows: bool,
}

/// Columns that fit in `width` and the width each card then gets.
///
/// A card never gets narrower than `min_width` while more than one column fits; an area narrower
/// than one card gives its whole width to a single column, so the card fills it and its content
/// is cut instead of drawn outside.
pub(super) fn columns(width: u16, sizing: Sizing) -> (usize, u16) {
    let min = u32::from(sizing.min_width.max(1));
    let max = sizing.max_width.max(sizing.min_width.max(1));
    let gap = u32::from(sizing.column_gap);
    let width32 = u32::from(width);
    if width32 <= min {
        return (1, width);
    }
    let columns = ((width32 + gap) / (min + gap)).max(1);
    let card = (width32 - gap * (columns - 1)) / columns;
    (usize::try_from(columns).unwrap_or(1), u16::try_from(card).unwrap_or(u16::MAX).min(max))
}

impl Layout {
    /// Lays out `count` cards in `area`.
    pub(super) fn new(area: Rect, count: usize, sizing: Sizing) -> Self {
        let pitch = u32::from(sizing.height.max(1)) + u32::from(sizing.row_gap);
        let fits = (u32::from(area.height) + u32::from(sizing.row_gap)) / pitch;
        let visible = usize::try_from(fits).unwrap_or(usize::MAX).max(1);
        let place = |width: u16| {
            let (columns, card_width) = columns(width, sizing);
            (columns, card_width, count.div_ceil(columns))
        };
        let (mut columns, mut card_width, mut rows) = place(area.width);
        let overflows = rows > visible;
        let mut body = area;
        if overflows {
            // The scrollbar takes the last column; the cards are laid out again without it.
            body.width = area.width.saturating_sub(1);
            (columns, card_width, rows) = place(body.width);
        }
        Self {
            body,
            columns,
            card_width,
            card_height: sizing.height.max(1),
            column_gap: sizing.column_gap,
            row_gap: sizing.row_gap,
            count,
            rows,
            visible,
            overflows,
        }
    }

    fn row_pitch(&self) -> i64 {
        i64::from(self.card_height) + i64::from(self.row_gap)
    }

    fn column_pitch(&self) -> i64 {
        i64::from(self.card_width) + i64::from(self.column_gap)
    }

    /// The first row of cards that may be shown: the last rows fill the area when scrolled down.
    pub(super) fn max_offset(&self) -> usize {
        self.rows.saturating_sub(self.visible)
    }

    /// The row and column of card `index`.
    pub(super) fn cell_of(&self, index: usize) -> (usize, usize) {
        (index / self.columns.max(1), index % self.columns.max(1))
    }

    /// Card `index`'s rectangle when row `offset` is the first shown. It may reach below the body;
    /// painting clips it there.
    pub(super) fn card_rect(&self, index: usize, offset: usize) -> Rect {
        let (row, column) = self.cell_of(index);
        let row = i64::try_from(row).unwrap_or(i64::MAX) - i64::try_from(offset).unwrap_or(0);
        let column = i64::try_from(column).unwrap_or(0);
        let x = i64::from(self.body.x) + column * self.column_pitch();
        let y = i64::from(self.body.y) + row * self.row_pitch();
        let clamp = |value: i64| i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX });
        Rect::new(clamp(x), clamp(y), self.card_width, self.card_height)
    }

    /// The cards on screen, from the top left, when row `offset` is the first shown: whole rows,
    /// and the top of the next row when the area has room left for it.
    pub(super) fn shown(&self, offset: usize) -> std::ops::Range<usize> {
        let pitch = u64::try_from(self.row_pitch()).unwrap_or(1).max(1);
        let reach = u64::from(self.body.height).div_ceil(pitch);
        let rows = usize::try_from(reach).unwrap_or(usize::MAX);
        let start = offset.saturating_mul(self.columns).min(self.count);
        let end = offset.saturating_add(rows).saturating_mul(self.columns).min(self.count);
        start..end
    }

    /// The card under cell `(x, y)` when row `offset` is the first shown; `None` in the gaps.
    pub(super) fn index_at(&self, x: i32, y: i32, offset: usize) -> Option<usize> {
        if !self.body.contains(x, y) {
            return None;
        }
        let dx = i64::from(x) - i64::from(self.body.x);
        let dy = i64::from(y) - i64::from(self.body.y);
        let (column_pitch, row_pitch) = (self.column_pitch().max(1), self.row_pitch().max(1));
        if dx % column_pitch >= i64::from(self.card_width) || dy % row_pitch >= i64::from(self.card_height) {
            return None;
        }
        let column = usize::try_from(dx / column_pitch).ok()?;
        let row = offset.checked_add(usize::try_from(dy / row_pitch).ok()?)?;
        let index = row.checked_mul(self.columns)?.checked_add(column)?;
        (column < self.columns && index < self.count).then_some(index)
    }

    /// The card a keyboard `step` reaches from `from` (the first or last card when none is
    /// chosen yet). Steps stop at the edges: nothing wraps to the next or previous row.
    pub(super) fn step(&self, step: Step, from: Option<usize>) -> Option<usize> {
        let last = self.count.checked_sub(1)?;
        let columns = self.columns.max(1);
        let Some(from) = from.map(|index| index.min(last)) else {
            return Some(if matches!(step, Step::Up | Step::PageUp | Step::End) { last } else { 0 });
        };
        let (row, column) = self.cell_of(from);
        let last_row = last / columns;
        let page = self.visible.max(1);
        // Moving between rows keeps the column; a shorter last row gives its last card instead.
        let in_row = |row: usize| (row * columns + column).min(last);
        Some(match step {
            Step::Left => from - usize::from(column > 0),
            Step::Right if column + 1 < columns => (from + 1).min(last),
            Step::Right => from,
            Step::Up => in_row(row.saturating_sub(1)),
            Step::Down => in_row((row + 1).min(last_row)),
            Step::PageUp => in_row(row.saturating_sub(page)),
            Step::PageDown => in_row((row + page).min(last_row)),
            Step::Home => 0,
            Step::End => last,
        })
    }

    /// The thumb geometry of the scrollbar for row `offset`.
    pub(super) fn metrics(&self, offset: usize) -> super::super::scrollbar::ScrollMetrics {
        super::super::scrollbar::ScrollMetrics { total: self.rows, visible: self.visible, offset }
    }

    /// The scrollbar's column, while rows overflow.
    pub(super) fn bar(&self) -> Option<Rect> {
        self.overflows.then(|| Rect::new(self.body.right(), self.body.y, 1, self.body.height))
    }

    /// The first row to show so card `index` is whole on screen, starting from `offset`.
    pub(super) fn reveal(&self, index: usize, offset: usize) -> usize {
        let (row, _) = self.cell_of(index);
        if row < offset {
            row
        } else if row >= offset + self.visible {
            row + 1 - self.visible
        } else {
            offset
        }
    }
}

/// A keyboard move between cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

/// Height of `rows` rows of cards with their gaps, for measuring.
pub(super) fn height_of(rows: usize, sizing: Sizing) -> u16 {
    let rows = i64::try_from(rows).unwrap_or(i64::MAX);
    let total = rows
        .saturating_mul(i64::from(sizing.height.max(1)) + i64::from(sizing.row_gap))
        .saturating_sub(i64::from(sizing.row_gap));
    clamp_u16(i32::try_from(total).unwrap_or(i32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPS: Sizing = Sizing { min_width: 24, max_width: 32, height: 3, column_gap: 2, row_gap: 1 };

    fn grid(width: u16, height: u16, count: usize) -> Layout {
        Layout::new(Rect::new(0, 0, width, height), count, APPS)
    }

    #[test]
    fn columns_follow_the_width() {
        assert_eq!(columns(96, APPS), (3, 30));
        assert_eq!(columns(140, APPS), (5, 26));
        assert_eq!(columns(76, APPS), (3, 24), "three cards of the least width and two gaps fill 76 cells");
        assert_eq!(columns(75, APPS), (2, 32), "one cell less leaves two cards, at most 32 wide");
        assert_eq!(columns(200, APPS), (7, 26));
    }

    #[test]
    fn a_narrow_area_is_one_column_as_wide_as_the_area() {
        assert_eq!(columns(30, APPS), (1, 30), "wider than the least but not two cards: one card");
        assert_eq!(columns(24, APPS), (1, 24));
        assert_eq!(columns(12, APPS), (1, 12), "narrower than a card: the card fills the area");
        assert_eq!(columns(0, APPS), (1, 0));
    }

    #[test]
    fn the_scrollbar_takes_a_column_only_when_rows_overflow() {
        let fits = grid(96, 20, 12);
        assert_eq!((fits.rows, fits.visible, fits.overflows, fits.body.width), (4, 5, false, 96));
        let scrolls = grid(96, 20, 100);
        assert!(scrolls.overflows);
        assert_eq!(scrolls.body.width, 95);
        assert_eq!(scrolls.bar(), Some(Rect::new(95, 0, 1, 20)));
    }

    #[test]
    fn keys_move_in_two_dimensions_and_stop_at_the_edges() {
        // Three columns, eight cards: the last row holds two.
        let layout = grid(96, 30, 8);
        assert_eq!(layout.step(Step::Right, Some(0)), Some(1));
        assert_eq!(layout.step(Step::Right, Some(2)), Some(2), "the end of a row does not wrap");
        assert_eq!(layout.step(Step::Left, Some(3)), Some(3), "nor does its start");
        assert_eq!(layout.step(Step::Right, Some(7)), Some(7), "the last card stops");
        assert_eq!(layout.step(Step::Down, Some(1)), Some(4));
        assert_eq!(layout.step(Step::Down, Some(5)), Some(7), "a short last row gives its last card");
        assert_eq!(layout.step(Step::Down, Some(7)), Some(7));
        assert_eq!(layout.step(Step::Up, Some(7)), Some(4));
        assert_eq!(layout.step(Step::Up, Some(1)), Some(1));
        assert_eq!(layout.step(Step::Home, Some(5)), Some(0));
        assert_eq!(layout.step(Step::End, Some(0)), Some(7));
        assert_eq!(layout.step(Step::Down, None), Some(0));
        assert_eq!(layout.step(Step::Up, None), Some(7));
        assert_eq!(grid(96, 30, 0).step(Step::Down, None), None);
    }

    #[test]
    fn pages_move_by_the_rows_that_fit() {
        // Rows of four cells: 12 rows fit three whole rows of cards.
        let layout = grid(96, 12, 30);
        assert_eq!(layout.visible, 3);
        assert_eq!(layout.step(Step::PageDown, Some(1)), Some(10));
        assert_eq!(layout.step(Step::PageDown, Some(28)), Some(28));
        assert_eq!(layout.step(Step::PageUp, Some(10)), Some(1));
        assert_eq!(layout.step(Step::PageUp, Some(4)), Some(1));
    }

    #[test]
    fn cells_map_to_cards_and_gaps_to_none() {
        let layout = grid(96, 20, 8);
        assert_eq!(layout.index_at(0, 0, 0), Some(0));
        assert_eq!(layout.index_at(29, 2, 0), Some(0));
        assert_eq!(layout.index_at(30, 0, 0), None, "the gap between columns");
        assert_eq!(layout.index_at(32, 0, 0), Some(1));
        assert_eq!(layout.index_at(0, 3, 0), None, "the gap between rows");
        assert_eq!(layout.index_at(0, 4, 0), Some(3));
        assert_eq!(layout.index_at(70, 4, 1), None, "past the last card");
        assert_eq!(layout.index_at(0, 0, 1), Some(3), "scrolled by a row");
    }

    #[test]
    fn revealing_scrolls_the_least() {
        let layout = grid(96, 12, 60);
        assert_eq!(layout.reveal(0, 0), 0);
        assert_eq!(layout.reveal(9, 0), 1, "row 3 comes in at the bottom");
        assert_eq!(layout.reveal(3, 5), 1, "row 1 comes in at the top");
        assert_eq!(layout.reveal(20, 5), 5, "a card on screen does not scroll");
    }

    #[test]
    fn shown_covers_whole_rows_and_the_top_of_the_next() {
        let layout = grid(96, 10, 60);
        assert_eq!(layout.visible, 2);
        assert_eq!(layout.shown(0), 0..9, "two whole rows and the top two rows of the third");
        assert_eq!(layout.shown(19), 57..60);
    }

    #[test]
    fn heights_add_gaps_between_rows_only() {
        assert_eq!(height_of(3, APPS), 11);
        assert_eq!(height_of(0, APPS), 0);
        assert_eq!(height_of(usize::MAX, APPS), u16::MAX);
    }
}
