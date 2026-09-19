//! Tab strips.
//!
//! [`Tabs`] and its options live here; `strip` lays the tabs out, `paint` draws them and the
//! shared behaviour of every tab view (opening, closing, reordering) is in
//! [`tab_model`](super::tab_model).

mod add;
#[cfg(test)]
mod add_tests;
mod paint;
mod strip;
#[cfg(test)]
mod tests;

use std::time::Duration;

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::{Key, Modifiers};
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::close_mark;
use super::context_item::ContextItem;
use super::context_menu;
use super::edge_scroll::{Edge, Zone};
use super::popup_menu::{PopupAction, PopupMenu};
use super::tab_model::{self, Direction, TabModel, drop_target, preview_order};
use strip::Strip;

/// How wide each tab of a [`Tabs`] strip is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabWidth {
    /// As wide as its label. The default.
    #[default]
    Fit,
    /// Exactly this many cells; longer labels end in `…`.
    Fixed(u16),
    /// The tabs share the strip's width equally; when there are too many to fit, they keep a
    /// readable minimum and the strip overflows.
    Fill,
}

/// What a [`Tabs`] strip does when its tabs do not fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// Arrow controls at both ends scroll the strip one tab at a time, without opening anything.
    /// They are small buttons: hovered they brighten and raise the pillar, pressed they flash one
    /// tone brighter, and at the end of the strip they fade and ignore presses. ctrl+PgUp,
    /// ctrl+PgDn and the wheel scroll the same way, and opening a tab brings it into view. The
    /// default.
    #[default]
    Arrows,
    /// A control at the end shows how many tabs are hidden and opens a menu of them; ↓ opens it
    /// from the keyboard.
    Menu,
}

/// Width of the gap between tabs.
const GAP: u16 = 1;
/// Cells before a label.
const PAD: u16 = 2;
/// Extra cells a close mark takes: a gap after the label and the three cells of the mark, which
/// end where the right padding begins.
const CLOSE: u16 = 1 + close_mark::WIDTH - 1;
/// The narrowest tab `TabWidth::Fill` shrinks to before the strip overflows.
const FILL_MIN: u16 = 12;
/// Width of a scroll arrow: a space, the chevron and a space.
const ARROW: u16 = 3;
/// Width of the add button: a space, the `+` and a space, like an arrow.
const ADD: u16 = 3;

/// A row of tabs. The open tab is a raised surface; tabs are never boxed or bracketed.
///
/// With no options it is the plainest strip: keys while focused are ←/→ (or h/l) to open the
/// neighbouring tab, and 1–9 open a tab by number when numbers are shown. When the tabs do not
/// fit, arrow controls appear at both ends: a click scrolls one tab, and ctrl+PgUp, ctrl+PgDn
/// and the wheel do the same. Opening a tab brings it into view.
///
/// Capabilities are independent options:
/// - [`closable`](Self::closable): a faint `×` on every tab that brightens on hover; clicking
///   it, a middle click on the tab or ctrl+w closes the tab. [`pinned`](Self::pinned) tabs
///   cannot be closed.
/// - [`tab_width`](Self::tab_width): fit, fixed or filling tabs.
/// - [`overflow`](Self::overflow): scroll arrows or a menu of hidden tabs.
/// - [`reorderable`](Self::reorderable): drag a tab to move it; a ghost follows the pointer and
///   the tabs make room where it will land. ctrl+shift+←/→ moves the open tab. Held on a scroll
///   arrow or past an end of an overflowing strip, the dragged tab scrolls the strip: one tab after
///   400 ms, then one every 150 ms (sooner the further past the end) until the pointer leaves or the
///   strip ends; the arrow lights up meanwhile and the drop slot follows the tabs coming into view.
///   [`on_drag_scroll`](Self::on_drag_scroll) reports each step.
/// - [`context_menu`](Self::context_menu): a right click on a tab opens a menu of actions for
///   it at the pointer; the menu key or shift+F10 opens the menu of the open tab. Without it a
///   right click does nothing.
/// - [`on_add`](Self::on_add): a `+` button right after the last tab, or at the strip's right end
///   while tabs hide, that asks for a new tab. Tab from the tabs reaches it; Enter and Space press
///   it; resting on it, or reaching it with the keyboard, shows what it does. A tab dragged onto it
///   moves to the end.
///
/// Style keys: `tab` with `hover`, `selected`, `focus`; `tab-index` for numbers; `close-mark`
/// (with `active` on a raised tab, `hover` under the pointer); `tab-arrow` (`bg`, `fg`, `pillar`)
/// with `hover`, `pressed`, `disabled`; `tab-menu` (`bg`, `fg`, `pillar`) with `hover`, `active`
/// while its menu is open; `tab-ghost` and `tab-drop` (`bg`) while dragging; `popup-menu`,
/// `popup-item`, `popup-check` for the menu of hidden tabs; `tab-add` (`bg`, `fg`, `pillar`) with
/// `hover`, `focus`, `pressed` and `tooltip` for its hint; the keys of
/// [`ContextItem`](super::ContextItem) for the context menu.
pub struct Tabs<Msg> {
    labels: Vec<String>,
    numbered: bool,
    width: TabWidth,
    overflow: Overflow,
    on_add: Option<Box<dyn Fn() -> Msg>>,
    model: TabModel<Msg>,
}

