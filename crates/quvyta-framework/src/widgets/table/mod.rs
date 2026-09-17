//! Tables: rows of cells under a header, virtualised, sortable and selectable.
//!
//! The columns, cells and rows an application builds are in `model`; `layout` decides column
//! widths and which columns show; `paint` draws the header and the rows.

mod layout;
mod model;
mod paint;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, KeyChord, Modifiers};
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::IndexMessage;
use super::row::LEAD;
use super::rows::{self, RowScroll, Step};
use layout::Placed;
pub use model::{Column, ColumnWidth, SortDirection, TableCell, TableRow};

/// Cells between two columns. Columns are told apart by space, never by a drawn line.
const COLUMN_GAP: u16 = 2;

/// Cells taken by the check mark of a multi-select table.
const MARK: u16 = 2;

/// Builds a message from a column and a direction.
type SortMessage<Msg> = Box<dyn Fn(usize, SortDirection) -> Msg>;

/// Rows of cells under a header row. Only the rows on screen are drawn, so a table stays fast
/// with any number of rows; pass the rows as an `Arc<[TableRow]>` kept in your state to avoid
/// copying them every frame.
///
/// The header sits on a raised surface with faint titles. Columns are separated by space; their
/// widths follow [`ColumnWidth`] and [`Column::min`], and when the minimums do not fit, the
/// columns scroll sideways: arrows at the ends of the header show which side hides columns. The
/// application owns the selection, the checked rows and the sort order and is told about changes
/// through messages. A hovered or selected row raises its surface and shows the pillar; only its
/// first cell slides one cell right. The check mark of a multi-select table never moves.
///
/// Keys while focused: ↑/↓ or k/j, PgUp/PgDn, Home/End move; Enter activates; Space toggles in
/// multi-select tables and activates otherwise; ←/→ scroll columns that overflow; with
/// [`Table::on_sort`], `s` sorts by the next sortable column and `shift+s` reverses the order.
///
/// The mouse does the same: a click on a row selects and activates it, a click on its check mark
/// (or the cell after it) only toggles, a click on a sortable title sorts by it and a second
/// click reverses it, and a click on a header arrow scrolls the columns one step.
///
/// Style keys: rows use `list-item` (`hover`, `selected`, `focus`, `pressed`) and
/// `list-item.faint` like [`List`](super::List); `table-header` (`bg`, `fg`) with `hover` over a
/// sortable title and `selected` on the sorted one; `table-sort` for the sort arrow;
/// `table-scroll` (`fg`, `bg`) with `hover` for the header arrows; `list-header` for the empty
/// text; `scrollbar`.
pub struct Table<Msg> {
    columns: Vec<Column>,
    rows: Arc<[TableRow]>,
    selected: Option<usize>,
    checked: Option<Vec<bool>>,
    sort: Option<(usize, SortDirection)>,
    empty: String,
    on_select: Option<IndexMessage<Msg>>,
    on_activate: Option<IndexMessage<Msg>>,
    on_toggle: Option<IndexMessage<Msg>>,
    on_sort: Option<SortMessage<Msg>>,
}

#[derive(Debug, Default)]
struct TableMemory {
    /// Widest cell of every column, for the rows it was measured on.
    fit: Option<(Arc<[TableRow]>, Vec<u16>)>,
    /// First visible column when the columns overflow.
    column_offset: usize,
    /// The largest useful `column_offset` in the last frame; zero when nothing overflows.
    max_column_offset: usize,
    /// Whether columns were hidden on the right in the last frame.
    more: bool,
    placed: Vec<Placed>,
}

