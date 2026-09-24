//! The pointer on the rows of a [`Table`](super::Table) or the cards of a
//! [`CardGrid`](super::CardGrid): selecting one row, several or a range, opening with one click or
//! two, drawing a box over the free space, and dragging the selection onto another row.
//!
//! Each of these is an option the widget is asked for, and a widget asked for none of them does
//! what it always did: a press selects the row under it and opens it. The two widgets lay their
//! rows out differently, one under another or in columns, so each says where its rows are through
//! [`PickedRows`] and this module does the rest the same way for both.

use crate::event::{MouseButton, MouseEvent, MouseKind};
use crate::geometry::Rect;
use crate::widget::{EventCx, PaintCx};

use super::click::{Click, LastPress};
use super::select_box::SelectBox;

/// A drop of dragged rows onto another row that a droppable [`Table`](super::Table) or
/// [`CardGrid`](super::CardGrid) asks for, such as files onto a folder. The widget never moves the
/// application's rows; the application does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowDrop {
    /// The rows that move, by index, in their order: the pressed row alone, or the whole
    /// selection when the drag started on one of its rows.
    pub rows: Vec<usize>,
    /// The row they were dropped on.
    pub into: usize,
}

/// Builds a message from a whole new selection.
type ChooseMessage<Msg> = Box<dyn Fn(Vec<usize>) -> Msg>;

/// Builds a message from a drop.
type DropMessage<Msg> = Box<dyn Fn(RowDrop) -> Msg>;

/// Tells whether the row of an index takes drops.
type DropFilter = Box<dyn Fn(usize) -> bool>;

/// Where a point is on a widget's rows.
pub(crate) enum Spot {
    /// On the row of this index.
    Row(usize),
    /// Among the rows but on none of them: below the last one, or between two cards.
    Free,
    /// Not among the rows at all: a header, a scrollbar.
    Outside,
}