#[derive(Debug, Default)]
struct TabsMemory {
    offset: usize,
    followed: Option<usize>,
    hidden: Vec<usize>,
    /// The arrow last pressed (by click or key) and when, for its flash.
    pressed: Option<(Arrow, Duration)>,
    /// Whether keyboard focus inside the strip is on the add button rather than the tabs.
    on_add: bool,
    /// When the add button was last pressed, for its flash.
    add_pressed: Option<Duration>,
    /// Since when the pointer rests on the add button, and since when its hint shows.
    add_hovered_since: Option<Duration>,
    add_hint_since: Option<Duration>,
    /// The add button while its hint shows, for the overlay.
    add_hint: Option<Rect>,
}

/// One of the two scroll arrows, named by the end of the strip it scrolls towards.
use Edge as Arrow;

impl<Msg: 'static> Tabs<Msg> {
    /// Tabs with `labels`; the first is open.
    #[must_use]
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
        let model = TabModel::new(labels.len());
        Self { labels, numbered: false, width: TabWidth::Fit, overflow: Overflow::Arrows, on_add: None, model }
    }

    /// The open tab.
    #[must_use]
    pub fn active(mut self, index: usize) -> Self {
        self.model.active = index;
        self
    }

    /// Shows 1, 2, 3… before the labels and lets number keys open tabs.
    #[must_use]
    pub fn numbered(mut self, numbered: bool) -> Self {
        self.numbered = numbered;
        self
    }

    /// Message for opening tab `index`.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.model.on_select = Some(Box::new(message));
        self
    }

    /// Makes tabs closable: `message(index)` asks the application to close tab `index`.
    /// [`TabEdit::Close`](super::TabEdit) applies it to the application's list.
    #[must_use]
    pub fn closable(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.model.set_on_close(message);
        self
    }

    /// Tabs that cannot be closed, such as a pinned start page. They show no close mark.
    #[must_use]
    pub fn pinned(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.model.pinned = indices.into_iter().collect();
        self
    }

    /// How wide the tabs are; [`TabWidth::Fit`] by default.
    #[must_use]
    pub fn tab_width(mut self, width: TabWidth) -> Self {
        self.width = width;
        self
    }

    /// What happens when the tabs do not fit; [`Overflow::Arrows`] by default.
    #[must_use]
    pub fn overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Makes tabs reorderable: `message(from, to)` asks the application to move a tab.
    /// [`TabEdit::Move`](super::TabEdit) applies it to the application's list.
    #[must_use]
    pub fn reorderable(mut self, message: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.model.set_on_move(message);
        self
    }

    /// Message for each step a dragged tab scrolls an overflowing strip, with the position of the
    /// first tab now in view (counted from 0), e.g. to log it. Only reorderable strips scroll this
    /// way.
    #[must_use]
    pub fn on_drag_scroll(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.model.on_drag_scroll = Some(Box::new(message));
        self
    }

    /// Adds a `+` button that sends `message` when pressed, e.g. to open a new tab. It stands one
    /// gap after the last tab; while tabs hide it keeps its place at the strip's right end, after
    /// the arrows or the menu control, and with no tabs it starts the strip. Tab moves keyboard
    /// focus from the tabs to it; Enter and Space press it.
    #[must_use]
    pub fn on_add(mut self, message: impl Fn() -> Msg + 'static) -> Self {
        self.on_add = Some(Box::new(message));
        self
    }

    /// Gives every tab a context menu: `items(index)` builds the entries for tab `index`, such as
    /// Close, Close others or Pin. A right click on a tab opens its menu at the pointer; the menu
    /// key or shift+F10 opens the menu of the open tab below it, scrolling the tab into view first.
    /// Choosing an entry sends its message.
    #[must_use]
    pub fn context_menu(mut self, items: impl Fn(usize) -> Vec<ContextItem<Msg>> + 'static) -> Self {
        self.model.set_context_menu(items);
        self
    }
}

