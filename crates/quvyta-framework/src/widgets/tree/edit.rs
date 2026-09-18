//! Reordering a tree's nodes among their siblings, by dragging or with the keys, and the context
//! menu of a node.
//!
//! Siblings are a stack of rows, so a drag works like the tabs of a [`TabRail`](super::super::TabRail):
//! the landing place is the resting sibling under the pointer ([`drop_target`]), the siblings make
//! room while the drag lasts ([`preview_order`]), a ghost follows the pointer and the move is the
//! same `from`/`to` pair [`TabEdit::Move`] applies.

use crate::event::{Event, KeyEvent, MouseButton, MouseEvent, MouseKind};
use crate::geometry::Rect;
use crate::keymap::Key;
use crate::widget::{EventCx, PaintCx, Widget};

use super::super::TabEdit;
use super::super::context_item;
use super::super::context_menu::{self, ContextMenu};
use super::super::edge_scroll::{Edge, EdgeScroll, Zone};
use super::super::rows::RowScroll;
use super::super::tab_model::{Direction, drop_target};
use super::{Flat, Tree, TreeNode};

/// A move of one node among its siblings that a [reorderable](Tree::reorderable) tree asks for.
/// The tree never changes the application's nodes; [`apply`](Self::apply) does the bookkeeping on
/// the application's own list of siblings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeMove {
    /// The key of the node that moves.
    pub key: String,
    /// The key of its parent, `None` for a top-level node. The node keeps this parent.
    pub parent: Option<String>,
    /// The node's position among its siblings before the move, counted from 0.
    pub from: usize,
    /// Its position after the move.
    pub to: usize,
}

impl TreeMove {
    /// Moves the sibling at `from` so that its position becomes `to`, the list the application
    /// keeps for [`parent`](Self::parent)'s children (or its top-level nodes). Positions out of
    /// range are ignored.
    pub fn apply<T>(&self, siblings: &mut Vec<T>) {
        let mut unused = 0;
        TabEdit::Move { from: self.from, to: self.to }.apply(siblings, &mut unused);
    }
}

/// How a drag lays the tree out: the dragged node folded (its children travel with it and would
/// only hide the siblings) and, while it is over a landing place, its siblings in the order the
/// drop would give.
pub(super) struct Arrange<'k> {
    pub(super) key: &'k str,
    pub(super) parent: Option<&'k str>,
    pub(super) order: Option<(usize, usize)>,
}

/// A pointer press on a row of a reorderable tree that may become a drag.
#[derive(Debug, Clone)]
struct Press {
    key: String,
    start: (i32, i32),
    pointer: (i32, i32),
    dragging: bool,
}

/// Pointer state of a reorderable tree, kept in its memory.
#[derive(Debug, Default)]
struct TreePointer {
    pressed: Option<Press>,
    /// Scrolling while the dragged node rests against the top or bottom row.
    edge: EdgeScroll,
}

/// A node being dragged, for painting.
pub(super) struct Drag {
    pub(super) key: String,
    pub(super) pointer: (i32, i32),
}

/// The node whose context menu is open, kept in the tree's memory.
#[derive(Debug, Default)]
struct MenuNode(Option<String>);

