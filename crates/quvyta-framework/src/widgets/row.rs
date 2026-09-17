//! The anatomy of every selectable row in a list-like widget, built once so hover, selection and
//! slide look and behave the same everywhere.
//!
//! A row has three parts after its pillar:
//!
//! 1. **Fixed leading marks**: indentation, a multi-select check mark, an expand chevron. They
//!    never move, so a mark stays exactly where the pointer clicks it.
//! 2. **The sliding part**: icon and label. When the row is hovered or selected and the theme
//!    allows sliding, only this part moves one cell right.
//! 3. **Fixed trailing content**: detail, badge, shortcut, value, status, close mark. The caller
//!    draws it anchored to the right edge; the row only keeps its cells free.
//!
//! The label column always keeps one spare cell for the slide, so a label is cut with `…` at the
//! same place whether its row rests or slides.
//!
//! [`List`](super::List), [`Menu`](super::Menu), [`Tree`](super::Tree),
//! [`Accordion`](super::Accordion), [`WidgetDock`](super::WidgetDock) and
//! [`TabRail`](super::TabRail) draw their rows with it; [`Table`](super::Table) uses its fixed
//! marks and its spare-cell rule for the first cell.

use crate::color::Rgb;
use crate::geometry::Rect;
use crate::style::{CellStyle, WidgetStyle};
use crate::text;
use crate::widget::PaintCx;

/// Cells before the fixed marks: the pillar and a space.
pub(crate) const LEAD: u16 = 2;

/// A glyph drawn in a row with its style; it takes its width plus one cell of air.
pub(crate) type Mark = (String, CellStyle);

/// What one row shows, part by part.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Parts<'a> {
    /// Cells of indentation after [`LEAD`], e.g. a tree level; they never move.
    pub(crate) indent: u16,
    /// Leading marks that never move: a check mark, a chevron.
    pub(crate) fixed: &'a [Mark],
    /// Marks that slide with the label: icons.
    pub(crate) sliding: &'a [Mark],
    /// The label, drawn in the row style's text colour.
    pub(crate) label: &'a str,
    /// Cells at the right the caller fills with anchored content.
    pub(crate) trailing: u16,
}

/// The mark of icon `key` in theme colour `color`, or in `fallback` (usually the row's text colour)
/// when the icon has none.
pub(crate) fn icon(cx: &PaintCx<'_>, key: &str, color: Option<&str>, fallback: Option<Rgb>) -> Mark {
    let fg = color.map_or(fallback, |token| Some(cx.color(token)));
    (cx.env().icons().glyph(key).into_owned(), CellStyle { fg, ..CellStyle::default() })
}

/// The fixed check mark of a multi-select row: the accent box when `checked`, a faint one when not.
pub(crate) fn check(cx: &PaintCx<'_>, checked: bool) -> Mark {
    let (key, color) = if checked { ("select-on", cx.color("accent")) } else { ("select-off", cx.color("muted")) };
    (cx.env().icons().glyph(key).into_owned(), CellStyle::fg(color))
}

/// Paints `text` anchored to the right end of `rect`, keeping the row's last cell free: the detail,
/// badge or count that never slides.
pub(crate) fn paint_trailing(cx: &mut PaintCx<'_>, rect: Rect, text: &str, style: CellStyle) {
    let width = text::width(text);
    cx.text(rect.right() - 1 - i32::from(width), rect.y, text, style, width);
}

/// Draws `marks` from `x` on row `y` within `budget` cells and returns the cells used.
pub(crate) fn paint_marks(cx: &mut PaintCx<'_>, x: i32, y: i32, marks: &[Mark], budget: u16) -> u16 {
    let mut used = 0u16;
    for (glyph, style) in marks {
        let left = budget.saturating_sub(used);
        if left == 0 {
            break;
        }
        cx.text(x + i32::from(used), y, glyph, *style, left);
        used = used.saturating_add(text::width(glyph).saturating_add(1)).min(budget);
    }
    used
}

/// Paints `style`'s surface and pillar over `rect`, then the row's `parts`. When `slide` is set
/// only the sliding part moves one cell right.
pub(crate) fn paint_parts(cx: &mut PaintCx<'_>, rect: Rect, style: &WidgetStyle, slide: bool, parts: &Parts<'_>) {
    let text_style = style.text();
    if let Some(bg) = text_style.bg {
        cx.fill(rect, bg);
    }
    if let Some(pillar) = style.color("pillar") {
        cx.pillar(rect.x, rect.y, pillar);
    }
    // Pillar, indentation, fixed marks, trailing content, the spare slide cell and the margin.
    // Saturating: a deep indent or a trailing text can be wider than any screen.
    let reserved = super::cells::sum([LEAD, parts.indent, parts.trailing, 1, 1]);
    let mut budget = rect.width.saturating_sub(reserved);
    let fixed_x = rect.x + i32::from(LEAD) + i32::from(parts.indent);
    let used = paint_marks(cx, fixed_x, rect.y, parts.fixed, budget);
    budget = budget.saturating_sub(used);
    let mut x = fixed_x + i32::from(used) + i32::from(slide);
    let used = paint_marks(cx, x, rect.y, parts.sliding, budget);
    x += i32::from(used);
    budget = budget.saturating_sub(used);
    let shown = text::truncate(parts.label, budget).into_owned();
    cx.text(x, rect.y, &shown, CellStyle { bg: None, ..text_style }, budget);
}

/// Paints a row with only sliding `marks` (icons) and `label`: the plain row of lists, menus and
/// tab rails.
pub(crate) fn paint(
    cx: &mut PaintCx<'_>,
    rect: Rect,
    style: &WidgetStyle,
    slide: bool,
    marks: &[Mark],
    label: &str,
    trailing: u16,
) {
    paint_parts(cx, rect, style, slide, &Parts { sliding: marks, label, trailing, ..Parts::default() });
}