impl<Msg: 'static> Widget<Msg> for Tabs<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let width = match self.width {
            TabWidth::Fill => available.width,
            TabWidth::Fit | TabWidth::Fixed(_) => {
                Self::total_width(&self.widths(available.width)).saturating_add(self.add_room())
            }
        };
        Size::new(width, 1).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        let on_add = self.add_focus_paint(cx);
        if self.labels.is_empty() {
            let strip = self.strip(area, &[], 0, &[]);
            let pointer = cx.pointer();
            self.paint_add(cx, area, &strip, pointer, on_add);
            return;
        }
        let focused = cx.is_focus_visible() && !on_add;
        let pointer = cx.pointer();
        let widths = self.widths(area.width);
        let active = self.model.active();
        let drag = self.model.drag(cx);
        let identity = self.identity();

        // Scroll to the open tab: always with a menu, only when it changed with arrows, so the
        // arrows can look around without being pulled back.
        let offset = {
            let memory = cx.memory::<TabsMemory>();
            let (offset, followed) = (memory.offset, memory.followed);
            let follows = drag.is_none() && (self.overflow == Overflow::Menu || followed != Some(active));
            let offset = if follows {
                self.follow(area, &identity, offset, active, &widths)
            } else {
                offset.min(self.labels.len() - 1)
            };
            let offset = if drag.is_none() { self.settle(area, &identity, offset, &widths) } else { offset };
            let memory = cx.memory::<TabsMemory>();
            memory.offset = offset;
            memory.followed = Some(active);
            offset
        };

        let resting = self.strip(area, &identity, offset, &widths);
        let slots = self.drop_slots(&resting);
        let target = drag.map(|d| (d.index, drop_target(&slots, d.index, d.pointer, Direction::Across)));
        let order = preview_order(self.labels.len(), target);
        let strip = if target.is_some() { self.strip(area, &order, offset, &widths) } else { resting };

        // An open context menu takes the pointer; only the tab it acts on stays raised.
        let menu_tab = self.model.menu_tab(cx);
        if menu_tab.is_some() {
            cx.request_overlay(area);
        }
        let pointer = pointer.filter(|_| menu_tab.is_none());

        for (position, (index, rect)) in strip.tabs.iter().enumerate() {
            if drag.is_some_and(|d| d.index == *index) {
                tab_model::paint_drop_slot(cx, *rect);
                continue;
            }
            let mut states = Vec::new();
            if menu_tab == Some(*index) || (drag.is_none() && pointer.is_some_and(|(x, y)| rect.contains(x, y))) {
                states.push(State::Hover);
            }
            if *index == active {
                states.push(State::Selected);
                if focused {
                    states.push(State::Focus);
                }
            }
            self.paint_tab(cx, *rect, *index, offset + position, &states);
        }

        // The arrows and the menu control take no hover while a tab is dragged, except the arrow
        // of the end the dragged tab is held against, which lights up as it scrolls the strip.
        let control_pointer = drag.is_none().then_some(pointer).flatten();
        let held = drag.and_then(|drag| self.drag_zone(area, &strip, drag.pointer.0)).map(|zone| zone.edge);
        match self.overflow {
            Overflow::Arrows => self.paint_arrows(cx, &strip, offset, control_pointer, held),
            Overflow::Menu => self.paint_menu_control(cx, &strip, control_pointer),
        }
        self.paint_add(cx, area, &strip, control_pointer, on_add);

        if let Some(drag) = drag {
            self.paint_ghost(cx, area, &strip, drag, widths[drag.index], &order);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        if self.model.paint_menu(cx, anchor) {
            return;
        }
        if !PopupMenu::is_open_paint(cx) {
            self.paint_add_hint(cx);
            return;
        }
        let hidden = cx.memory::<TabsMemory>().hidden.clone();
        let labels: Vec<String> = hidden.iter().map(|i| self.labels[*i].clone()).collect();
        // The strip keeps the open tab in view whenever it has room for a tab at all; a strip with
        // room only for the control lists every tab and checks the open one.
        let current = hidden.iter().position(|i| *i == self.model.active());
        PopupMenu::paint(cx, anchor, &labels, current);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let area = cx.area();
        if self.labels.is_empty() {
            return self.add_event(cx, event, &self.strip(area, &[], 0, &[]));
        }
        let widths = self.widths(area.width);
        let identity = self.identity();
        let mut offset = cx.memory::<TabsMemory>().offset;
        // The menu key opens the open tab's menu below that tab, so it first scrolls the tab back
        // into view when the arrows moved it away.
        if let Event::Key(key) = event
            && self.model.has_menu()
            && context_menu::is_menu_key(key)
            && !context_menu::is_open_in(cx)
        {
            offset = self.follow(area, &identity, offset, self.model.active(), &widths);
            cx.memory::<TabsMemory>().offset = offset;
        }
        let strip = self.strip(area, &identity, offset, &widths);
        if self.overflow == Overflow::Menu && PopupMenu::is_open(cx) {
            let hidden = cx.memory::<TabsMemory>().hidden.clone();
            let labels: Vec<String> = hidden.iter().map(|i| self.labels[*i].clone()).collect();
            match PopupMenu::event(cx, event, &labels) {
                PopupAction::Chosen(row) => {
                    self.model.open(cx, hidden[row]);
                    return true;
                }
                // A press elsewhere on the strip closes the menu and still does what it pressed,
                // so one click on a tab both closes the menu and opens the tab. A press on the
                // control that opened the menu only closes it.
                PopupAction::Closed if Self::presses_beside(event, strip.menu) => {}
                PopupAction::Used | PopupAction::Closed => return true,
                PopupAction::Ignored => {}
            }
        }
        let hit = match event {
            Event::Mouse(mouse) => self.hit(&strip, mouse.x, mouse.y),
            _ => None,
        };
        let open_tab = strip.tabs.iter().find(|(index, _)| *index == self.model.active()).map(|(_, rect)| *rect);
        if self.model.menu_event(cx, event, hit, open_tab) {
            return true;
        }
        if self.add_event(cx, event, &strip) {
            return true;
        }
        match event {
            Event::Key(key) => {
                if self.model.key(cx, key, Direction::Across) {
                    return true;
                }
                if self.numbered
                    && let Key::Char(c @ '1'..='9') = key.chord.key
                    && key.chord.mods == Modifiers::default()
                {
                    let index = usize::try_from(u32::from(c) - u32::from('1')).unwrap_or(usize::MAX);
                    return self.model.open(cx, index);
                }
                let ctrl = Modifiers { ctrl: true, ..Modifiers::default() };
                // Scrolling keys only mean something while tabs are hidden; a strip that fits
                // leaves ctrl+PgUp and ctrl+PgDn to the application.
                if self.scrolls(&strip) && key.chord.mods == ctrl {
                    let arrow = match key.chord.key {
                        Key::PageUp => Arrow::Back,
                        Key::PageDown => Arrow::Forward,
                        _ => return false,
                    };
                    self.scroll(cx, &strip, arrow, true);
                    return true;
                }
                if strip.menu.is_some() && key.is_plain(Key::Down) {
                    self.open_menu(cx, &strip);
                    return true;
                }
                false
            }
            Event::Mouse(mouse) => {
                let left_down = mouse.kind == MouseKind::Down(MouseButton::Left);
                // Presses on the arrows and the menu control are theirs; drags and releases pass on,
                // so a tab dragged over an arrow still lands.
                let down = matches!(mouse.kind, MouseKind::Down(_));
                if down && let Some(arrow) = strip.arrow_at(mouse.x, mouse.y) {
                    if left_down {
                        self.scroll(cx, &strip, arrow, true);
                    }
                    return true;
                }
                if self.scrolls(&strip) && matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) {
                    let arrow = if mouse.kind == MouseKind::ScrollUp { Arrow::Back } else { Arrow::Forward };
                    self.scroll(cx, &strip, arrow, false);
                    return true;
                }
                if down && strip.menu.is_some_and(|menu| menu.contains(mouse.x, mouse.y)) {
                    if left_down {
                        self.open_menu(cx, &strip);
                    }
                    return true;
                }
                let used = self.model.pointer(cx, mouse, hit, &self.drop_slots(&strip), Direction::Across);
                if mouse.kind == MouseKind::Drag(MouseButton::Left) {
                    let zone = self.drag_zone(area, &strip, mouse.x);
                    self.model.edge_scroll(cx, zone, |cx, arrow| {
                        self.scroll(cx, &strip, arrow, true).then(|| cx.memory::<TabsMemory>().offset)
                    });
                }
                used
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.labels.is_empty() || self.on_add.is_some()
    }
}