impl<Msg: 'static> Tree<Msg> {
    /// The siblings of `key` with its parent key: the parent's children, or the top-level nodes.
    pub(super) fn siblings(&self, key: &str) -> Option<(Option<&str>, &[TreeNode])> {
        fn find<'a>(
            nodes: &'a [TreeNode],
            parent: Option<&'a str>,
            key: &str,
        ) -> Option<(Option<&'a str>, &'a [TreeNode])> {
            if nodes.iter().any(|node| node.key == key) {
                return Some((parent, nodes));
            }
            nodes.iter().find_map(|node| find(&node.children, Some(&node.key), key))
        }
        find(&self.roots, None, key)
    }

    /// Asks to move `key` from sibling position `from` to `to`.
    fn move_node(&self, cx: &mut EventCx<'_, Msg>, key: &str, parent: Option<&str>, from: usize, to: usize) {
        if from != to
            && let Some(message) = &self.on_move
        {
            cx.emit(message(TreeMove { key: key.to_owned(), parent: parent.map(str::to_owned), from, to }));
        }
    }

    /// Ctrl+Shift+↑/↓ move the selected node one place among its siblings; true when used.
    pub(super) fn move_key(&self, cx: &mut EventCx<'_, Msg>, key: &KeyEvent) -> bool {
        let mods = key.chord.mods;
        let up = key.chord.key == Key::Up;
        if self.on_move.is_none() || !mods.ctrl || !mods.shift || mods.alt || !(up || key.chord.key == Key::Down) {
            return false;
        }
        let Some(selected) = self.selected.as_deref() else {
            return true;
        };
        let Some((parent, siblings)) = self.siblings(selected) else {
            return true;
        };
        let Some(from) = siblings.iter().position(|node| node.key == selected) else {
            return true;
        };
        let to = if up { from.saturating_sub(1) } else { (from + 1).min(siblings.len() - 1) };
        self.move_node(cx, selected, parent, from, to);
        true
    }

    /// The resting layout of a drag of `key`: the node folded, the siblings in their order.
    pub(super) fn resting(&self, key: &str) -> Vec<Flat<'_>> {
        let parent = self.siblings(key).and_then(|(parent, _)| parent);
        self.flatten_with(Some(&Arrange { key, parent, order: None }))
    }

    /// Where a drag of `key` with the pointer at `pointer` would land: the dragged node's sibling
    /// position, the landing position and the parent, computed on the resting layout so the target
    /// stays put while the siblings make room.
    pub(super) fn landing(&self, key: &str, pointer: (i32, i32), area: Rect, offset: usize) -> Option<(usize, usize)> {
        let flat = self.resting(key);
        let row = flat.iter().position(|row| row.node.key == key)?;
        let parent = flat[row].parent;
        let from = flat[row].index;
        let visible = usize::from(area.height);
        let slots: Vec<(usize, Rect)> = flat
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible)
            .filter(|(_, other)| other.parent == parent)
            .map(|(at, other)| {
                let y = area.y + i32::try_from(at - offset).unwrap_or(0);
                (other.index, Rect::new(area.x, y, area.width, 1))
            })
            .collect();
        Some((from, drop_target(&slots, from, pointer, Direction::Down)))
    }

    /// The node being dragged right now, if any.
    pub(super) fn drag(&self, cx: &mut PaintCx<'_>) -> Option<Drag> {
        self.on_move.as_ref()?;
        let press = cx.memory::<TreePointer>().pressed.clone()?;
        (press.dragging && self.siblings(&press.key).is_some())
            .then_some(Drag { key: press.key, pointer: press.pointer })
    }

    /// Pointer input of a reorderable tree on row `index` of `flat` (the row under the pointer, if
    /// any). A press selects the row and holds it; moving a row's height makes it a drag, and the
    /// release drops it among its siblings. A press and release without a drag opens, closes or
    /// activates the row like Enter. Returns `None` when the event is not the tree's to take.
    pub(super) fn reorder_pointer(
        &self,
        cx: &mut EventCx<'_, Msg>,
        mouse: &MouseEvent,
        flat: &[Flat<'_>],
        index: Option<usize>,
    ) -> Option<bool> {
        let at = (mouse.x, mouse.y);
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) => {
                let row = &flat[index?];
                self.select(cx, flat, index?);
                cx.capture_pointer();
                let press = Press { key: row.node.key.clone(), start: at, pointer: at, dragging: false };
                *cx.memory::<TreePointer>() = TreePointer { pressed: Some(press), edge: EdgeScroll::default() };
                Some(true)
            }
            MouseKind::Drag(MouseButton::Left) => {
                let memory = cx.memory::<TreePointer>();
                let press = memory.pressed.as_mut()?;
                press.pointer = at;
                if (at.1 - press.start.1).abs() >= 1 {
                    press.dragging = true;
                }
                let dragging = press.dragging;
                self.edge_scroll(cx, mouse.y, flat.len(), dragging);
                Some(true)
            }
            MouseKind::Up(MouseButton::Left) => {
                let memory = cx.memory::<TreePointer>();
                memory.edge = EdgeScroll::default();
                let press = memory.pressed.take()?;
                let area = cx.area();
                if press.dragging {
                    let offset = cx.memory::<RowScroll>().offset;
                    if let Some((from, to)) = self.landing(&press.key, at, Self::rows_area(area, flat.len()), offset) {
                        let parent = self.siblings(&press.key).and_then(|(parent, _)| parent.map(str::to_owned));
                        self.move_node(cx, &press.key, parent.as_deref(), from, to);
                    }
                } else if let Some(row) = flat.iter().position(|row| row.node.key == press.key) {
                    self.open_or_activate(cx, row, flat[row].node);
                }
                Some(true)
            }
            _ => None,
        }
    }

    /// The rows of `area` with `total` rows: the scrollbar column is not a landing place.
    pub(super) fn rows_area(area: Rect, total: usize) -> Rect {
        let overflows = total > usize::from(area.height);
        Rect::new(area.x, area.y, area.width.saturating_sub(u16::from(overflows)), area.height)
    }

    /// Scrolls one row while a dragged node rests on the top or bottom row or past them.
    fn edge_scroll(&self, cx: &mut EventCx<'_, Msg>, y: i32, total: usize, dragging: bool) {
        let area = cx.area();
        let last = area.bottom() - 1;
        let zone = if y <= area.y {
            Some(Zone { edge: Edge::Back, beyond: u16::try_from(area.y - y).unwrap_or(u16::MAX) })
        } else if y >= last {
            Some(Zone { edge: Edge::Forward, beyond: u16::try_from(y - last).unwrap_or(u16::MAX) })
        } else {
            None
        };
        let visible = usize::from(area.height);
        let mut edge = cx.memory::<TreePointer>().edge;
        edge.drive(cx, zone.filter(|_| dragging), |cx, edge| {
            let memory = cx.memory::<RowScroll>();
            memory.offset = match edge {
                Edge::Back => memory.offset.checked_sub(1)?,
                Edge::Forward => Some(memory.offset + 1).filter(|next| *next + visible <= total)?,
            };
            Some(memory.offset)
        });
        cx.memory::<TreePointer>().edge = edge;
    }

    /// The context menu of `key` with its messages replaced by positions, and the messages.
    fn menu_for(&self, key: &str) -> Option<(ContextMenu<usize>, Vec<Msg>)> {
        let items = self.menu.as_ref()?;
        let (items, messages) = context_item::keyed(items(key));
        Some((ContextMenu::new(items), messages))
    }

    fn open_menu(&self, cx: &mut EventCx<'_, Msg>, key: &str, anchor: Rect, keyboard: bool) {
        let Some((menu, _)) = self.menu_for(key) else {
            return;
        };
        cx.memory::<MenuNode>().0 = Some(key.to_owned());
        cx.with_messages(|cx: &mut EventCx<'_, usize>| menu.open(cx, anchor, keyboard));
    }

    /// Offers `event` to the context menu, when the tree has one; true when the menu used it.
    ///
    /// A right press on a row opens the menu of that node at the pointer; the menu key or
    /// Shift+F10 opens the menu of the selected node below its row, scrolling it into view first.
    /// While the menu is open it takes the keys, the wheel and presses on itself and sends the
    /// message of the chosen entry; a left press anywhere else closes it and is not used, so the
    /// tree still acts on it.
    pub(super) fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event, flat: &[Flat<'_>]) -> bool {
        if self.menu.is_none() || flat.is_empty() {
            return false;
        }
        let right_press = match event {
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Right) => Some((mouse.x, mouse.y)),
            _ => None,
        };
        if context_menu::is_open_in(cx) {
            let node = cx.memory::<MenuNode>().0.clone().filter(|key| flat.iter().any(|row| row.node.key == *key));
            let elsewhere = right_press.is_some_and(|(x, y)| !context_menu::contains(cx, x, y));
            // A right press beside the menu, or a node that went away, closes it; the press may open
            // the menu of another node below.
            match node.and_then(|key| self.menu_for(&key)).filter(|_| !elsewhere) {
                Some((menu, messages)) => {
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
        let area = cx.area();
        let visible = usize::from(area.height);
        match event {
            Event::Mouse(_) => {
                let Some((x, y)) = right_press else {
                    return false;
                };
                let offset = cx.memory::<RowScroll>().offset;
                let row = usize::try_from(y - area.y).ok().map(|r| offset + r).filter(|i| *i < flat.len());
                let Some(row) = row.filter(|_| x < Self::rows_area(area, flat.len()).right()) else {
                    return false;
                };
                self.open_menu(cx, &flat[row].node.key, Rect::new(x, y, 1, 1), false);
                true
            }
            Event::Key(key) if context_menu::is_menu_key(key) => {
                let Some(row) = self.selected_index(flat) else {
                    return false;
                };
                let memory = cx.memory::<RowScroll>();
                if row < memory.offset {
                    memory.offset = row;
                } else if visible > 0 && row >= memory.offset + visible {
                    memory.offset = row + 1 - visible;
                }
                let y = area.y + i32::try_from(row - memory.offset).unwrap_or(0);
                let anchor = Rect::new(area.x, y, Self::rows_area(area, flat.len()).width, 1);
                self.open_menu(cx, &flat[row].node.key, anchor, true);
                true
            }
            _ => false,
        }
    }

    /// The node whose context menu is open in the tree being painted; its row stays raised as if
    /// hovered, so it is clear what the menu acts on.
    pub(super) fn menu_node(&self, cx: &mut PaintCx<'_>) -> Option<String> {
        if self.menu.is_none() || !context_menu::is_open(cx) {
            return None;
        }
        cx.memory::<MenuNode>().0.clone()
    }

    /// Paints the open context menu; called from `paint_overlay`.
    pub(super) fn paint_menu(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        if let Some((menu, _)) = self.menu_node(cx).and_then(|key| self.menu_for(&key)) {
            menu.paint_overlay(cx, anchor);
        }
    }
}
