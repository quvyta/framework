//! The F12 debug layer: clickable areas, focus order and frame statistics.

use super::engine::Stats;
use crate::geometry::Rect;
use crate::style::CellStyle;
use crate::text;
use crate::widget::PaintCx;

/// Paints the debug layer over the finished frame.
pub(crate) fn paint(cx: &mut PaintCx<'_>, stats: &Stats) {
    let accent = cx.color("accent");
    let ink = cx.color("ink");
    let overlay = cx.color("overlay");
    let dim = cx.color("dim");
    let text_color = cx.color("text");

    let hits = cx.frame.hits.clone();
    let hovered = cx.interaction.hovered;
    for (rect, id) in &hits {
        let tint = if Some(*id) == hovered { cx.color("active") } else { cx.color("raised") };
        cx.fill(Rect::new(rect.x, rect.y, rect.width, 1), tint);
    }

    let order = cx.frame.focusable.clone();
    for (index, id) in order.iter().enumerate() {
        let Some(rect) = cx.frame.rects.get(id).copied() else {
            continue;
        };
        let label = format!(" {} ", index + 1);
        let style = CellStyle::fg(ink).on(accent).with_bold(true);
        cx.text(rect.right() - i32::from(text::width(&label)), rect.y, &label, style, 6);
    }

    let lines = [
        format!("frame {}", stats.frames),
        format!("paint {:.1} ms", stats.paint_time.as_secs_f64() * 1000.0),
        format!("hit areas {}", hits.len()),
        format!("focus order {}", order.len()),
        format!("focused {}", cx.interaction.focused.map_or_else(|| "none".to_owned(), |id| format!("{id:?}"))),
    ];
    let width = lines.iter().map(|line| text::width(line)).max().unwrap_or(0) + 4;
    let height = u16::try_from(lines.len()).unwrap_or(0) + 2;
    let screen = cx.clip();
    let panel =
        Rect::new(screen.right() - i32::from(width) - 2, screen.bottom() - i32::from(height) - 2, width, height);
    cx.clear(panel, overlay);
    cx.pillar(panel.x, panel.y + 1, accent);
    for (row, line) in lines.iter().enumerate() {
        let y = panel.y + 1 + i32::try_from(row).unwrap_or(0);
        let color = if row == 0 { text_color } else { dim };
        cx.text(panel.x + 2, y, line, CellStyle::fg(color).with_bold(row == 0), width - 2);
    }
}
