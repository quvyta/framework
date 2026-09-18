//! The readout row of a timeline: the block being read, in words.

use crate::geometry::Rect;
use crate::style::CellStyle;
use crate::text;
use crate::widget::PaintCx;

use super::super::axis::{DAY, clock_label};
use super::{TimeBlock, Timeline};

impl<Msg: 'static> Timeline<Msg> {
    /// Writes the block being read: its name, then its times and its length, each dropped when
    /// the row is too narrow, the name cut with `…` last of all. With nothing to read, an empty
    /// day says so and a day with blocks leaves the row blank.
    pub(super) fn paint_readout(&self, cx: &mut PaintCx<'_>, row: Rect, read: Option<&TimeBlock>) {
        let style = cx.style("timeline-readout", None, &[]);
        let (name_color, detail_color) = if self.disabled {
            (cx.color("muted"), cx.color("muted"))
        } else {
            (
                style.color("fg").unwrap_or_else(|| cx.color("text")),
                style.color("detail").unwrap_or_else(|| cx.color("dim")),
            )
        };
        let Some(block) = read else {
            if self.blocks.is_empty() {
                let empty = crate::t!("quvyta.timeline.empty");
                let shown = text::truncate(&empty, row.width);
                cx.text(row.x, row.y, &shown, CellStyle::fg(cx.color("muted")), row.width);
            }
            return;
        };
        let open = self.open_words(block);
        let start = block.start.seconds_since_midnight();
        let end = block.end.seconds_since_midnight();
        let times = crate::t!("quvyta.time.span", from = clock_label(start, false), to = clock_label(end, false));
        let length = duration((end + DAY - start) % DAY);
        let name_width = text::width(&block.label);
        let details = match open {
            Some(open) => vec![format!("{times}  {open}  {length}"), format!("{times}  {open}"), open],
            None => vec![format!("{times}  {length}"), times],
        };
        let detail =
            details.iter().find(|detail| name_width.saturating_add(2).saturating_add(text::width(detail)) <= row.width);
        let name = text::truncate(&block.label, row.width);
        let written = cx.text(row.x, row.y, &name, CellStyle::fg(name_color), row.width);
        if let Some(detail) = detail {
            let x = row.x + i32::from(written) + 2;
            cx.text(x, row.y, detail, CellStyle::fg(detail_color), text::width(detail));
        }
    }

    /// What the open edges of `block` mean, in the active language: where it came from, and
    /// whether it goes on into the next day or is still running. `None` for a closed block.
    fn open_words(&self, block: &TimeBlock) -> Option<String> {
        let (start, end) = self.extent(block);
        let from = block.open_start.then(|| {
            if start == 0 {
                crate::t!("quvyta.timeline.from-previous-day")
            } else {
                crate::t!("quvyta.timeline.from-earlier")
            }
        });
        let to = block.open_end.then(|| {
            if end == DAY {
                crate::t!("quvyta.timeline.continues-next-day")
            } else {
                crate::t!("quvyta.timeline.running")
            }
        });
        match (from, to) {
            (Some(from), Some(to)) => Some(crate::t!("quvyta.timeline.open-both", start = from, end = to)),
            (from, to) => from.or(to),
        }
    }
}

/// `seconds` as a length in the active language: hours and minutes, whole hours, minutes, or
/// seconds for less than a minute.
fn duration(seconds: u32) -> String {
    let hours = seconds / 3_600;
    let minutes = seconds % 3_600 / 60;
    match (hours, minutes) {
        (0, 0) => crate::t!("quvyta.time.seconds", seconds = seconds),
        (0, minutes) => crate::t!("quvyta.time.minutes", minutes = minutes),
        (hours, 0) => crate::t!("quvyta.time.hours", hours = hours),
        (hours, minutes) => crate::t!("quvyta.time.hours-minutes", hours = hours, minutes = minutes),
    }
}
