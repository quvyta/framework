//! Timelines: a day on a time axis, with blocks for what filled it and gaps for what did not.
//!
//! The model an application builds is here, together with the input handling; `layout` works
//! out where the day, its blocks and their lanes sit and `paint` draws them.

mod layout;
mod paint;
mod readout;
#[cfg(test)]
mod tests;

use crate::date::TimeOfDay;
use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::Key;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::IndexMessage;
use super::axis::DAY;

/// The visible stretches zooming steps through, in seconds: a day, half a day, six hours, three
/// hours and one hour. An hour is the closest a day strip zooms in: below it a block of a few
/// minutes already spans several cells.
const ZOOM_SPANS: [u32; 5] = [DAY, 43_200, 21_600, 10_800, 3_600];

/// Builds a message from a new visible range.
type ZoomMessage<Msg> = Box<dyn Fn(TimeOfDay, TimeOfDay) -> Msg>;

/// One block of a [`Timeline`]: a stretch of the day with a name, such as work from 09:00 to
/// 11:10.
///
/// A block whose end is not after its start runs past midnight into the next day; one whose end
/// is its start is a moment, still drawn one cell wide.
///
/// A block that does not really stop where the strip shows it stopping — a session that runs on
/// past midnight, one carried over from the day before, a counter still running — marks that
/// edge with [`open_end`](Self::open_end) or [`open_start`](Self::open_start), and a block that
/// only echoes time recorded elsewhere, such as the next day's part of that session, is
/// [`faint`](Self::faint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeBlock {
    label: String,
    start: TimeOfDay,
    end: TimeOfDay,
    tone: Option<usize>,
    open_start: bool,
    open_end: bool,
    faint: bool,
}

impl TimeBlock {
    /// A block called `label` from `start` to `end`.
    #[must_use]
    pub fn new(label: impl Into<String>, start: TimeOfDay, end: TimeOfDay) -> Self {
        Self { label: label.into(), start, end, tone: None, open_start: false, open_end: false, faint: false }
    }

    /// Takes the theme's `index`-th series tone
    /// ([`Theme::series_color`](crate::theme::Theme::series_color)) instead of the accent, for a
    /// block of one category among several. Give the category the same index in the
    /// [`Legend`](super::Legend) beside the timeline, through [`Legend::tones`](super::Legend::tones).
    #[must_use]
    pub fn tone(mut self, index: usize) -> Self {
        self.tone = Some(index);
        self
    }

    /// Leaves the end open: the block goes on after the time it is drawn to, because it runs past
    /// the end of the day or because it is still running. Its last cells fade towards the track
    /// instead of stopping square, and the readout says which: "continues next day" for a block
    /// that reaches the day's end, "running" for one that stops before it.
    ///
    /// Only the block's own end is drawn open. Where a zoomed [`range`](Timeline::range) cuts the
    /// block the edge stays square, as the range is where the view stops, not where the block
    /// does. A block too short for a fade keeps its tone, and the readout still says it.
    #[must_use]
    pub fn open_end(mut self) -> Self {
        self.open_end = true;
        self
    }

    /// Leaves the start open: the block began before the time it is drawn from, such as a session
    /// carried over from the previous day. Its first cells fade in from the track, and the readout
    /// says "from the previous day" for a block that starts with the day, "from earlier" for one
    /// that starts later. Drawn like [`open_end`](Self::open_end), at the other edge.
    #[must_use]
    pub fn open_start(mut self) -> Self {
        self.open_start = true;
        self
    }

    /// Draws the block in a quieter tone, halfway between its own and the empty track, for time
    /// that belongs to something shown in full elsewhere — the next day's part of a session that
    /// started the night before. It still answers hover and selection, and it keeps its category's
    /// tone family, so it reads as the same thing, fainter.
    #[must_use]
    pub fn faint(mut self) -> Self {
        self.faint = true;
        self
    }
}

