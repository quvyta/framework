//! Partial block glyphs shared by the charts and the progress bar: bars measured in eighths of a
//! cell.

use crate::color::Rgb;
use crate::geometry::Rect;
use crate::icons::GlyphMode;
use crate::style::CellStyle;
use crate::widget::PaintCx;

/// Blocks filled from the left, one to seven eighths.
const LEFT: [&str; 7] = ["▏", "▎", "▍", "▌", "▋", "▊", "▉"];

/// Blocks filled from the bottom, one to seven eighths.
const LOWER: [&str; 7] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇"];

/// `fraction` of `cells` in whole eighths, rounded.
pub(crate) fn eighths(fraction: f32, cells: u16) -> u32 {
    // Cell counts are small, so the product fits f32 and the rounded result fits u32.
    (fraction.clamp(0.0, 1.0) * f32::from(cells) * 8.0).round() as u32
}

/// `fraction` of `steps`, rounded to a whole step.
pub(crate) fn scaled(fraction: f32, steps: u32) -> u32 {
    // Step counts are small (a few cells times eight), so they fit f32 exactly.
    (fraction.clamp(0.0, 1.0) * steps as f32).round() as u32
}

/// The block `partial` eighths (1 to 7) filled from the left.
pub(crate) fn left_block(partial: usize) -> &'static str {
    LEFT[partial.clamp(1, LEFT.len()) - 1]
}

/// `eighths` as drawn in the current glyph mode: ASCII has no partial glyphs, so it rounds to
/// whole cells.
fn for_mode(cx: &PaintCx<'_>, eighths: u32) -> u32 {
    if cx.env().glyph_mode() == GlyphMode::Ascii { (eighths + 4) / 8 * 8 } else { eighths }
}

/// Draws a horizontal bar of `eighths` from the left of `row` (one row high) in `color`. ASCII
/// mode rounds to whole cells, since it has no partial glyphs.
pub(crate) fn horizontal(cx: &mut PaintCx<'_>, row: Rect, eighths: u32, color: Rgb) {
    let eighths = for_mode(cx, eighths);
    let full = u16::try_from(eighths / 8).unwrap_or(u16::MAX).min(row.width);
    cx.fill(Rect::new(row.x, row.y, full, row.height), color);
    let partial = usize::try_from(eighths % 8).unwrap_or(0);
    if partial > 0 && full < row.width {
        for y in row.y..row.bottom() {
            cx.text(row.x + i32::from(full), y, left_block(partial), CellStyle::fg(color), 1);
        }
    }
}

/// Draws a vertical bar of `eighths` rising from the bottom of `column` (one column wide) in
/// `color`. ASCII mode rounds to whole cells.
pub(crate) fn vertical(cx: &mut PaintCx<'_>, column: Rect, eighths: u32, color: Rgb) {
    let eighths = for_mode(cx, eighths);
    let full = u16::try_from(eighths / 8).unwrap_or(u16::MAX).min(column.height);
    let bottom = column.bottom();
    cx.fill(Rect::new(column.x, bottom - i32::from(full), column.width, full), color);
    let partial = usize::try_from(eighths % 8).unwrap_or(0);
    if partial > 0 && full < column.height {
        let y = bottom - i32::from(full) - 1;
        for x in column.x..column.right() {
            cx.text(x, y, LOWER[partial - 1], CellStyle::fg(color), 1);
        }
    }
}
