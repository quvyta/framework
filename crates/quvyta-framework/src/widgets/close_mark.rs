//! The close mark every closable surface shares: a space, the close glyph and a space. The whole
//! three cells are the target, and under the pointer they light up together, so the mark reads
//! as a small button rather than a lone glyph.

use crate::geometry::Rect;
use crate::style::CellStyle;
use crate::theme::State;
use crate::widget::PaintCx;

/// Cells a close mark takes.
pub(crate) const WIDTH: u16 = 3;

/// The cells of a close mark whose left edge is at `x` on row `y`.
pub(crate) fn rect(x: i32, y: i32) -> Rect {
    Rect::new(x, y, WIDTH, 1)
}

/// Paints the close mark at `x`, `y`. `raised` says the surface carrying the mark is hovered, open
/// or otherwise raised, which lifts the mark from a whisper; the pointer on the mark lights its
/// three cells. Returns the cells, which are the press target.
///
/// Style keys: `close-mark` (`fg`, `bg`, `bold`) with `active` while its surface is raised and
/// `hover` under the pointer.
pub(crate) fn paint(cx: &mut PaintCx<'_>, x: i32, y: i32, raised: bool) -> Rect {
    let area = rect(x, y);
    let mut states = Vec::new();
    if raised {
        states.push(State::Active);
    }
    if cx.pointer_anywhere().is_some_and(|(px, py)| area.contains(px, py)) {
        states.push(State::Hover);
    }
    let style = cx.style("close-mark", None, &states).text();
    if let Some(bg) = style.bg {
        cx.fill(area, bg);
    }
    let glyph = cx.env().icons().glyph("close").into_owned();
    cx.text(x + 1, y, &glyph, CellStyle { bg: None, ..style }, 1);
    area
}
