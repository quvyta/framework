//! The behaviour every tab view shares: opening, closing and reordering tabs with keys and the
//! mouse, and the context menu of a tab. [`Tabs`](super::Tabs) lays tabs out across,
//! [`TabRail`](super::TabRail) down; both keep their options and input handling here, so a
//! closable, reorderable tab with a menu behaves the same in either.

use crate::event::{Event, KeyEvent, MouseButton, MouseEvent, MouseKind};
use crate::geometry::Rect;
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::widget::{EventCx, PaintCx, Widget};

use super::IndexMessage;
use super::context_item::{self, ContextItem};
use super::context_menu::{self, ContextMenu};
use super::edge_scroll::{Edge, EdgeScroll, Zone};

/// Builds a message from a tab's old and new index.
type MoveMessage<Msg> = Box<dyn Fn(usize, usize) -> Msg>;

/// Builds the context menu entries of a tab from its index.
type MenuItems<Msg> = Box<dyn Fn(usize) -> Vec<ContextItem<Msg>>>;

/// A change a closable or reorderable tab view asks for. Tab views never change the
/// application's tabs themselves; [`TabEdit::apply`] does the bookkeeping on the application's
/// own list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabEdit {
    /// Tab `index` is closed.
    Close(usize),
    /// The tab at `from` moves so that its index becomes `to`.
    Move {
        /// The tab's index before the move.
        from: usize,
        /// The tab's index after the move.
        to: usize,
    },
}

impl TabEdit {
    /// Applies the edit to `tabs` and keeps `active` on the same tab. Closing the open tab opens
    /// the tab that took its place, or the new last tab when it was the last one. Indices out of
    /// range are ignored.
    pub fn apply<T>(self, tabs: &mut Vec<T>, active: &mut usize) {
        match self {
            Self::Close(index) => {
                if index >= tabs.len() {
                    return;
                }
                tabs.remove(index);
                if index < *active {
                    *active -= 1;
                }
                *active = (*active).min(tabs.len().saturating_sub(1));
            }
            Self::Move { from, to } => {
                if from >= tabs.len() || to >= tabs.len() || from == to {
                    return;
                }
                let tab = tabs.remove(from);
                tabs.insert(to, tab);
                *active = if *active == from {
                    to
                } else if from < *active && to >= *active {
                    *active - 1
                } else if from > *active && to <= *active {
                    *active + 1
                } else {
                    *active
                };
            }
        }
    }
}

/// The axis tabs are laid out along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Side by side, like [`Tabs`](super::Tabs).
    Across,
    /// Stacked, like [`TabRail`](super::TabRail).
    Down,
}

/// What the pointer is over in a tab view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabHit {
    /// Tab `index`, painted in `rect`.
    Tab(usize, Rect),
    /// The close mark of tab `index`.
    Close(usize),
}

/// A pointer press on a tab that may become a drag or a close.
#[derive(Debug, Clone, Copy)]
struct Pressed {
    index: usize,
    start: (i32, i32),
    pointer: (i32, i32),
    grab: (i32, i32),
    dragging: bool,
    close: bool,
}

/// Pointer state of a tab view, kept in its memory.
#[derive(Debug, Default)]
struct TabPointer {
    pressed: Option<Pressed>,
    /// Scrolling while the dragged tab rests against an end of the view.
    edge: EdgeScroll,
}

/// A tab being dragged, for painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Drag {
    /// The dragged tab.
    pub(crate) index: usize,
    /// The pointer cell.
    pub(crate) pointer: (i32, i32),
    /// Where in the tab the pointer grabbed it, relative to its top-left cell.
    pub(crate) grab: (i32, i32),
}

/// Options and messages of a tab view, and the input handling built on them.
pub(crate) struct TabModel<Msg> {
    pub(crate) count: usize,
    pub(crate) active: usize,
    pub(crate) pinned: Vec<usize>,
    pub(crate) on_select: Option<IndexMessage<Msg>>,
    pub(crate) on_close: Option<IndexMessage<Msg>>,
    pub(crate) on_move: Option<MoveMessage<Msg>>,
    /// Message for each step a dragged tab scrolls the view, with the first position in view.
    pub(crate) on_drag_scroll: Option<IndexMessage<Msg>>,
    menu: Option<MenuItems<Msg>>,
}

