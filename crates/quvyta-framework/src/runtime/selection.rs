//! Selecting text on screen with the mouse.
//!
//! Rules, in order:
//!
//! 1. Nothing is selectable unless asked. A widget whose text is content to copy marks its area
//!    as a selection region ([`PaintCx::selectable`](crate::widget::PaintCx::selectable)):
//!    `CodeView`, `Markdown` and `Terminal` do by themselves, any node does with
//!    [`NodeMut::selectable`](crate::widget::NodeMut::selectable). Menus, titles, controls and
//!    the empty canvas never start a selection.
//! 2. A left press goes to widgets first. A widget that uses the press (a button, a text field,
//!    a list, a scrollbar) keeps the mouse; no selection starts.
//! 3. A press nobody used starts a selection when it lands in a region and not in an area kept
//!    out ([`NodeMut::selectable`](crate::widget::NodeMut::selectable) with `false` or
//!    [`PaintCx::unselectable`](crate::widget::PaintCx::unselectable)). Regions beneath a modal
//!    layer are out of reach.
//! 4. The selection stays inside its region, the innermost one under the press; a scrolling
//!    region is narrowed to its content, without the scrollbar.
//! 5. Dragging extends it by cells; a double press selects a word, a triple press a line.
//! 6. Releasing only keeps the selection; nothing is copied. The `copy` key copies it clean. A
//!    right press on it opens a menu with Copy (clean) and Raw copy. A clean copy leaves out
//!    decoration cells ([`PaintCx::decoration`](crate::widget::PaintCx::decoration), such as
//!    pillars and scrollbars) and the spaces that pad the end of each line, and keeps the spaces
//!    between words and the line breaks; a raw copy keeps every cell as shown. Any other key or
//!    a press elsewhere clears the selection.
//! 7. The selection follows the widget it started on, so it stays on the same text while that
//!    content scrolls, and disappears when the widget does.

use std::time::Duration;

use ratatui_core::buffer::{Buffer, Cell};

use crate::geometry::{Rect, clamp_u16};
use crate::theme::State;
use crate::widget::{Frame, PaintCx, WidgetId};

/// Presses closer together than this on the same cell count as double and triple presses.
pub(crate) const MULTI_PRESS: Duration = Duration::from_millis(400);

/// How much a selection covers around the pressed cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unit {
    Cell,
    Word,
    Line,
}

/// How selected text is copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyKind {
    /// Without decoration cells and without the spaces that pad each line's end.
    Clean,
    /// Every selected cell exactly as shown.
    Raw,
}

/// A press remembered to recognise double and triple presses.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Press {
    at: (i32, i32),
    time: Duration,
    count: u8,
}

impl Press {
    /// The press at `at`, counting it as the next of a series when it follows `last` closely.
    pub(crate) fn next(last: Option<Self>, at: (i32, i32), time: Duration) -> Self {
        let count = match last {
            Some(last) if last.at == at && time.saturating_sub(last.time) < MULTI_PRESS => last.count % 3 + 1,
            _ => 1,
        };
        Self { at, time, count }
    }
}

/// A selection in screen cells.
#[derive(Debug, Clone)]
pub(crate) struct Selection {
    region: Rect,
    /// The selectable widget the selection belongs to.
    owner: WidgetId,
    /// The owner's child under the press, e.g. a scroll view's content without its scrollbar.
    inner: Option<WidgetId>,
    tracked: Option<(WidgetId, Rect)>,
    anchor: (i32, i32),
    head: (i32, i32),
    unit: Unit,
    /// Whether the pointer is still down.
    pub(crate) dragging: bool,
    /// Whether anything is selected yet; a plain press selects nothing until it moves.
    visible: bool,
    /// Copy the text at the next render, when the buffer is at hand.
    pub(crate) copy_pending: Option<CopyKind>,
    copied_at: Option<Duration>,
    /// The selected cells as painted last, one rect per row.
    painted: Vec<Rect>,
}

