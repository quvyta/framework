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

use super::click::Click;
use super::row::LEAD;
use super::row_menu::{self, RowAnchor, RowMenuItems};
use super::row_pointer::{self, PickedRows, Picking, RowDrop, Spot};
use super::rows::{self, RowScroll, Step};
use super::select_box;
use super::{ContextItem, IndexMessage};
use layout::Placed;
pub use model::{Column, ColumnWidth, SortDirection, TableCell, TableRow};
use paint::RowPaint;

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
/// Four capabilities make the rows work the way a file explorer's do, each off until asked for:
///
/// - [`activate_on(Click::Double)`](Self::activate_on): a click only selects a row and a double
///   click activates it, so a click can start a drag or a selection without opening anything.
/// - [`multi_select`](Self::multi_select): several rows are selected at once with Ctrl+click,
///   Shift+click, Shift+arrows, Ctrl+A and Space; they share the selection tone while only the cursor's row carries the
///   pillar and slides.
/// - [`box_select`](Self::box_select): a drag from the free space below the rows draws a box, and
///   the rows it covers become the selection.
/// - [`droppable`](Self::droppable): the selection is dragged onto a row that takes it, such as a
///   folder, which takes the accent tone while the drag is over it.
///
/// Style keys: rows use `list-item` (`hover`, `selected`, `focus`, `pressed`) and
/// `list-item.faint` like [`List`](super::List); `table-header` (`bg`, `fg`) with `hover` over a
/// sortable title and `selected` on the sorted one; `table-sort` for the sort arrow;
/// `table-scroll` (`fg`, `bg`) with `hover` for the header arrows; `list-header` for the empty
/// text; `tree-drop` for the row a drag would drop on; `text-selection` (`bg`) for the selection
/// box; `scrollbar`.
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
    menu: Option<RowMenuItems<Msg>>,
    menu_on_activate: bool,
    picking: Picking<Msg>,
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
            menu: None,
            menu_on_activate: false,
            picking: Picking::default(),
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

    /// Gives every row a context menu: `items(index)` builds the entries for the row of that
    /// index, and the menu acts on the row it was opened on rather than on the selected one.
    ///
    /// A right press on a row opens the menu at the pointer; the menu key or Shift+F10 opens the
    /// menu of the selected row below it, scrolling it into view first. The row the menu belongs
    /// to stays raised while it is open, so it is clear what the entries act on. A right press on
    /// a row that is not checked makes it the selection first, so a menu never acts on rows the
    /// person did not mean.
    #[must_use]
    pub fn context_menu(mut self, items: impl Fn(usize) -> Vec<ContextItem<Msg>> + 'static) -> Self {
        self.menu = Some(Box::new(items));
        self
    }

    /// Makes a row's [context menu](Self::context_menu) its action: Enter opens the menu of the
    /// selected row below it and a click opens the menu of the clicked row where it was clicked,
    /// instead of sending [`on_activate`](Self::on_activate).
    ///
    /// For a table whose rows are acted on only through a few choices: a right click is not what
    /// most people try in a terminal and many keyboards have no menu key, so the menu is also
    /// reached the way any row is opened. A row whose menu has no entries opens nothing. Off by
    /// default; without a context menu it does nothing.
    #[must_use]
    pub fn menu_on_activate(mut self, on: bool) -> Self {
        self.menu_on_activate = on;
        self
    }

    /// How many clicks activate a row: [`Click::Single`], the default, selects and activates at
    /// once; [`Click::Double`] only selects on a click and activates on a second press on the same
    /// row within [`Click::INTERVAL`]. Enter activates either way.
    #[must_use]
    pub fn activate_on(mut self, click: Click) -> Self {
        self.picking.activate_on = click;
        self
    }

    /// Lets several rows be selected at once: `selected` holds their indexes and `message(rows)`
    /// asks the application to make `rows` the whole new selection.
    ///
    /// The row given to [`selected`](Self::selected) stays the cursor: the row the keys move from
    /// and the only one with the pillar, while every selected row takes the selection tone.
    /// Ctrl+click adds a row or takes it out, Shift+click selects the rows from the last plain or
    /// Ctrl click to this one; Shift with ↑/↓, PgUp/PgDn or Home/End extends that range, Ctrl+A
    /// selects every row, Space adds or takes out the cursor's row, Esc reduces several selected
    /// rows to the cursor's, and a plain click or arrow selects that one row. A right click on a selected row keeps the selection for its menu; on another
    /// row it makes that row the selection first.
    #[must_use]
    pub fn multi_select(mut self, selected: &[usize], message: impl Fn(Vec<usize>) -> Msg + 'static) -> Self {
        self.picking.chosen = selected.to_vec();
        self.picking.on_choose = Some(Box::new(message));
        self
    }

    /// Lets a drag from the free space below the rows draw a box: the rows it covers become the
    /// selection while it is drawn, or join it when Ctrl was held at the press, and a click there
    /// without a drag clears the selection. The box is a tone laid over the cells it covers, never
    /// a frame. It needs [`multi_select`](Self::multi_select) and does nothing without it.
    #[must_use]
    pub fn box_select(mut self, on: bool) -> Self {
        self.picking.box_select = on;
        self
    }

    /// Lets rows be dragged onto other rows, such as files onto a folder: `accepts(index)` tells
    /// whether a row takes drops and `message(RowDrop)` asks the application to move the rows.
    ///
    /// A drag carries the pressed row, or the whole [selection](Self::multi_select) when it is
    /// pressed on one of its rows; a click on a selected row without a drag makes it the one
    /// selected row on release. The row under the pointer takes the accent tone while it can take
    /// the drag. A release anywhere else, or on one of the dragged rows, does nothing. With
    /// [`Click::Single`] a row activates on release rather than on press, so pressing a row to
    /// drag it does not activate it.
    #[must_use]
    pub fn droppable(
        mut self,
        message: impl Fn(RowDrop) -> Msg + 'static,
        accepts: impl Fn(usize) -> bool + 'static,
    ) -> Self {
        self.picking.dropping = Some((Box::new(message), Box::new(accepts)));
        self
    }

    /// A drop released with Ctrl held asks for a copy with `message` instead of the move of
    /// [`droppable`](Self::droppable), the way a file explorer copies. A terminal that does not
    /// report Ctrl with the pointer always moves. It does nothing without `droppable`.
    #[must_use]
    pub fn on_copy_drop(mut self, message: impl Fn(RowDrop) -> Msg + 'static) -> Self {
        self.picking.copy_drop = Some(Box::new(message));
        self
    }

    /// The cells the rows have to themselves: the scrollbar column is not part of a row.
    fn rows_width(area: Rect, overflows: bool) -> u16 {
        area.width.saturating_sub(u16::from(overflows))
    }

    /// Offers `event` to the row menu. A right press picks the row under the pointer and makes it
    /// the selection unless it is checked, because a menu on a checked row acts on the checked
    /// rows, which the application knows about.
    fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let area = cx.area();
        let body = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        let total = self.rows.len();
        let visible = usize::from(body.height);
        let overflows = total > visible;
        row_menu::event(
            cx,
            event,
            self.menu.as_ref(),
            total,
            |cx, x, y| {
                if y < body.y || x >= area.x + i32::from(Self::rows_width(area, overflows)) {
                    return None;
                }
                let offset = cx.memory::<RowScroll>().offset;
                let row = usize::try_from(y - body.y).ok().map(|row| offset + row).filter(|row| *row < total)?;
                let checked = self.checked.as_ref().is_some_and(|checked| checked.get(row).copied().unwrap_or(false));
                if self.picking.is_multi() && !self.picking.is_chosen(row) {
                    self.picking.select_one(cx, self, row);
                } else if !checked && !self.picking.is_multi() {
                    self.select(cx, row);
                }
                Some(RowAnchor { row, at: Rect::new(x, y, 1, 1), keyboard: false })
            },
            |cx| self.selected_anchor(cx),
        )
    }

    /// Where the menu of the selected row unfolds from for the keyboard: below the whole row,
    /// scrolled into view first.
    fn selected_anchor(&self, cx: &mut EventCx<'_, Msg>) -> Option<RowAnchor> {
        let area = cx.area();
        let body_y = area.y + 1;
        let total = self.rows.len();
        let visible = usize::from(area.height.saturating_sub(1));
        let overflows = total > visible;
        let row = self.selected.filter(|row| *row < total)?;
        let memory = cx.memory::<RowScroll>();
        if row < memory.offset {
            memory.offset = row;
        } else if visible > 0 && row >= memory.offset + visible {
            memory.offset = row + 1 - visible;
        }
        let y = body_y + i32::try_from(row - memory.offset).unwrap_or(0);
        let at = Rect::new(area.x, y, Self::rows_width(area, overflows), 1);
        Some(RowAnchor { row, at, keyboard: true })
    }

    /// Whether Enter and a click open the row's menu rather than the row.
    fn activation_is_menu(&self) -> bool {
        self.menu_on_activate && self.menu.is_some()
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
        // An open row menu takes the pointer: only the row it acts on stays raised, so the menu
        // and the row it belongs to are read together.
        let menu_row = row_menu::open_row(cx, self.menu.as_ref());
        if menu_row.is_some() {
            cx.request_overlay(area);
        }
        let offset = cx.memory::<RowScroll>().follow(self.selected, total, visible);
        let row_width = Self::rows_width(area, overflows);
        let rows_rect = Rect::new(area.x, body.y, row_width, body.height);
        // The row a drag is over takes the accent tone when it can take what is dragged.
        let target = row_pointer::dragged(cx).and_then(|((x, y), carried)| {
            let index = offset + usize::try_from(y - body.y).ok()?;
            (rows_rect.contains(x, y) && index < total && self.picking.takes_drop(&carried, index)).then_some(index)
        });
        for (row, index) in (offset..total).take(visible).enumerate() {
            let rect = Rect::new(area.x, body.y + i32::try_from(row).unwrap_or(0), row_width, 1);
            self.paint_row(cx, rect, index, &placed, RowPaint { focused, pressed, menu_row, target });
        }
        if let Some(drawn) = row_pointer::drawn_box(cx) {
            select_box::paint(cx, drawn, rows_rect);
        }
        rows::paint_scrollbar(cx, body, total, offset, None);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        row_menu::paint(cx, self.menu.as_ref(), anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.menu_event(cx, event) {
            return true;
        }
        let area = cx.area();
        let body = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        let total = self.rows.len();
        match event {
            Event::Key(key) => {
                let page = usize::from(body.height);
                let extend = |_: &mut EventCx<'_, Msg>, plain: &crate::event::KeyEvent| {
                    rows::SHIFT_STEPS
                        .contains(&plain.chord.key)
                        .then(|| Step::from_key(plain).and_then(|step| step.apply(self.selected, total, page)))
                };
                if self.picking.selection_key(cx, key, self, total, extend) {
                    return true;
                }
                if let Some(step) = Step::from_key(key) {
                    let Some(target) = step.apply(self.selected, total, page) else {
                        return false;
                    };
                    if self.picking.is_multi() {
                        self.picking.select_one(cx, self, target);
                    } else {
                        self.select(cx, target);
                    }
                    return true;
                }
                if key.is_plain(Key::Left) || key.is_plain(Key::Right) {
                    return Self::scroll_columns(cx, key.is_plain(Key::Right));
                }
                if key.is_plain(Key::Enter) {
                    if self.activation_is_menu() {
                        return self
                            .selected_anchor(cx)
                            .is_some_and(|anchor| row_menu::open_as_action(cx, self.menu.as_ref(), &anchor));
                    }
                    return self.selected.is_some_and(|index| self.activate(cx, index));
                }
                if key.is_plain(Key::Space) {
                    let Some(index) = self.selected else { return false };
                    return self.picking.toggle(cx, self, index) || self.toggle(cx, index) || self.activate(cx, index);
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
                if mouse.kind == MouseKind::Down(MouseButton::Left) {
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
                    if self.checked.is_some()
                        && mouse.x < area.x + i32::from(LEAD + MARK)
                        && let Spot::Row(index) = self.spot(cx, mouse.x, mouse.y)
                        && self.toggle(cx, index)
                    {
                        return true;
                    }
                }
                self.picking.mouse(cx, mouse, self).unwrap_or(false)
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.rows.is_empty()
    }
}

impl<Msg: 'static> PickedRows<Msg> for Table<Msg> {
    fn spot(&self, cx: &mut EventCx<'_, Msg>, x: i32, y: i32) -> Spot {
        let area = cx.area();
        let body = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        let total = self.rows.len();
        let overflows = total > usize::from(body.height);
        let rows = Rect::new(area.x, body.y, Self::rows_width(area, overflows), body.height);
        if !rows.contains(x, y) {
            return Spot::Outside;
        }
        let index = cx.memory::<RowScroll>().offset + usize::try_from(y - body.y).unwrap_or(0);
        if index < total { Spot::Row(index) } else { Spot::Free }
    }

    fn covered(&self, cx: &mut EventCx<'_, Msg>, rect: Rect) -> Vec<usize> {
        let area = cx.area();
        let body = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
        let offset = cx.memory::<RowScroll>().offset;
        let (top, bottom) = (rect.y.max(body.y), rect.bottom().min(body.bottom()));
        (top..bottom)
            .filter_map(|y| usize::try_from(y - body.y).ok())
            .map(|row| offset + row)
            .filter(|index| *index < self.rows.len())
            .collect()
    }

    fn cursor(&self) -> Option<usize> {
        self.selected
    }

    fn select(&self, cx: &mut EventCx<'_, Msg>, index: usize) {
        Table::select(self, cx, index);
    }

    fn open(&self, cx: &mut EventCx<'_, Msg>, index: usize, (x, y): (i32, i32)) {
        if self.activation_is_menu() {
            let anchor = RowAnchor { row: index, at: Rect::new(x, y, 1, 1), keyboard: false };
            row_menu::open_as_action(cx, self.menu.as_ref(), &anchor);
        } else {
            self.activate(cx, index);
        }
    }
}
