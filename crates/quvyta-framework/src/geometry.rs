//! Cell geometry: rectangles, sizes and padding.
//!
//! Rectangles use signed coordinates so content scrolled above or left of its viewport can be
//! laid out normally and clipped when drawn.

/// A width and height in terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Size {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

impl Size {
    /// Creates a size.
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    /// The component-wise minimum of two sizes.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.width.min(other.width), self.height.min(other.height))
    }
}

/// Space kept free inside an area, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Padding {
    /// Rows above.
    pub top: u16,
    /// Columns to the right.
    pub right: u16,
    /// Rows below.
    pub bottom: u16,
    /// Columns to the left.
    pub left: u16,
}

impl Padding {
    /// The same padding on every side.
    #[must_use]
    pub const fn all(cells: u16) -> Self {
        Self { top: cells, right: cells, bottom: cells, left: cells }
    }

    /// `vertical` rows above and below, `horizontal` columns left and right.
    #[must_use]
    pub const fn symmetric(vertical: u16, horizontal: u16) -> Self {
        Self { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
    }

    /// Total columns taken.
    #[must_use]
    pub fn horizontal(self) -> u16 {
        self.left.saturating_add(self.right)
    }

    /// Total rows taken.
    #[must_use]
    pub fn vertical(self) -> u16 {
        self.top.saturating_add(self.bottom)
    }
}

/// A rectangle of cells. `x` and `y` may be negative or beyond the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    /// Left column.
    pub x: i32,
    /// Top row.
    pub y: i32,
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

impl Rect {
    /// Creates a rectangle.
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    /// The column just past the right edge, saturating at `i32::MAX`.
    #[must_use]
    pub fn right(self) -> i32 {
        self.x.saturating_add(i32::from(self.width))
    }

    /// The row just past the bottom edge, saturating at `i32::MAX`.
    #[must_use]
    pub fn bottom(self) -> i32 {
        self.y.saturating_add(i32::from(self.height))
    }

    /// The size of the rectangle.
    #[must_use]
    pub fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Whether the rectangle covers no cell.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Whether the cell at `(x, y)` lies inside.
    #[must_use]
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// The overlapping part of two rectangles; empty (at `self`'s origin) when they do not overlap.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= x || bottom <= y {
            return Self::new(self.x, self.y, 0, 0);
        }
        Self::new(x, y, clamp_u16(right - x), clamp_u16(bottom - y))
    }

    /// The rectangle shrunk by `padding`, never below zero size.
    #[must_use]
    pub fn inset(self, padding: Padding) -> Self {
        Self::new(
            self.x.saturating_add(i32::from(padding.left)),
            self.y.saturating_add(i32::from(padding.top)),
            self.width.saturating_sub(padding.horizontal()),
            self.height.saturating_sub(padding.vertical()),
        )
    }

    /// One row of the rectangle, relative to its top.
    #[must_use]
    pub fn row(self, offset: u16) -> Self {
        Self::new(self.x, self.y.saturating_add(i32::from(offset)), self.width, u16::from(offset < self.height))
    }

    /// A rectangle of `size` centred inside this one, clamped to fit.
    #[must_use]
    pub fn centered(self, size: Size) -> Self {
        let size = size.min(self.size());
        Self::new(
            self.x.saturating_add(i32::from((self.width - size.width) / 2)),
            self.y.saturating_add(i32::from((self.height - size.height) / 2)),
            size.width,
            size.height,
        )
    }
}

/// Converts a non-negative `i32` cell count to `u16`, saturating.
pub(crate) fn clamp_u16(value: i32) -> u16 {
    u16::try_from(value.max(0)).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_clips_and_handles_disjoint() {
        let a = Rect::new(0, 0, 10, 5);
        assert_eq!(a.intersect(Rect::new(5, -2, 10, 4)), Rect::new(5, 0, 5, 2));
        assert!(a.intersect(Rect::new(20, 20, 3, 3)).is_empty());
    }

    #[test]
    fn inset_never_underflows() {
        let r = Rect::new(2, 3, 4, 2).inset(Padding::symmetric(1, 3));
        assert_eq!(r, Rect::new(5, 4, 0, 0));
        assert_eq!(Padding::all(2).horizontal(), 4);
    }

    #[test]
    fn contains_uses_half_open_edges() {
        let r = Rect::new(-1, -1, 2, 2);
        assert!(r.contains(-1, -1) && r.contains(0, 0));
        assert!(!r.contains(1, 0));
    }

    #[test]
    fn edges_saturate_far_from_the_origin() {
        let far = Rect::new(i32::MAX - 1, i32::MAX - 1, 10, 10);
        assert_eq!((far.right(), far.bottom()), (i32::MAX, i32::MAX));
        assert_eq!(far.inset(Padding::all(4)), Rect::new(i32::MAX, i32::MAX, 2, 2));
        assert_eq!(far.row(3).y, i32::MAX);
        assert!(far.contains(i32::MAX - 1, i32::MAX - 1));
        assert_eq!(far.centered(Size::new(4, 4)).x, i32::MAX);
    }

    #[test]
    fn centered_and_rows() {
        let r = Rect::new(0, 0, 10, 6);
        assert_eq!(r.centered(Size::new(4, 2)), Rect::new(3, 2, 4, 2));
        assert_eq!(r.centered(Size::new(40, 2)), Rect::new(0, 2, 10, 2));
        assert_eq!(r.row(2), Rect::new(0, 2, 10, 1));
        assert_eq!(r.row(9).height, 0);
    }
}