impl Selection {
    /// Starts a selection for a press nobody used, or returns `None` where selecting is not
    /// allowed.
    pub(crate) fn begin(frame: &Frame, press: Press, hit: Option<WidgetId>) -> Option<Self> {
        let (x, y) = press.at;
        if frame.unselectable.iter().any(|rect| rect.contains(x, y)) {
            return None;
        }
        // The innermost reachable region under the press that no other widget covers: the widget
        // hit lies inside the region, or the region inside it.
        let owner = frame
            .selectable
            .iter()
            .rev()
            .filter(|(rect, id)| rect.contains(x, y) && frame.reachable(*id))
            .find(|(_, id)| hit.is_none_or(|target| frame.is_within(target, *id) || frame.is_within(*id, target)))
            .map(|(_, id)| *id)?;
        // A focusable region such as a scroll view narrows to its child under the press.
        let inner = if frame.focusable.contains(&owner) {
            frame
                .parents
                .iter()
                .find(|(child, parent)| **parent == owner && frame.rects.get(child).is_some_and(|r| r.contains(x, y)))
                .map(|(child, _)| *child)
        } else {
            None
        };
        let region = Self::region_in(frame, owner, inner)?;
        // The smallest widget under the press moves with its content, e.g. text in a scroll view.
        let tracked = frame
            .rects
            .iter()
            .filter(|(id, rect)| rect.contains(x, y) && frame.is_within(**id, owner))
            .min_by_key(|(_, rect)| u32::from(rect.width) * u32::from(rect.height))
            .map(|(id, rect)| (*id, *rect));
        let unit = match press.count {
            1 => Unit::Cell,
            2 => Unit::Word,
            _ => Unit::Line,
        };
        Some(Self {
            region,
            owner,
            inner,
            tracked,
            anchor: (x, y),
            head: (x, y),
            unit,
            dragging: true,
            visible: unit != Unit::Cell,
            copy_pending: None,
            copied_at: None,
            painted: Vec::new(),
        })
    }

    /// The area a selection may cover: the visible part of the owner's region, narrowed to its
    /// child under the press.
    fn region_in(frame: &Frame, owner: WidgetId, inner: Option<WidgetId>) -> Option<Rect> {
        let rect = frame.selectable.iter().rev().find(|(_, id)| *id == owner).map(|(rect, _)| *rect)?;
        let region = inner.and_then(|inner| frame.rects.get(&inner)).map_or(rect, |child| rect.intersect(*child));
        (!region.is_empty()).then_some(region)
    }

    /// Extends the selection to the pointer, kept inside the region.
    pub(crate) fn drag_to(&mut self, x: i32, y: i32) {
        let region = self.region;
        let head = (x.clamp(region.x, region.right() - 1), y.clamp(region.y, region.bottom() - 1));
        if head != self.anchor {
            self.visible = true;
        }
        self.head = head;
    }

    /// Ends the drag and keeps what is selected; nothing is copied. Returns `false` when nothing
    /// was selected and the selection should go.
    pub(crate) fn release(&mut self) -> bool {
        self.dragging = false;
        self.visible
    }

    /// Whether the cell is selected, as painted last.
    pub(crate) fn contains(&self, x: i32, y: i32) -> bool {
        self.visible && self.painted.iter().any(|row| row.contains(x, y))
    }

    /// Moves with the tracked widget and region after a new layout. Returns `false` when they
    /// are gone.
    pub(crate) fn follow(&mut self, frame: &Frame) -> bool {
        match Self::region_in(frame, self.owner, self.inner) {
            Some(region) => self.region = region,
            None => return false,
        }
        if let Some((id, old)) = self.tracked {
            let Some(new) = frame.rects.get(&id).copied() else {
                return false;
            };
            let (dx, dy) = (new.x - old.x, new.y - old.y);
            self.anchor = (self.anchor.0 + dx, self.anchor.1 + dy);
            self.head = (self.head.0 + dx, self.head.1 + dy);
            self.tracked = Some((id, new));
        }
        true
    }