/// The tab whose context menu is open, kept in the view's memory.
#[derive(Debug, Default)]
struct MenuTab(usize);

impl<Msg> TabModel<Msg> {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            count,
            active: 0,
            pinned: Vec::new(),
            on_select: None,
            on_close: None,
            on_move: None,
            on_drag_scroll: None,
            menu: None,
        }
    }

    pub(crate) fn set_context_menu(&mut self, items: impl Fn(usize) -> Vec<ContextItem<Msg>> + 'static) {
        self.menu = Some(Box::new(items));
    }

    /// The context menu of tab `index` with its messages replaced by positions, and the messages.
    fn menu_for(&self, index: usize) -> Option<(ContextMenu<usize>, Vec<Msg>)> {
        let items = self.menu.as_ref()?;
        let (items, messages) = context_item::keyed(items(index));
        Some((ContextMenu::new(items), messages))
    }

    /// Whether the view has a context menu.
    pub(crate) fn has_menu(&self) -> bool {
        self.menu.is_some()
    }

    /// Opens the context menu of tab `index` below `anchor`; from the keyboard its first entry is
    /// highlighted.
    fn open_menu(&self, cx: &mut EventCx<'_, Msg>, index: usize, anchor: Rect, keyboard: bool) {
        let Some((menu, _)) = self.menu_for(index) else {
            return;
        };
        cx.memory::<MenuTab>().0 = index;
        cx.with_messages(|cx: &mut EventCx<'_, usize>| menu.open(cx, anchor, keyboard));
    }

    /// Offers `event` to the context menu, when the view has one; true when the menu used it.
    ///
    /// A right press on a tab opens the menu of that tab at the pointer, the menu key or
    /// shift+F10 opens the menu of the open tab below `open_tab` (its rect on screen, if shown).
    /// While the menu is open it takes the keys, the wheel and presses on itself, and sends the
    /// message of the chosen entry; a left press anywhere else closes it and is not used, so the
    /// view still acts on it. `hit` is the tab under the pointer.
    pub(crate) fn menu_event(
        &self,
        cx: &mut EventCx<'_, Msg>,
        event: &Event,
        hit: Option<TabHit>,
        open_tab: Option<Rect>,
    ) -> bool {
        if self.menu.is_none() || self.count == 0 {
            return false;
        }
        let right_press = match event {
            Event::Mouse(mouse) if mouse.kind == MouseKind::Down(MouseButton::Right) => Some((mouse.x, mouse.y)),
            _ => None,
        };
        if context_menu::is_open_in(cx) {
            let tab = cx.memory::<MenuTab>().0;
            let elsewhere = right_press.is_some_and(|(x, y)| !context_menu::contains(cx, x, y));
            // A right press beside the menu, or a tab that went away, closes it; the press may open
            // the menu of another tab below.
            if elsewhere || tab >= self.count {
                cx.with_messages(|cx: &mut EventCx<'_, usize>| ContextMenu::<usize>::close(cx));
            } else if let Some((menu, messages)) = self.menu_for(tab) {
                let (used, chosen) = cx.with_messages(|cx| menu.event(cx, event));
                if let Some(message) = chosen.last().and_then(|chosen| messages.into_iter().nth(*chosen)) {
                    cx.emit(message);
                }
                return used;
            }
        }
        match event {
            Event::Mouse(_) => {
                let Some((x, y)) = right_press else {
                    return false;
                };
                if let Some(TabHit::Tab(index, _) | TabHit::Close(index)) = hit {
                    self.open_menu(cx, index, Rect::new(x, y, 1, 1), false);
                }
                true
            }
            Event::Key(key) if context_menu::is_menu_key(key) => {
                let area = cx.area();
                let anchor = open_tab.unwrap_or(Rect::new(area.x, area.y, 1, 1));
                self.open_menu(cx, self.active(), anchor, true);
                true
            }
            _ => false,
        }
    }

    /// Paints the open context menu; call it from `paint_overlay`. True when it painted one.
    pub(crate) fn paint_menu(&self, cx: &mut PaintCx<'_>, anchor: Rect) -> bool {
        match self.menu_tab(cx).and_then(|tab| self.menu_for(tab)) {
            Some((menu, _)) => {
                menu.paint_overlay(cx, anchor);
                true
            }
            None => false,
        }
    }

    /// The tab whose context menu is open in the view being painted. The view keeps that tab
    /// raised as if hovered, so it is clear what the menu acts on.
    pub(crate) fn menu_tab(&self, cx: &mut PaintCx<'_>) -> Option<usize> {
        if self.menu.is_none() || !context_menu::is_open(cx) {
            return None;
        }
        Some(cx.memory::<MenuTab>().0).filter(|tab| *tab < self.count)
    }

    pub(crate) fn set_on_close(&mut self, message: impl Fn(usize) -> Msg + 'static) {
        self.on_close = Some(Box::new(message));
    }

    pub(crate) fn set_on_move(&mut self, message: impl Fn(usize, usize) -> Msg + 'static) {
        self.on_move = Some(Box::new(message));
    }

    /// The open tab, within range.
    pub(crate) fn active(&self) -> usize {
        self.active.min(self.count.saturating_sub(1))
    }

    /// Whether tab `index` shows a close mark and can be closed.
    pub(crate) fn closable(&self, index: usize) -> bool {
        self.on_close.is_some() && !self.pinned.contains(&index)
    }

    pub(crate) fn reorderable(&self) -> bool {
        self.on_move.is_some()
    }

    /// Opens tab `index`; true when it exists.
    pub(crate) fn open(&self, cx: &mut EventCx<'_, Msg>, index: usize) -> bool {
        if index >= self.count {
            return false;
        }
        if index != self.active
            && let Some(message) = &self.on_select
        {
            cx.emit(message(index));
        }
        true
    }

    /// Closes tab `index` when it can be closed.
    pub(crate) fn close(&self, cx: &mut EventCx<'_, Msg>, index: usize) -> bool {
        if index >= self.count || !self.closable(index) {
            return false;
        }
        if let Some(message) = &self.on_close {
            cx.emit(message(index));
        }
        true
    }

    fn move_tab(&self, cx: &mut EventCx<'_, Msg>, from: usize, to: usize) {
        if from != to
            && to < self.count
            && let Some(message) = &self.on_move
        {
            cx.emit(message(from, to));
        }
    }

    /// Keys every tab view shares: the arrows along `direction` (and h/l or k/j) open the
    /// neighbour; with closing on, ctrl+w closes the open tab; with reordering on, ctrl+shift
    /// and the arrows move it.
    pub(crate) fn key(&self, cx: &mut EventCx<'_, Msg>, key: &KeyEvent, direction: Direction) -> bool {
        let (back, forward, back_letter, forward_letter) = match direction {
            Direction::Across => (Key::Left, Key::Right, 'h', 'l'),
            Direction::Down => (Key::Up, Key::Down, 'k', 'j'),
        };
        let active = self.active();
        if key.is_plain(back) || key.is_plain(Key::Char(back_letter)) {
            return active > 0 && self.open(cx, active - 1);
        }
        if key.is_plain(forward) || key.is_plain(Key::Char(forward_letter)) {
            return self.open(cx, active + 1);
        }
        let mods = key.chord.mods;
        if self.on_close.is_some() && mods.ctrl && !mods.shift && !mods.alt && key.chord.key == Key::Char('w') {
            self.close(cx, active);
            return true;
        }
        if self.reorderable() && mods.ctrl && mods.shift && !mods.alt {
            if key.chord.key == back {
                if active > 0 {
                    self.move_tab(cx, active, active - 1);
                }
                return true;
            }
            if key.chord.key == forward {
                self.move_tab(cx, active, active + 1);
                return true;
            }
        }
        false
    }

    /// Pointer input. `hit` is what the pointer is over; `slots` are the tabs on screen in
    /// their resting order, used to find where a dragged tab lands.
    pub(crate) fn pointer(
        &self,
        cx: &mut EventCx<'_, Msg>,
        mouse: &MouseEvent,
        hit: Option<TabHit>,
        slots: &[(usize, Rect)],
        direction: Direction,
    ) -> bool {
        let at = (mouse.x, mouse.y);
        match mouse.kind {
            MouseKind::Down(MouseButton::Middle) => match hit {
                Some(TabHit::Tab(index, _) | TabHit::Close(index)) if self.on_close.is_some() => {
                    self.close(cx, index);
                    true
                }
                _ => false,
            },
            MouseKind::Down(MouseButton::Left) => match hit {
                Some(TabHit::Close(index)) => {
                    cx.capture_pointer();
                    let press = Pressed { index, start: at, pointer: at, grab: (0, 0), dragging: false, close: true };
                    *cx.memory::<TabPointer>() = TabPointer { pressed: Some(press), edge: EdgeScroll::default() };
                    true
                }
                Some(TabHit::Tab(index, rect)) => {
                    self.open(cx, index);
                    if self.reorderable() {
                        cx.capture_pointer();
                        let grab = (at.0 - rect.x, at.1 - rect.y);
                        let press = Pressed { index, start: at, pointer: at, grab, dragging: false, close: false };
                        *cx.memory::<TabPointer>() = TabPointer { pressed: Some(press), edge: EdgeScroll::default() };
                    }
                    true
                }
                None => false,
            },
            MouseKind::Drag(MouseButton::Left) => {
                let memory = cx.memory::<TabPointer>();
                let Some(press) = &mut memory.pressed else {
                    return false;
                };
                press.pointer = at;
                let travel = match direction {
                    Direction::Across => (at.0 - press.start.0).abs() >= 2,
                    Direction::Down => (at.1 - press.start.1).abs() >= 1,
                };
                if !press.close && travel {
                    press.dragging = true;
                }
                true
            }
            MouseKind::Up(MouseButton::Left) => {
                let memory = cx.memory::<TabPointer>();
                memory.edge = EdgeScroll::default();
                let Some(press) = memory.pressed.take() else {
                    return false;
                };
                if press.close {
                    if hit == Some(TabHit::Close(press.index)) {
                        self.close(cx, press.index);
                    }
                } else if press.dragging {
                    let to = drop_target(slots, press.index, at, direction);
                    self.move_tab(cx, press.index, to);
                }
                true
            }
            _ => false,
        }
    }

    /// Scrolls the view while a dragged tab rests against one of its ends; call it with every left
    /// drag, after [`pointer`](Self::pointer). `zone` is the end the pointer is on, if any; it only
    /// counts while a tab is being dragged, so a press held on a tab or on its close mark never
    /// scrolls. `step` scrolls the view one tab towards an end and returns the first position now in
    /// view, or none at that end. Each step sends the drag-scroll message.
    pub(crate) fn edge_scroll(
        &self,
        cx: &mut EventCx<'_, Msg>,
        zone: Option<Zone>,
        step: impl FnOnce(&mut EventCx<'_, Msg>, Edge) -> Option<usize>,
    ) {
        let memory = cx.memory::<TabPointer>();
        let dragging = memory.pressed.is_some_and(|press| press.dragging);
        let mut edge = memory.edge;
        let first = edge.drive(cx, zone.filter(|_| dragging), step);
        cx.memory::<TabPointer>().edge = edge;
        if let (Some(first), Some(message)) = (first, &self.on_drag_scroll) {
            cx.emit(message(first));
        }
    }

    /// The tab being dragged right now, if any.
    pub(crate) fn drag(&self, cx: &mut PaintCx<'_>) -> Option<Drag> {
        let press = cx.memory::<TabPointer>().pressed?;
        (press.dragging && press.index < self.count).then_some(Drag {
            index: press.index,
            pointer: press.pointer,
            grab: press.grab,
        })
    }
}

/// Paints the tinted slot where a dragged tab will land, in `tab-drop`.
pub(crate) fn paint_drop_slot(cx: &mut PaintCx<'_>, rect: Rect) {
    let drop = cx.style("tab-drop", None, &[]).text();
    cx.clear(rect, drop.bg.unwrap_or_else(|| cx.color("raised")));
}

/// Paints the surface of the ghost that follows the pointer while a tab is dragged, in `tab-ghost`,
/// and returns that style for the ghost's text.
pub(crate) fn paint_ghost_surface(cx: &mut PaintCx<'_>, rect: Rect) -> CellStyle {
    let ghost = cx.style("tab-ghost", None, &[]).text();
    cx.clear(rect, ghost.bg.unwrap_or_else(|| cx.color("active")));
    ghost
}

/// The index a tab dragged from `from` gets when dropped at `pointer`: the index of the resting
/// tab under the pointer, or of the nearest one on screen. Computing it from the resting layout
/// keeps the target steady while the preview moves tabs around.
pub(crate) fn drop_target(slots: &[(usize, Rect)], from: usize, pointer: (i32, i32), direction: Direction) -> usize {
    let along = |rect: &Rect| match direction {
        Direction::Across => rect.x,
        Direction::Down => rect.y,
    };
    let position = match direction {
        Direction::Across => pointer.0,
        Direction::Down => pointer.1,
    };
    let Some((first, _)) = slots.first() else {
        return from;
    };
    slots.iter().take_while(|(_, rect)| along(rect) <= position).last().map_or(*first, |(index, _)| *index)
}

/// The order tabs are shown in while `drag` would drop tab `from` at `to`.
pub(crate) fn preview_order(count: usize, drag: Option<(usize, usize)>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    if let Some((from, to)) = drag
        && from < count
        && to < count
    {
        let tab = order.remove(from);
        order.insert(to, tab);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_the_open_tab() {
        let mut tabs = vec!["a", "b", "c", "d"];
        let mut active = 2;
        TabEdit::Close(0).apply(&mut tabs, &mut active);
        assert_eq!((tabs.as_slice(), active), (&["b", "c", "d"][..], 1));
        TabEdit::Close(1).apply(&mut tabs, &mut active);
        assert_eq!((tabs.as_slice(), active), (&["b", "d"][..], 1));
        TabEdit::Close(1).apply(&mut tabs, &mut active);
        assert_eq!((tabs.as_slice(), active), (&["b"][..], 0));

        let mut tabs = vec!["a", "b", "c", "d"];
        let mut active = 1;
        TabEdit::Move { from: 1, to: 3 }.apply(&mut tabs, &mut active);
        assert_eq!((tabs.as_slice(), active), (&["a", "c", "d", "b"][..], 3));
        TabEdit::Move { from: 0, to: 3 }.apply(&mut tabs, &mut active);
        assert_eq!((tabs.as_slice(), active), (&["c", "d", "b", "a"][..], 2));
        TabEdit::Move { from: 3, to: 0 }.apply(&mut tabs, &mut active);
        assert_eq!((tabs.as_slice(), active), (&["a", "c", "d", "b"][..], 3));
        TabEdit::Move { from: 9, to: 0 }.apply(&mut tabs, &mut active);
        assert_eq!(active, 3);
    }

    #[test]
    fn drop_target_is_the_resting_tab_under_the_pointer() {
        let slots = [(2, Rect::new(4, 0, 6, 1)), (3, Rect::new(11, 0, 9, 1)), (4, Rect::new(21, 0, 5, 1))];
        assert_eq!(drop_target(&slots, 3, (0, 0), Direction::Across), 2);
        assert_eq!(drop_target(&slots, 2, (12, 0), Direction::Across), 3);
        assert_eq!(drop_target(&slots, 2, (40, 0), Direction::Across), 4);
        assert_eq!(preview_order(4, Some((0, 2))), vec![1, 2, 0, 3]);
    }
}
