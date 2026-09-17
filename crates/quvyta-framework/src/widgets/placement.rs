//! Placing a layer next to the thing it belongs to: dropdowns, popovers, menus, tooltips.

use crate::geometry::{Rect, Size, clamp_u16};
use crate::motion::steps;

/// The side of its anchor a layer prefers. When the layer does not fit there it flips to the
/// opposite side, and it is always kept on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// Under the anchor, left edges aligned.
    #[default]
    Below,
    /// Over the anchor, left edges aligned.
    Above,
    /// To the right of the anchor, top edges aligned.
    Right,
    /// To the left of the anchor, top edges aligned.
    Left,
}

impl Placement {
    /// Every placement.
    pub const ALL: [Self; 4] = [Self::Below, Self::Above, Self::Right, Self::Left];

    /// A short name, e.g. for settings screens.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Below => "below",
            Self::Above => "above",
            Self::Right => "right",
            Self::Left => "left",
        }
    }

    /// The other side of the anchor.
    pub(crate) fn opposite(self) -> Self {
        match self {
            Self::Below => Self::Above,
            Self::Above => Self::Below,
            Self::Right => Self::Left,
            Self::Left => Self::Right,
        }
    }

    fn vertical(self) -> bool {
        matches!(self, Self::Below | Self::Above)
    }
}

/// Where a layer of `size` goes next to `anchor` on `screen`, and the side it ended up on.
///
/// The preferred side is used when the layer fits there, otherwise the opposite side, otherwise
/// whichever of the two has more room. A side placement (right or left) that fits on neither
/// side falls back to below or above. Along the other axis the layer is aligned with the anchor
/// and slid back onto the screen.
pub(crate) fn place(anchor: Rect, size: Size, screen: Rect, preferred: Placement) -> (Rect, Placement) {
    let size = size.min(screen.size());
    let (width, height) = (i32::from(size.width), i32::from(size.height));
    let room = |side: Placement| match side {
        Placement::Below => screen.bottom() - anchor.bottom(),
        Placement::Above => anchor.y - screen.y,
        Placement::Right => screen.right() - anchor.right(),
        Placement::Left => anchor.x - screen.x,
    };
    let needed = |side: Placement| if side.vertical() { height } else { width };
    let side = if room(preferred) >= needed(preferred) {
        preferred
    } else if room(preferred.opposite()) >= needed(preferred) {
        preferred.opposite()
    } else if !preferred.vertical() {
        return place(anchor, size, screen, Placement::Below);
    } else if room(preferred.opposite()) > room(preferred) {
        preferred.opposite()
    } else {
        preferred
    };
    let clamp_x = |x: i32| x.min(screen.right() - width).max(screen.x);
    let clamp_y = |y: i32| y.min(screen.bottom() - height).max(screen.y);
    let (x, y) = match side {
        Placement::Below => (clamp_x(anchor.x), clamp_y(anchor.bottom())),
        Placement::Above => (clamp_x(anchor.x), clamp_y(anchor.y - height)),
        Placement::Right => (clamp_x(anchor.right()), clamp_y(anchor.y)),
        Placement::Left => (clamp_x(anchor.x - width), clamp_y(anchor.y)),
    };
    (Rect::new(x, y, clamp_u16(width), clamp_u16(height)), side)
}

/// The part shown of a layer at `full`, `progress` of the way (0 to 1) through unfolding from
/// its anchor on `side`: whole rows grow away from an anchor above or below it, whole columns
/// from an anchor beside it. At least one row or column shows.
pub(crate) fn unfold(full: Rect, side: Placement, progress: f32) -> Rect {
    match side {
        Placement::Below => Rect::new(full.x, full.y, full.width, steps(progress, full.height).max(1)),
        Placement::Above => {
            let rows = steps(progress, full.height).max(1);
            Rect::new(full.x, full.bottom() - i32::from(rows), full.width, rows)
        }
        Placement::Right => Rect::new(full.x, full.y, steps(progress, full.width).max(1), full.height),
        Placement::Left => {
            let columns = steps(progress, full.width).max(1);
            Rect::new(full.right() - i32::from(columns), full.y, columns, full.height)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect::new(0, 0, 40, 20);

    #[test]
    fn uses_the_preferred_side_when_it_fits() {
        let anchor = Rect::new(5, 3, 10, 1);
        assert_eq!(
            place(anchor, Size::new(12, 4), SCREEN, Placement::Below),
            (Rect::new(5, 4, 12, 4), Placement::Below)
        );
        assert_eq!(
            place(anchor, Size::new(12, 3), SCREEN, Placement::Above),
            (Rect::new(5, 0, 12, 3), Placement::Above)
        );
        assert_eq!(
            place(anchor, Size::new(8, 3), SCREEN, Placement::Right),
            (Rect::new(15, 3, 8, 3), Placement::Right)
        );
    }

    #[test]
    fn flips_when_there_is_no_room() {
        let bottom = Rect::new(5, 18, 10, 1);
        assert_eq!(place(bottom, Size::new(12, 4), SCREEN, Placement::Below).1, Placement::Above);
        let top = Rect::new(5, 1, 10, 1);
        assert_eq!(place(top, Size::new(12, 4), SCREEN, Placement::Above).1, Placement::Below);
        let right_edge = Rect::new(34, 5, 4, 1);
        assert_eq!(
            place(right_edge, Size::new(8, 3), SCREEN, Placement::Right),
            (Rect::new(26, 5, 8, 3), Placement::Left)
        );
    }

    #[test]
    fn unfolds_away_from_the_anchor_in_whole_cells() {
        let full = Rect::new(4, 6, 10, 4);
        assert_eq!(unfold(full, Placement::Below, 0.5), Rect::new(4, 6, 10, 2));
        assert_eq!(unfold(full, Placement::Above, 0.5), Rect::new(4, 8, 10, 2));
        assert_eq!(unfold(full, Placement::Right, 0.3), Rect::new(4, 6, 3, 4));
        assert_eq!(unfold(full, Placement::Left, 0.3), Rect::new(11, 6, 3, 4));
        assert_eq!(unfold(full, Placement::Below, 0.0), Rect::new(4, 6, 10, 1), "one row shows at once");
        assert_eq!(unfold(full, Placement::Left, 1.0), full);
    }

    #[test]
    fn stays_on_screen() {
        let corner = Rect::new(36, 10, 4, 1);
        let (rect, _) = place(corner, Size::new(12, 4), SCREEN, Placement::Below);
        assert_eq!(rect, Rect::new(28, 11, 12, 4));
        let (tall, side) = place(Rect::new(2, 8, 4, 1), Size::new(6, 30), SCREEN, Placement::Below);
        assert_eq!((tall.y, tall.height, side), (0, 20, Placement::Below));
        let wide = place(Rect::new(18, 8, 4, 1), Size::new(30, 2), SCREEN, Placement::Right);
        assert_eq!(wide.1, Placement::Below);
    }
}