/// What a widget whose rows the pointer picks tells the picking: where its rows are, and what
/// moving the cursor to one and opening one mean for it.
pub(crate) trait PickedRows<Msg> {
    /// Where `(x, y)` is on the rows as they were painted last.
    fn spot(&self, cx: &mut EventCx<'_, Msg>, x: i32, y: i32) -> Spot;
    /// The rows `rect` covers, in their order.
    fn covered(&self, cx: &mut EventCx<'_, Msg>, rect: Rect) -> Vec<usize>;
    /// The row the cursor is on.
    fn cursor(&self) -> Option<usize>;
    /// Moves the cursor to row `index`.
    fn select(&self, cx: &mut EventCx<'_, Msg>, index: usize);
    /// Opens row `index`, pressed at `at`.
    fn open(&self, cx: &mut EventCx<'_, Msg>, index: usize, at: (i32, i32));
}

/// How a widget's rows answer the pointer, as its options set it. Without any option a press
/// selects the row and opens it.
pub(crate) struct Picking<Msg> {
    /// How many clicks open a row.
    pub(crate) activate_on: Click,
    /// The rows selected together, when several can be.
    pub(crate) chosen: Vec<usize>,
    /// Asks for a whole new selection; set when several rows can be selected.
    pub(crate) on_choose: Option<ChooseMessage<Msg>>,
    /// Whether a drag over the free space draws a box that selects.
    pub(crate) box_select: bool,
    /// Asks to move dragged rows, and tells which rows take them.
    pub(crate) dropping: Option<(DropMessage<Msg>, DropFilter)>,
    /// Asks to copy dragged rows, for a drop released with Ctrl held.
    pub(crate) copy_drop: Option<DropMessage<Msg>>,
}

impl<Msg> Default for Picking<Msg> {
    fn default() -> Self {
        Self {
            activate_on: Click::Single,
            chosen: Vec::new(),
            on_choose: None,
            box_select: false,
            dropping: None,
            copy_drop: None,
        }
    }
}

/// A press on a row that may become a drag.
#[derive(Debug, Clone)]
struct RowPress {
    index: usize,
    start: (i32, i32),
    pointer: (i32, i32),
    dragging: bool,
    /// The rows a drag from this press carries.
    carried: Vec<usize>,
    /// Whether the press kept several selected rows, which a click without a drag reduces to
    /// this one.
    reduce: bool,
}

/// What the held button is doing.
#[derive(Debug, Clone)]
enum Held {
    Row(RowPress),
    Box(SelectBox<usize>),
}

/// The pointer's state on a widget's rows, kept in its memory.
#[derive(Debug, Default)]
struct PointerRows {
    presses: LastPress<usize>,
    held: Option<Held>,
    /// Where a Shift+click range starts: the row last clicked without Shift.
    anchor: Option<usize>,
}

impl<Msg: 'static> Picking<Msg> {
    /// Whether several rows can be selected.
    pub(crate) fn is_multi(&self) -> bool {
        self.on_choose.is_some()
    }

    /// Whether row `index` is one of the rows selected together.
    pub(crate) fn is_chosen(&self, index: usize) -> bool {
        self.is_multi() && self.chosen.contains(&index)
    }

    /// Reports `rows` as the new selection, when it differs from the current one.
    fn choose(&self, cx: &mut EventCx<'_, Msg>, rows: Vec<usize>) {
        if let Some(message) = &self.on_choose
            && rows != self.chosen
        {
            cx.emit(message(rows));
        }
    }

    /// Moves the cursor to row `index` and makes it the whole selection: a plain click.
    pub(crate) fn select_one(&self, cx: &mut EventCx<'_, Msg>, rows: &impl PickedRows<Msg>, index: usize) {
        rows.select(cx, index);
        if self.is_multi() {
            cx.memory::<PointerRows>().anchor = Some(index);
            self.choose(cx, vec![index]);
        }
    }

    /// Adds row `index` to the selection or takes it out, and moves the cursor there: Ctrl+click
    /// and Space. False when several rows cannot be selected.
    pub(crate) fn toggle(&self, cx: &mut EventCx<'_, Msg>, rows: &impl PickedRows<Msg>, index: usize) -> bool {
        if !self.is_multi() {
            return false;
        }
        let mut chosen = self.chosen.clone();
        match chosen.iter().position(|row| *row == index) {
            Some(at) => {
                chosen.remove(at);
            }
            None => chosen.push(index),
        }
        rows.select(cx, index);
        cx.memory::<PointerRows>().anchor = Some(index);
        self.choose(cx, chosen);
        true
    }

    /// Selects the rows from the last plain or Ctrl click to row `index`: Shift+click. The start
    /// stays, so a second Shift+click reshapes the same range.
    fn select_range(&self, cx: &mut EventCx<'_, Msg>, rows: &impl PickedRows<Msg>, index: usize) {
        let start = cx.memory::<PointerRows>().anchor.or_else(|| rows.cursor()).unwrap_or(index);
        cx.memory::<PointerRows>().anchor = Some(start);
        rows.select(cx, index);
        self.choose(cx, (start.min(index)..=start.max(index)).collect());
    }

    /// Whether dragged `carried` rows can be dropped on row `into`.
    pub(crate) fn takes_drop(&self, carried: &[usize], into: usize) -> bool {
        self.dropping.as_ref().is_some_and(|(_, accepts)| accepts(into)) && !carried.contains(&into)
    }

    /// Offers a pointer event to the picking. `None` when it is not the picking's to take, such as
    /// a press outside the rows, so the widget handles it itself.
    pub(crate) fn mouse(
        &self,
        cx: &mut EventCx<'_, Msg>,
        mouse: &MouseEvent,
        rows: &impl PickedRows<Msg>,
    ) -> Option<bool> {
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => self.press(cx, mouse, rows),
            MouseKind::Drag(MouseButton::Left) => self.drag(cx, (mouse.x, mouse.y), rows),
            MouseKind::Up(MouseButton::Left) => self.release(cx, mouse, rows),
            _ => None,
        }
    }

    fn press(&self, cx: &mut EventCx<'_, Msg>, mouse: &MouseEvent, rows: &impl PickedRows<Msg>) -> Option<bool> {
        let at = (mouse.x, mouse.y);
        let mods = mouse.mods;
        let index = match rows.spot(cx, mouse.x, mouse.y) {
            Spot::Outside => return None,
            Spot::Free if self.box_select && self.is_multi() => {
                let drawn = SelectBox::new(at, mods.ctrl && !mods.alt, &self.chosen);
                let covered = rows.covered(cx, drawn.rect());
                self.choose(cx, drawn.selection(covered));
                let memory = cx.memory::<PointerRows>();
                memory.presses.forget();
                memory.held = Some(Held::Box(drawn));
                cx.capture_pointer();
                return Some(true);
            }
            Spot::Free => return None,
            Spot::Row(index) => index,
        };
        if self.is_multi() && !mods.alt && mods.ctrl != mods.shift {
            cx.memory::<PointerRows>().presses.forget();
            if mods.ctrl {
                self.toggle(cx, rows, index);
            } else {
                self.select_range(cx, rows, index);
            }
            return Some(true);
        }
        let now = cx.now();
        if self.activate_on == Click::Double && cx.memory::<PointerRows>().presses.press(index, now) {
            rows.select(cx, index);
            rows.open(cx, index, at);
            return Some(true);
        }
        // A press on one of several selected rows keeps them all, so they can be dragged together;
        // a click without a drag reduces them to this row on release.
        let reduce = self.dropping.is_some() && self.chosen.len() > 1 && self.is_chosen(index);
        if reduce {
            rows.select(cx, index);
        } else {
            self.select_one(cx, rows, index);
        }
        if self.dropping.is_none() {
            if self.activate_on == Click::Single {
                rows.open(cx, index, at);
            }
            return Some(true);
        }
        let carried = if reduce { sorted(&self.chosen) } else { vec![index] };
        let press = RowPress { index, start: at, pointer: at, dragging: false, carried, reduce };
        cx.memory::<PointerRows>().held = Some(Held::Row(press));
        cx.capture_pointer();
        Some(true)
    }

    fn drag(&self, cx: &mut EventCx<'_, Msg>, at: (i32, i32), rows: &impl PickedRows<Msg>) -> Option<bool> {
        let PointerRows { presses, held, .. } = cx.memory::<PointerRows>();
        match held.as_mut()? {
            Held::Box(drawn) => {
                drawn.stretch(at);
                let drawn = drawn.clone();
                let covered = rows.covered(cx, drawn.rect());
                self.choose(cx, drawn.selection(covered));
            }
            Held::Row(press) => {
                press.pointer = at;
                if !press.dragging && at != press.start {
                    press.dragging = true;
                    // A press that became a drag is not the first half of a double click.
                    presses.forget();
                }
            }
        }
        Some(true)
    }

    fn release(&self, cx: &mut EventCx<'_, Msg>, mouse: &MouseEvent, rows: &impl PickedRows<Msg>) -> Option<bool> {
        let at = (mouse.x, mouse.y);
        match cx.memory::<PointerRows>().held.take()? {
            Held::Box(mut drawn) => {
                drawn.stretch(at);
                let covered = rows.covered(cx, drawn.rect());
                self.choose(cx, drawn.selection(covered));
            }
            Held::Row(press) if press.dragging => {
                let Spot::Row(into) = rows.spot(cx, mouse.x, mouse.y) else { return Some(true) };
                if !self.takes_drop(&press.carried, into) {
                    return Some(true);
                }
                let drop = RowDrop { rows: press.carried, into };
                // A terminal that does not report Ctrl with the pointer simply moves.
                let message = match (&self.copy_drop, &self.dropping) {
                    (Some(copy), _) if mouse.mods.ctrl => copy(drop),
                    (_, Some((moving, _))) => moving(drop),
                    (_, None) => return Some(true),
                };
                cx.emit(message);
            }
            Held::Row(press) => {
                if press.reduce {
                    self.select_one(cx, rows, press.index);
                }
                if self.activate_on == Click::Single {
                    rows.open(cx, press.index, at);
                }
            }
        }
        Some(true)
    }
}

/// `rows` in their order.
fn sorted(rows: &[usize]) -> Vec<usize> {
    let mut rows = rows.to_vec();
    rows.sort_unstable();
    rows
}

/// The rows being dragged and where the pointer is, while a drag lasts.
pub(crate) fn dragged(cx: &mut PaintCx<'_>) -> Option<((i32, i32), Vec<usize>)> {
    match &cx.memory::<PointerRows>().held {
        Some(Held::Row(press)) if press.dragging => Some((press.pointer, press.carried.clone())),
        _ => None,
    }
}

/// The selection box being drawn, while it is.
pub(crate) fn drawn_box(cx: &mut PaintCx<'_>) -> Option<Rect> {
    match &cx.memory::<PointerRows>().held {
        Some(Held::Box(drawn)) => Some(drawn.rect()),
        _ => None,
    }
}