/// A day as a strip: blocks of colour for what filled it, the empty track for the gaps between
/// them — breaks, sleep, time not recorded.
///
/// The day is 24 hours from [`day_starts_at`](Self::day_starts_at), midnight by default. Blocks
/// are placed on it by time: a block covers the cells its stretch falls in, and a block shorter
/// than a cell still takes one cell, so a two-minute task is never lost on a whole-day strip.
/// Blocks without a [`tone`](TimeBlock::tone) take the accent; two blocks of the same tone that
/// touch are told apart by a quieter first cell on the later one, a seam of tone rather than a
/// line. A block wide enough for its name with a cell of air on each side writes the name inside
/// itself.
///
/// **Overlapping blocks** stand in lanes: each block takes the first lane, from the top, in which
/// nothing overlaps it, so a strip is one row tall for a day of consecutive blocks and grows a
/// row for each block that runs at the same time as another. An area shorter than the lanes puts
/// the lanes that do not fit into its last row, where the later block is drawn over the earlier
/// and the selected block is always drawn on top.
///
/// **Midnight.** A day that starts at 18:00 runs to 18:00 the next day, so a block from 23:00 to
/// 01:30 is one block in it. In a day that starts at midnight the same block is cut at the end
/// of the day; its part after midnight belongs to the next day's strip, where the caller gives
/// it from 00:00. A [`range`](Self::range) is placed in the same day, so a night from 22:00 to
/// 06:00 is a range of a day that starts before 22:00.
///
/// **Zoom** is the visible range, which the caller owns: [`range`](Self::range) shows a stretch
/// of the day across the whole width, and with [`on_zoom`](Self::on_zoom) the timeline asks for
/// a new one — `+` and `-` step through a day, 12, 6 and 3 hours and one hour around the selected
/// block, `0` goes back to the whole day, and the mouse wheel zooms around the pointer once the
/// timeline holds the focus (a click gives it), so scrolling a page past a timeline never gets
/// caught in it. Selecting a block outside a zoomed range moves the range to it.
///
/// A timeline is a picture until it is given [`on_select`](Self::on_select). Then the block
/// under the pointer and the selected block step towards the text colour, ←/→ (or h/l) walk the
/// blocks in time order, Home and End go to the first and the last, and a click selects the
/// block under it. [`readout`](Self::readout) adds a row that writes the block being read — the
/// one under the pointer, else the selected one — as its name, its times and its length, so the
/// pointer and the keyboard read the same words. Nothing moves or resizes when a block is
/// hovered or selected: a timeline is a narrow strip, not a list, so it never slides.
///
/// **Open edges and faint blocks.** A block marked [`open_end`](TimeBlock::open_end) or
/// [`open_start`](TimeBlock::open_start) fades towards the track over its last or first two
/// cells, a tone transition rather than a glyph, and the readout writes what the open edge means.
/// A [`faint`](TimeBlock::faint) block stands halfway between its tone and the track. On a
/// terminal with few colours every fading cell and every faint tone is kept apart from the track,
/// so a block never looks shorter than it is.
///
/// [`axis`](Self::axis) adds a row of hours under the strip, an [`Axis`](super::Axis) placed with
/// the same arithmetic as the blocks. An area too short for every row gives up the axis first,
/// then the readout, then lanes. The strip is made of colour, so it reads the same in every glyph
/// mode; on a terminal with few colours a tone that would merge with the track or with its own
/// hovered step is pushed further until the two differ.
///
/// Style keys: `timeline` (`track` for the empty day, `fill` for a block without a tone, `hover`
/// and `selected` for the tones a block steps towards), `timeline:focus` (`selected` while the
/// keyboard is on the timeline), `timeline-readout` (`fg` for the name, `detail` for the times
/// and the length), and `axis` for the hours.
pub struct Timeline<Msg> {
    blocks: Vec<TimeBlock>,
    day_start: TimeOfDay,
    range: Option<(TimeOfDay, TimeOfDay)>,
    axis: bool,
    readout: bool,
    selected: Option<usize>,
    disabled: bool,
    on_select: Option<IndexMessage<Msg>>,
    on_zoom: Option<ZoomMessage<Msg>>,
}

