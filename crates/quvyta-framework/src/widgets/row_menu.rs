//! The context menu of one row of a widget that owns its rows: which row it acts on, opening it
//! with the right button or with the menu key, and drawing it.
//!
//! A single [`ContextMenu`] wrapped around a list of rows opens for the area, not for the row that
//! was pressed, so its entries would act on whatever the cursor rests on instead. A file manager
//! deleting the wrong file is that mistake at its worst. So a row widget keeps the row its menu
//! belongs to and builds the entries for that row alone; [`Tree`](super::Tree), [`Table`](super::Table)
//! and [`CardGrid`](super::CardGrid) all work this way, and this module is what they share.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::Rect;
use crate::widget::{EventCx, PaintCx, Widget};

use super::context_item::{self, ContextItem};
use super::context_menu::{self, ContextMenu};

/// Builds the menu entries of the row `index`.
pub(crate) type RowMenuItems<Msg> = Box<dyn Fn(usize) -> Vec<ContextItem<Msg>>>;

/// The row whose context menu is open, kept in the widget's memory.
#[derive(Debug, Default)]
pub(crate) struct MenuRow(Option<usize>);

/// A row whose menu is about to open, and where it opens.
pub(crate) struct RowAnchor {
    /// The row the menu acts on.
    pub(crate) row: usize,
    /// The cell or the row the menu unfolds from.
    pub(crate) at: Rect,
    /// Whether the keyboard opened it, which highlights its first entry.
    pub(crate) keyboard: bool,
}

/// The menu of `row` with its messages replaced by positions, and the messages.
fn menu_for<Msg>(items: &RowMenuItems<Msg>, row: usize) -> (ContextMenu<usize>, Vec<Msg>) {
    let (entries, messages) = context_item::keyed(items(row));
    (ContextMenu::new(entries), messages)
}

/// Opens the menu of `anchor`'s row and remembers whose it is.
fn open<Msg: 'static>(cx: &mut EventCx<'_, Msg>, items: &RowMenuItems<Msg>, anchor: &RowAnchor) -> bool {
    let (menu, _) = menu_for(items, anchor.row);
    cx.memory::<MenuRow>().0 = Some(anchor.row);
    let at = anchor.at;
    let keyboard = anchor.keyboard;
    cx.with_messages(|cx: &mut EventCx<'_, usize>| menu.open(cx, at, keyboard));
    true
}

/// Offers `event` to the row menu of a widget with `count` rows; true when the menu used it.
///
/// A right press asks `press` for the row under the pointer and where its menu unfolds from; the
/// menu key or Shift+F10 asks `key` for the row the keys are on. Both may move the selection or
/// scroll first, so the menu never acts on a row that is not shown or not meant. While the menu is
/// open it takes the keys, the wheel and presses on itself and sends the message of the chosen
/// entry; a press beside it closes it and is left to the widget, which still acts on it.
pub(crate) fn event<Msg: 'static>(
    cx: &mut EventCx<'_, Msg>,
    event: &Event,
    items: Option<&RowMenuItems<Msg>>,
    count: usize,
    press: impl FnOnce(&mut EventCx<'_, Msg>, i32, i32) -> Option<RowAnchor>,
    key: impl FnOnce(&mut EventCx<'_, Msg>) -> Option<RowAnchor>,
) -> bool {
    let Some(items) = items else { return false };
    if count == 0 {
        return false;
    }
    let right_press = match event {
        Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Right) => Some((mouse.x, mouse.y)),
        _ => None,
    };
    if context_menu::is_open_in(cx) {
        let row = cx.memory::<MenuRow>().0.filter(|row| *row < count);
        let elsewhere = right_press.is_some_and(|(x, y)| !context_menu::contains(cx, x, y));
        // A right press beside the menu, or a row that went away, closes it; the press may open the
        // menu of another row below it.
        match row.filter(|_| !elsewhere) {
            Some(row) => {
                let (menu, messages) = menu_for(items, row);
                let (used, chosen) = cx.with_messages(|cx| menu.event(cx, event));
                if let Some(message) = chosen.last().and_then(|chosen| messages.into_iter().nth(*chosen)) {
                    cx.emit(message);
                }
                return used;
            }
            None => {
                cx.with_messages(|cx: &mut EventCx<'_, usize>| ContextMenu::<usize>::close(cx));
            }
        }
    }
    let anchor = match event {
        Event::Mouse(_) => match right_press {
            Some((x, y)) => press(cx, x, y),
            None => None,
        },
        Event::Key(pressed) if context_menu::is_menu_key(pressed) => key(cx),
        _ => None,
    };
    anchor.is_some_and(|anchor| open(cx, items, &anchor))
}

/// The row whose menu is open in the widget being painted, whose surface stays raised as if
/// hovered so it is clear what the menu acts on.
pub(crate) fn open_row<Msg>(cx: &mut PaintCx<'_>, items: Option<&RowMenuItems<Msg>>) -> Option<usize> {
    if items.is_none() || !context_menu::is_open(cx) {
        return None;
    }
    cx.memory::<MenuRow>().0
}

/// Paints the open row menu; called from the widget's `paint_overlay`.
pub(crate) fn paint<Msg: 'static>(cx: &mut PaintCx<'_>, items: Option<&RowMenuItems<Msg>>, anchor: Rect) {
    let Some(row) = open_row(cx, items) else { return };
    let Some(items) = items else { return };
    let (menu, _) = menu_for(items, row);
    menu.paint_overlay(cx, anchor);
}