    /// First and last selected cells, in reading order, expanded to words or lines.
    fn span(&self, buf: &Buffer) -> ((i32, i32), (i32, i32)) {
        let (mut start, mut end) = if (self.anchor.1, self.anchor.0) <= (self.head.1, self.head.0) {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        };
        match self.unit {
            Unit::Cell => {}
            Unit::Line => {
                start.0 = self.region.x;
                end.0 = self.region.right() - 1;
            }
            Unit::Word => {
                if is_word(symbol(buf, start.0, start.1)) {
                    while start.0 > self.region.x && is_word(symbol(buf, start.0 - 1, start.1)) {
                        start.0 -= 1;
                    }
                }
                if is_word(symbol(buf, end.0, end.1)) {
                    while end.0 + 1 < self.region.right() && is_word(symbol(buf, end.0 + 1, end.1)) {
                        end.0 += 1;
                    }
                }
            }
        }
        (start, end)
    }

    /// The cells selected on row `y`, as a column range, clipped to the region.
    fn row_columns(&self, span: ((i32, i32), (i32, i32)), y: i32) -> Option<(i32, i32)> {
        let ((start_x, start_y), (end_x, end_y)) = span;
        if y < start_y || y > end_y || y < self.region.y || y >= self.region.bottom() {
            return None;
        }
        let from = if y == start_y { start_x } else { self.region.x };
        let to = if y == end_y { end_x } else { self.region.right() - 1 };
        let (from, to) = (from.max(self.region.x), to.min(self.region.right() - 1));
        (from <= to).then_some((from, to))
    }

    /// The selected text, rows joined with line breaks. A clean copy leaves out the cells inside
    /// `decorations`, rows made only of them (such as padding) and the spaces that pad each
    /// row's end; a raw copy keeps every cell.
    pub(crate) fn text(&self, buf: &Buffer, decorations: &[Rect], kind: CopyKind) -> String {
        let span = self.span(buf);
        let mut lines = Vec::new();
        for y in span.0.1..=span.1.1 {
            let Some((from, to)) = self.row_columns(span, y) else { continue };
            let content: Vec<i32> = (from..=to)
                .filter(|x| kind == CopyKind::Raw || !decorations.iter().any(|rect| rect.contains(*x, y)))
                .collect();
            if content.is_empty() {
                continue;
            }
            let line: String = content.into_iter().map(|x| symbol(buf, x, y)).collect();
            lines.push(match kind {
                CopyKind::Clean => line.trim_end().to_owned(),
                CopyKind::Raw => line,
            });
        }
        lines.join("\n")
    }

    /// Paints the selection colour over the selected cells; flashes after a copy.
    pub(crate) fn paint(&mut self, cx: &mut PaintCx<'_>) {
        self.painted.clear();
        if !self.visible {
            return;
        }
        let flash = cx.env().theme().motion().flash;
        let flashing = self.copied_at.is_some_and(|at| cx.now() < at + flash);
        if let Some(at) = self.copied_at.filter(|_| flashing) {
            cx.request_frame_in(at + flash - cx.now());
        }
        let states = if flashing { vec![State::Pressed] } else { Vec::new() };
        let style = cx.style("text-selection", None, &states).text();
        let bg = style.bg.unwrap_or_else(|| cx.color("active"));
        let readable = style.fg.unwrap_or_else(|| cx.color("text"));
        let span = self.span(cx.buf);
        for y in span.0.1..=span.1.1 {
            let Some((from, to)) = self.row_columns(span, y) else { continue };
            let row = Rect::new(from, y, clamp_u16(to - from + 1), 1);
            cx.fill_keeping_text_readable(row, bg, readable);
            self.painted.push(row);
        }
    }

    /// Marks the copy as done at `now`, for the flash.
    pub(crate) fn copied(&mut self, now: Duration) {
        self.copy_pending = None;
        self.copied_at = Some(now);
    }
}

/// The symbol drawn at `(x, y)`, empty off screen.
fn symbol(buf: &Buffer, x: i32, y: i32) -> &str {
    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else { return "" };
    buf.cell((x, y)).map_or("", Cell::symbol)
}

/// Whether a cell belongs to a word: identifiers, paths, addresses and numbers stay together.
fn is_word(symbol: &str) -> bool {
    !symbol.is_empty() && symbol.chars().all(|c| c.is_alphanumeric() || "_-./:@~#%+=".contains(c))
}