impl<Msg: 'static> Tabs<Msg> {
    /// Whether `event` is a press somewhere other than on the menu control `menu`.
    fn presses_beside(event: &Event, menu: Option<Rect>) -> bool {
        matches!(event, Event::Mouse(mouse)
            if matches!(mouse.kind, MouseKind::Down(_)) && !menu.is_some_and(|rect| rect.contains(mouse.x, mouse.y)))
    }

    /// Whether `strip` scrolls: it hides tabs and has no menu for them. A strip too narrow for
    /// its arrows still scrolls with the wheel and the keys.
    fn scrolls(&self, strip: &Strip) -> bool {
        self.overflow == Overflow::Arrows && strip.tabs.len() < self.labels.len()
    }

    /// Opens the menu of hidden tabs, starting on the open tab when the strip had no room to show
    /// it.
    fn open_menu(&self, cx: &mut EventCx<'_, Msg>, strip: &Strip) {
        let hidden = Self::hidden(self.labels.len(), strip);
        PopupMenu::open(cx, hidden.iter().position(|i| *i == self.model.active()).unwrap_or(0));
    }

    /// The tabs `strip` does not show, in index order.
    fn hidden(count: usize, strip: &Strip) -> Vec<usize> {
        (0..count).filter(|i| !strip.tabs.iter().any(|(visible, _)| visible == i)).collect()
    }

    /// Whether `arrow` can scroll the strip `strip`, which starts at position `offset`.
    fn can_scroll(&self, strip: &Strip, offset: usize, arrow: Arrow) -> bool {
        match arrow {
            Arrow::Back => offset > 0,
            Arrow::Forward => offset + strip.tabs.len() < self.labels.len(),
        }
    }

    /// The end of the strip a tab dragged to column `x` is held against, if the strip scrolls: a
    /// scroll arrow, or past the strip's edge when it has no room for arrows. Only the column counts,
    /// so a pointer that strays off the strip's line while dragging still scrolls.
    fn drag_zone(&self, area: Rect, strip: &Strip, x: i32) -> Option<Zone> {
        if !self.scrolls(strip) || strip.add.is_some_and(|add| x >= add.x) {
            return None;
        }
        let (back_end, forward_start) =
            strip.arrows.map_or((area.x, area.right()), |(back, forward)| (back.right(), forward.x));
        if x < back_end {
            Some(Zone { edge: Edge::Back, beyond: clamp_u16(area.x - x) })
        } else if x >= forward_start {
            Some(Zone { edge: Edge::Forward, beyond: clamp_u16(x + 1 - area.right()) })
        } else {
            None
        }
    }

    /// Scrolls the strip one tab with `arrow`; `flash` lights the arrow as pressed, for clicks,
    /// keys and a dragged tab. An arrow at the end of the strip does nothing. True when it scrolled.
    fn scroll(&self, cx: &mut EventCx<'_, Msg>, strip: &Strip, arrow: Arrow, flash: bool) -> bool {
        let now = cx.now();
        let memory = cx.memory::<TabsMemory>();
        if !self.can_scroll(strip, memory.offset, arrow) {
            return false;
        }
        memory.offset = match arrow {
            Arrow::Back => memory.offset - 1,
            Arrow::Forward => memory.offset + 1,
        };
        if flash {
            memory.pressed = Some((arrow, now));
        }
        true
    }
}