impl<Msg: 'static> Timeline<Msg> {
    /// A whole-day timeline of `blocks`, a day that starts at midnight.
    #[must_use]
    pub fn new(blocks: impl IntoIterator<Item = TimeBlock>) -> Self {
        Self {
            blocks: blocks.into_iter().collect(),
            day_start: TimeOfDay::default(),
            range: None,
            axis: false,
            readout: false,
            selected: None,
            disabled: false,
            on_select: None,
            on_zoom: None,
        }
    }

    /// Starts the day at `time` instead of midnight, e.g. 18:00 for a night shift, so blocks
    /// across midnight stay whole.
    #[must_use]
    pub fn day_starts_at(mut self, time: TimeOfDay) -> Self {
        self.day_start = time;
        self
    }

    /// Shows only the stretch from `from` to `to` across the whole width: into the next day when
    /// `to` is not after `from`, the whole day when the two are the same. The range is kept
    /// inside the day, so a range that runs past the day's end stops there.
    #[must_use]
    pub fn range(mut self, from: TimeOfDay, to: TimeOfDay) -> Self {
        self.range = Some((from, to));
        self
    }

    /// Adds a row of hours under the strip.
    #[must_use]
    pub fn axis(mut self) -> Self {
        self.axis = true;
        self
    }

    /// Adds a row that writes the block being read: its name, its times and its length.
    #[must_use]
    pub fn readout(mut self) -> Self {
        self.readout = true;
        self
    }

    /// The selected block, as an index into the blocks given.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Greys the timeline out: it cannot be focused, hovered, selected or zoomed.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Message for moving the selection to a block, carrying its index into the blocks given;
    /// turns the pointer and keyboard handling on.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// Message asking for a new visible range, `from` and `to` in the sense of
    /// [`range`](Self::range); turns zooming on.
    #[must_use]
    pub fn on_zoom(mut self, message: impl Fn(TimeOfDay, TimeOfDay) -> Msg + 'static) -> Self {
        self.on_zoom = Some(Box::new(message));
        self
    }

    /// Whether blocks answer the pointer and the keyboard.
    fn selectable(&self) -> bool {
        self.on_select.is_some() && !self.disabled && !self.blocks.is_empty()
    }

    /// Whether the range answers `+`, `-`, `0` and the wheel.
    fn zoomable(&self) -> bool {
        self.on_zoom.is_some() && !self.disabled
    }

    /// Moves the selection to `index` and, in a zoomed range the block is not wholly inside,
    /// asks for the range to move to it.
    fn select(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        if let Some(message) = &self.on_select
            && self.selected != Some(index)
        {
            cx.emit(message(index));
        }
        let (from, span) = self.visible();
        let Some(block) = self.blocks.get(index) else { return };
        let (start, end) = self.extent(block);
        if span >= DAY || (start >= from && end <= from + span) {
            return;
        }
        let middle = start + (end - start) / 2;
        let moved = middle.saturating_sub(span / 2).min(DAY - span);
        self.ask_range(cx, moved - moved % 60, span);
    }

    /// Steps the visible range one zoom step in or out around `anchor` (seconds into the day),
    /// keeping the anchor where it is on screen. At either end of the steps nothing is asked.
    fn zoom(&self, cx: &mut EventCx<'_, Msg>, inward: bool, anchor: u32) {
        let (from, span) = self.visible();
        let next = if inward {
            ZOOM_SPANS.into_iter().find(|s| *s < span)
        } else {
            ZOOM_SPANS.into_iter().rev().find(|s| *s > span)
        };
        let Some(next) = next else { return };
        let anchor = anchor.clamp(from, from + span);
        let before = u64::from(anchor - from) * u64::from(next) / u64::from(span);
        let start = anchor.saturating_sub(u32::try_from(before).unwrap_or(0)).min(DAY - next);
        self.ask_range(cx, start - start % 60, next);
    }

    /// Asks for the range of `span` seconds from `from` seconds into the day.
    fn ask_range(&self, cx: &mut EventCx<'_, Msg>, from: u32, span: u32) {
        if let Some(message) = &self.on_zoom
            && (from, span) != self.visible()
        {
            cx.emit(message(self.clock(from), self.clock(from + span)));
        }
    }

    /// The anchor the keyboard zooms around: the middle of the selected block when it is in
    /// view, else the middle of the range.
    fn keyboard_anchor(&self) -> u32 {
        let (from, span) = self.visible();
        self.selected
            .and_then(|index| self.blocks.get(index))
            .map(|block| self.extent(block))
            .filter(|(start, end)| *end > from && *start < from + span)
            .map_or(from + span / 2, |(start, end)| start + (end - start) / 2)
    }

    /// The block a key moves the selection to, in time order.
    fn key_target(&self, key: &crate::event::KeyEvent) -> Option<usize> {
        let order = self.order();
        let last = order.len().checked_sub(1)?;
        let at = self.selected.and_then(|selected| order.iter().position(|index| *index == selected));
        let position = if key.is_plain(Key::Left) || key.is_plain(Key::Char('h')) {
            at.map_or(last, |at| at.saturating_sub(1))
        } else if key.is_plain(Key::Right) || key.is_plain(Key::Char('l')) {
            at.map_or(0, |at| (at + 1).min(last))
        } else if key.is_plain(Key::Home) {
            0
        } else if key.is_plain(Key::End) {
            last
        } else {
            return None;
        };
        order.get(position).copied()
    }

    /// Handles `+`, `-` and `0`; says whether the key was one of them.
    fn zoom_key(&self, cx: &mut EventCx<'_, Msg>, key: &crate::event::KeyEvent) -> bool {
        let plain = |c: char| key.is_plain(Key::Char(c));
        if plain('+') || plain('=') {
            self.zoom(cx, true, self.keyboard_anchor());
        } else if plain('-') {
            self.zoom(cx, false, self.keyboard_anchor());
        } else if plain('0') {
            self.ask_range(cx, 0, DAY);
        } else {
            return false;
        }
        true
    }
}

impl<Msg: 'static> Widget<Msg> for Timeline<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let rows = self.lanes().saturating_add(u16::from(self.axis)).saturating_add(u16::from(self.readout));
        Size::new(available.width, rows).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        if self.selectable() || self.zoomable() {
            cx.register_hit(area);
        }
        self.paint_all(cx, area);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if !self.focusable() {
            return false;
        }
        let area = cx.area();
        match event {
            Event::Key(key) => {
                if self.zoomable() && self.zoom_key(cx, key) {
                    return true;
                }
                if !self.selectable() {
                    return false;
                }
                let Some(target) = self.key_target(key) else { return false };
                self.select(cx, target);
                true
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseKind::Down(MouseButton::Left) if self.selectable() => {
                    let Some(index) = self.block_at(area, mouse.x, mouse.y) else { return false };
                    self.select(cx, index);
                    true
                }
                MouseKind::ScrollUp | MouseKind::ScrollDown if self.zoomable() && cx.is_focused() => {
                    let anchor = self.time_at(area, mouse.x);
                    self.zoom(cx, mouse.kind == MouseKind::ScrollUp, anchor);
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        self.selectable() || self.zoomable()
    }
}