impl<Msg: 'static> Table<Msg> {
    /// A table with `columns` showing `rows`.
    #[must_use]
    pub fn new(columns: impl IntoIterator<Item = Column>, rows: impl Into<Arc<[TableRow]>>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: rows.into(),
            selected: None,
            checked: None,
            sort: None,
            empty: String::new(),
            on_select: None,
            on_activate: None,
            on_toggle: None,
            on_sort: None,
        }
    }

    /// The selected row index.
    #[must_use]
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Turns on multiple selection; `checked[i]` tells whether row `i` is checked.
    #[must_use]
    pub fn checked(mut self, checked: Vec<bool>) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Shows the sort arrow on `column` pointing in `direction`. The rows must already be in
    /// that order; the table does not reorder them.
    #[must_use]
    pub fn sort(mut self, column: usize, direction: SortDirection) -> Self {
        self.sort = Some((column, direction));
        self
    }

    /// Text shown under the header when there are no rows.
    #[must_use]
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    /// Message for moving the selection to a row.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// Message for opening a row (Enter, click).
    #[must_use]
    pub fn on_activate(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_activate = Some(Box::new(message));
        self
    }

    /// Message for checking or unchecking a row of a multi-select table (Space, click on the mark).
    #[must_use]
    pub fn on_toggle(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_toggle = Some(Box::new(message));
        self
    }

    /// Message asking to sort by a column in a direction; turns on sorting by clicking titles of
    /// [`Column::sortable`] columns and with `s` / `shift+s`.
    #[must_use]
    pub fn on_sort(mut self, message: impl Fn(usize, SortDirection) -> Msg + 'static) -> Self {
        self.on_sort = Some(Box::new(message));
        self
    }

    fn lead(&self) -> u16 {
        LEAD + if self.checked.is_some() { MARK } else { 0 }
    }

    fn select(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        if Some(index) != self.selected
            && let Some(message) = &self.on_select
        {
            cx.emit(message(index));
        }
    }

    fn activate(&self, cx: &mut EventCx<'_, Msg>, index: usize) -> bool {
        let Some(message) = &self.on_activate else {
            return false;
        };
        cx.memory::<RowScroll>().flashed = Some(index);
        cx.flash();
        cx.emit(message(index));
        true
    }

    fn toggle(&self, cx: &mut EventCx<'_, Msg>, index: usize) -> bool {
        match (&self.checked, &self.on_toggle) {
            (Some(_), Some(message)) => {
                cx.emit(message(index));
                true
            }
            _ => false,
        }
    }

    fn request_sort(&self, cx: &mut EventCx<'_, Msg>, column: usize, direction: SortDirection) -> bool {
        match &self.on_sort {
            Some(message) if self.columns.get(column).is_some_and(|c| c.sortable) => {
                cx.emit(message(column, direction));
                true
            }
            _ => false,
        }
    }

    /// Sorting after a click on `column`'s title: the other direction when it is already sorted.
    fn click_sort(&self, column: usize) -> SortDirection {
        match self.sort {
            Some((sorted, direction)) if sorted == column => direction.reversed(),
            _ => SortDirection::Ascending,
        }
    }

    /// Scrolls overflowing columns one column forward or back. Returns whether the columns
    /// overflow at all, so the key or press is used even at an end.
    fn scroll_columns(cx: &mut EventCx<'_, Msg>, forward: bool) -> bool {
        let memory = cx.memory::<TableMemory>();
        if memory.max_column_offset == 0 {
            return false;
        }
        memory.column_offset = if forward {
            (memory.column_offset + 1).min(memory.max_column_offset)
        } else {
            memory.column_offset.saturating_sub(1)
        };
        true
    }

    /// Which way the header's scroll arrow at column `x` scrolls, when one is drawn there: the
    /// back arrow in the first cell while columns are hidden on the left, the forward arrow in
    /// the last cell while columns are hidden on the right.
    fn scroll_arrow_at(cx: &mut EventCx<'_, Msg>, area: Rect, x: i32) -> Option<bool> {
        let memory = cx.memory::<TableMemory>();
        if x == area.x && memory.column_offset > 0 {
            Some(false)
        } else if x == area.right() - 1 && memory.more {
            Some(true)
        } else {
            None
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Table<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let rows = self.rows.len().max(1) + 1;
        let widths =
            self.columns.iter().fold(0u16, |sum, c| sum.saturating_add(c.title_width()).saturating_add(COLUMN_GAP));
        Size::new(widths.saturating_add(self.lead() + 1), clamp_u16(i32::try_from(rows).unwrap_or(i32::MAX)))
            .min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        cx.register_hit(area);
        let body = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        let total = self.rows.len();
        let visible = usize::from(body.height);
        let overflows = total > visible;
        let lead = self.lead();
        let room = area.width.saturating_sub(lead + u16::from(overflows));

        let (placed, column_offset, more) = {
            let memory = cx.memory::<TableMemory>();
            let widest = if self.columns.iter().any(|c| c.width == ColumnWidth::Fit) {
                self.widest_cells(memory)
            } else {
                vec![0; self.columns.len()]
            };
            let (widths, overflow) = self.widths(&widest, room);
            // Columns that scroll sideways keep the forward arrow's cell and one cell of air before
            // it free, like the back arrow and the lead on the left, so the arrow never covers or
            // touches a title or a value. A scrollbar column already gives the arrow its cell.
            let arrow = if overflow { 2 - u16::from(overflows) } else { 0 };
            let room = room.saturating_sub(arrow);
            let max_offset = if overflow { Self::max_offset(&widths, room) } else { 0 };
            memory.max_column_offset = max_offset;
            memory.column_offset = memory.column_offset.min(max_offset);
            let placed = Self::place(&widths, memory.column_offset, area.x + i32::from(lead), room);
            let more =
                placed.last().is_some_and(|last| last.column + 1 < widths.len() || last.width < widths[last.column]);
            memory.placed.clone_from(&placed);
            memory.more = more;
            (placed, memory.column_offset, more)
        };
        self.paint_header(cx, area, &placed, column_offset, more);

        if total == 0 {
            let faint = cx.style("list-header", None, &[]).text();
            let budget = area.width.saturating_sub(LEAD);
            cx.text(area.x + i32::from(LEAD), body.y, &self.empty, faint, budget);
            return;
        }
        let focused = cx.is_focused();
        let pressed = cx.is_pressed();
        let offset = cx.memory::<RowScroll>().follow(self.selected, total, visible);
        let row_width = area.width.saturating_sub(u16::from(overflows));
        for (row, index) in (offset..total).take(visible).enumerate() {
            let rect = Rect::new(area.x, body.y + i32::try_from(row).unwrap_or(0), row_width, 1);
            self.paint_row(cx, rect, index, &placed, focused, pressed);
        }
        rows::paint_scrollbar(cx, body, total, offset, None);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let area = cx.area();
        let body = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        let total = self.rows.len();
        match event {
            Event::Key(key) => {
                if let Some(step) = Step::from_key(key) {
                    let Some(target) = step.apply(self.selected, total, usize::from(body.height)) else {
                        return false;
                    };
                    self.select(cx, target);
                    return true;
                }
                if key.is_plain(Key::Left) || key.is_plain(Key::Right) {
                    return Self::scroll_columns(cx, key.is_plain(Key::Right));
                }
                if key.is_plain(Key::Enter) {
                    return self.selected.is_some_and(|index| self.activate(cx, index));
                }
                if key.is_plain(Key::Space) {
                    let Some(index) = self.selected else { return false };
                    return self.toggle(cx, index) || self.activate(cx, index);
                }
                let shift_s = KeyChord { key: Key::Char('s'), mods: Modifiers { shift: true, ..Modifiers::default() } };
                if self.on_sort.is_some() && (key.is_plain(Key::Char('s')) || key.chord == shift_s) {
                    return match (key.chord == shift_s, self.sort) {
                        (true, Some((column, direction))) => self.request_sort(cx, column, direction.reversed()),
                        (true, None) => false,
                        (false, current) => {
                            let start = current.map_or(0, |(column, _)| column + 1);
                            let count = self.columns.len();
                            let next = (0..count)
                                .map(|step| (start + step) % count.max(1))
                                .find(|i| self.columns[*i].sortable);
                            next.is_some_and(|column| self.request_sort(cx, column, SortDirection::Ascending))
                        }
                    };
                }
                false
            }
            Event::Mouse(mouse) => {
                if rows::scroll_mouse(cx, mouse, body, total) {
                    return true;
                }
                if mouse.kind != MouseKind::Down(MouseButton::Left) {
                    return false;
                }
                if mouse.y == area.y {
                    if let Some(forward) = Self::scroll_arrow_at(cx, area, mouse.x) {
                        return Self::scroll_columns(cx, forward);
                    }
                    let placed = cx.memory::<TableMemory>().placed.clone();
                    let Some(place) = placed.iter().find(|place| Self::spans(place, mouse.x)) else {
                        return false;
                    };
                    return self.request_sort(cx, place.column, self.click_sort(place.column));
                }
                let offset = cx.memory::<RowScroll>().offset;
                let Some(index) = usize::try_from(mouse.y - body.y).ok().map(|row| offset + row).filter(|i| *i < total)
                else {
                    return false;
                };
                if self.checked.is_some() && mouse.x < area.x + i32::from(LEAD + MARK) && self.toggle(cx, index) {
                    return true;
                }
                self.select(cx, index);
                self.activate(cx, index);
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.rows.is_empty()
    }
}
