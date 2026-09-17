//! Vertical tabs for switching between projects or workspaces.
//!
//! [`TabRail`] and its options live here; `layout` places the rows, `paint` draws them and the
//! shared behaviour of every tab view (opening, closing, reordering) is in
//! [`tab_model`](super::tab_model).

mod layout;
mod paint;
#[cfg(test)]
mod tests;

use crate::event::{Event, MouseButton, MouseEvent, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::cells;
use super::close_mark;
use super::context_item::ContextItem;
use super::context_menu;
use super::edge_scroll::Edge;
use super::row::LEAD;
use super::scrollbar::{self, ScrollMetrics};
use super::tab_model::{self, Direction, TabModel, drop_target, preview_order};

/// Width of a collapsed rail: pillar, a one-cell label that never moves, a cell of air and the
/// column kept for the scrollbar, so the label and the scrollbar never touch.
const COLLAPSED: u16 = 4;

/// What the one-cell marker of a tab in a collapsed [`TabRail`] shows.
///
/// Set it for the whole rail with [`TabRail::collapsed_marker`] and for one tab with
/// [`RailTab::marker`], so users can pick a marker per tab as in the activity bar of an editor.
/// The expanded rail is not affected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollapsedMarker {
    /// The tab's icon, or its initial when it has none. The default.
    #[default]
    Icon,
    /// The first character of the name, upper case when the character has a single upper case
    /// form.
    Initial,
    /// The tab's position counted from 1. Positions past 9 do not fit one cell and show `…`,
    /// rather than a digit that would name another tab.
    Number,
}

/// One tab of a [`TabRail`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailTab {
    name: String,
    icon: Option<String>,
    badge: Option<String>,
    status: Option<String>,
    marker: Option<CollapsedMarker>,
}

impl RailTab {
    /// A tab called `name`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), icon: None, badge: None, status: None, marker: None }
    }

    /// What a collapsed rail shows for this tab, instead of the rail's
    /// [`collapsed_marker`](TabRail::collapsed_marker).
    #[must_use]
    pub fn marker(mut self, marker: CollapsedMarker) -> Self {
        self.marker = Some(marker);
        self
    }

    /// Icon key drawn before the name; a collapsed rail shows only the icon (or the name's first
    /// letter when there is none) unless its marker says otherwise.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>) -> Self {
        self.icon = Some(key.into());
        self
    }

    /// Short faint text at the right, such as the number of running containers.
    #[must_use]
    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }

    /// A status dot in theme colour `token` (`success`, `warning`, `danger`, `info`) before the
    /// badge; a collapsed rail colours the tab's icon instead.
    #[must_use]
    pub fn status(mut self, token: impl Into<String>) -> Self {
        self.status = Some(token.into());
        self
    }
}

/// Tabs stacked down the side of the screen, such as the open projects of an IDE.
///
/// Every tab is a row with an icon, the name and optional status dot and badge. The open tab is
/// raised with the accent pillar, which breathes while the rail has keyboard focus; a hovered tab
/// rises softly and its icon and name slide one cell while the status dot, badge and close mark stay
/// put. [`collapsed`](Self::collapsed) narrows the rail to a thin strip of four cells: the pillar, a
/// one-cell icon (or the name's first letter) that keeps its column when its tab is hovered or
/// open, a cell of air and a column that becomes the scrollbar when tabs overflow.
/// [`collapsed_marker`](Self::collapsed_marker) and [`RailTab::marker`] make that cell the tab's
/// initial or position instead. [`on_add`](Self::on_add) ends the rail with an add (+) row.
///
/// [`row_height`](Self::row_height) makes every row a raised block several lines tall, with its
/// content on the middle line, the close mark in its top right corner and the pillar down the
/// whole block. Blocks taller than a line are one canvas line apart, so they do not read as one
/// column; [`gap`](Self::gap) sets another distance, and `gap(0)` stacks them. Hover, presses,
/// dragging, scrolling and the scrollbar count whole blocks; the collapsed strip, its name card and
/// the add row keep the same height.
///
/// In a collapsed rail a name card beside the row shows the name and badge of the hovered tab, or
/// of the open tab while the rail has keyboard focus. The pointer can move onto the card: it stays
/// while the pointer is on the row or the card, rises under the pointer and opens its tab when
/// clicked. With closing on, the card ends in the tab's close mark, on the card's middle line.
///
/// Keys while focused: ↑/↓ (or k/j) open the neighbouring tab, Home/End the first and last. The
/// options are the same as [`Tabs`](super::Tabs) and behave the same:
/// [`closable`](Self::closable) (a faint `×`, middle click, ctrl+w), [`pinned`](Self::pinned) and
/// [`reorderable`](Self::reorderable) (drag with a ghost over the slot where the tab lands,
/// ctrl+shift+↑/↓; held on the first or last visible block or past the rail's end, a dragged tab
/// scrolls the rail one block after 400 ms and then every 150 ms, sooner the further past the end,
/// with the scrollbar lit and the drop slot following, see [`on_drag_scroll`](Self::on_drag_scroll))
/// and [`context_menu`](Self::context_menu) (a right click on a tab opens its
/// menu at the pointer, the menu key or shift+F10 the menu of the open tab; without it a right click
/// does nothing). Rows that do not fit scroll with the wheel or the scrollbar (click or drag it) and
/// follow the open tab.
///
/// Style keys: `rail-tab` with `hover`, `selected`, `focus` and the variant `tall` for blocks taller
/// than a line; `rail-badge` with the same states; `rail-hint` (`bg`, `fg`) with `hover` for the
/// name card of a collapsed rail; `rail-add` with `hover` and `tall` for the add row;
/// `close-mark`, `tab-ghost` and `tab-drop` shared with [`Tabs`](super::Tabs); `scrollbar`; the
/// keys of [`ContextItem`](super::ContextItem) for the context menu.
pub struct TabRail<Msg> {
    tabs: Vec<RailTab>,
    collapsed: bool,
    marker: CollapsedMarker,
    row_height: u16,
    /// Canvas lines between rows when set; otherwise [`TabRail::spacing`] decides.
    gap: Option<u16>,
    on_add: Option<Box<dyn Fn() -> Msg>>,
    model: TabModel<Msg>,
}

#[derive(Debug, Default)]
struct RailMemory {
    offset: usize,
    followed: Option<usize>,
    /// The collapsed row whose name card is shown beside the rail: a tab, or the add row.
    hint: Option<RailRow>,
    /// The name card painted in the last frame and its row. The card stays while the pointer is on
    /// it, and presses on it act on its row.
    card: Option<(RailRow, Rect)>,
    /// Whether the pointer is dragging the scrollbar.
    dragging_scrollbar: bool,
}

impl RailMemory {
    /// Scrolls the least that shows row `index` when `visible` rows fit.
    fn reveal(&mut self, index: usize, visible: usize) {
        if index < self.offset {
            self.offset = index;
        } else if index >= self.offset + visible {
            self.offset = index + 1 - visible;
        }
    }
}

/// A row of the rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RailRow {
    Tab(usize),
    Add,
}

impl<Msg: 'static> TabRail<Msg> {
    /// A rail of `tabs`; the first is open.
    #[must_use]
    pub fn new(tabs: impl IntoIterator<Item = RailTab>) -> Self {
        let tabs: Vec<RailTab> = tabs.into_iter().collect();
        let model = TabModel::new(tabs.len());
        Self { tabs, collapsed: false, marker: CollapsedMarker::Icon, row_height: 1, gap: None, on_add: None, model }
    }

    /// The open tab.
    #[must_use]
    pub fn active(mut self, index: usize) -> Self {
        self.model.active = index;
        self
    }

    /// Message for opening tab `index`.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.model.on_select = Some(Box::new(message));
        self
    }

    /// Narrows the rail to a four-cell strip of icons; a name card beside a hovered tab names it
    /// and takes clicks.
    #[must_use]
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// What the collapsed strip shows for tabs without their own [`RailTab::marker`]: their icon
    /// (the default), initial or position.
    #[must_use]
    pub fn collapsed_marker(mut self, marker: CollapsedMarker) -> Self {
        self.marker = marker;
        self
    }

    /// Makes every row a block `lines` tall (at least one), like the project tabs of an IDE: the
    /// name, icon, status and badge sit on the middle line (the upper one of an even block), the
    /// close mark flush right on the first line, and resting tabs are raised blocks too. One line,
    /// the default, is a plain list of rows. Blocks taller than one line are one canvas line apart
    /// unless [`gap`](Self::gap) is set. Any height works; the rail scrolls by whole blocks.
    #[must_use]
    pub fn row_height(mut self, lines: u16) -> Self {
        self.row_height = lines.max(1);
        self
    }

    /// Leaves `lines` of canvas between rows. By default one line separates rows taller than one
    /// line and none separates one-line rows; `gap(0)` stacks tall blocks.
    #[must_use]
    pub fn gap(mut self, lines: u16) -> Self {
        self.gap = Some(lines);
        self
    }

    /// Ends the rail with an add row (`+`) that sends `message` when clicked, e.g. to open a
    /// new project.
    #[must_use]
    pub fn on_add(mut self, message: impl Fn() -> Msg + 'static) -> Self {
        self.on_add = Some(Box::new(message));
        self
    }

    /// Makes tabs closable: `message(index)` asks the application to close tab `index`.
    /// [`TabEdit::Close`](super::TabEdit) applies it to the application's list.
    #[must_use]
    pub fn closable(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.model.set_on_close(message);
        self
    }

    /// Tabs that cannot be closed. They show no close mark.
    #[must_use]
    pub fn pinned(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.model.pinned = indices.into_iter().collect();
        self
    }

    /// Makes tabs reorderable: `message(from, to)` asks the application to move a tab.
    /// [`TabEdit::Move`](super::TabEdit) applies it to the application's list.
    #[must_use]
    pub fn reorderable(mut self, message: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.model.set_on_move(message);
        self
    }

    /// Message for each step a dragged tab scrolls an overflowing rail, with the first row now in
    /// view (counted from 0), e.g. to log it. Only reorderable rails scroll this way.
    #[must_use]
    pub fn on_drag_scroll(mut self, message: impl Fn(usize) -> Msg + 'static) -> Self {
        self.model.on_drag_scroll = Some(Box::new(message));
        self
    }

    /// Gives every tab a context menu: `items(index)` builds the entries for tab `index`, such as
    /// Close, Close others or Pin. A right click on a tab (or on the name card of a collapsed rail)
    /// opens its menu at the pointer; the menu key or shift+F10 opens the menu of the open tab below
    /// its block, scrolling the tab into view first. Choosing an entry sends its message.
    #[must_use]
    pub fn context_menu(mut self, items: impl Fn(usize) -> Vec<ContextItem<Msg>> + 'static) -> Self {
        self.model.set_context_menu(items);
        self
    }
}

impl<Msg: 'static> Widget<Msg> for TabRail<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let height = self.natural_height(self.rows());
        if self.collapsed {
            return Size::new(COLLAPSED, height).min(available);
        }
        let widest = (0..self.tabs.len())
            .map(|i| {
                let icon = if self.tabs[i].icon.is_some() { 2 } else { 0 };
                cells::sum([LEAD, icon, text::width(&self.tabs[i].name), self.trailing_width(i, self.row_height), 3])
            })
            .max()
            .unwrap_or(0);
        Size::new(widest, height).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        let card = {
            let memory = cx.memory::<RailMemory>();
            memory.hint = None;
            memory.card.take()
        };
        if self.rows() == 0 || area.is_empty() {
            return;
        }
        let focused = cx.is_focus_visible();
        let pointer = cx.pointer();
        let active = self.model.active();
        let drag = self.model.drag(cx);
        let visible = self.visible_blocks(area.height);
        let offset = {
            let memory = cx.memory::<RailMemory>();
            if drag.is_none() && memory.followed != Some(active) {
                memory.reveal(active, visible);
                memory.followed = Some(active);
            }
            memory.offset = memory.offset.min(self.rows().saturating_sub(visible));
            memory.offset
        };
        let (content, metrics) = self.content(area, offset);
        let resting = self.slots(&self.identity(), offset, content);
        let target = drag.map(|d| (d.index, drop_target(&resting, d.index, d.pointer, Direction::Down)));
        let order = preview_order(self.tabs.len(), target);

        // The row whose name card the pointer is on keeps its card and stays raised, so the
        // pointer can travel from the row onto the card.
        let carded = card
            .filter(|(_, rect)| drag.is_none() && pointer.is_some_and(|(x, y)| rect.contains(x, y)))
            .map(|(row, _)| row);
        // With keyboard focus and the pointer elsewhere, a collapsed rail names its open tab.
        let named = (self.collapsed && focused && pointer.is_none() && drag.is_none()).then_some(RailRow::Tab(active));
        // An open context menu takes the overlay and the pointer: no name card competes with it,
        // and only the tab it acts on stays raised.
        let menu_tab = self.model.menu_tab(cx);
        let menu = menu_tab.is_some();
        if menu {
            cx.request_overlay(area);
        }
        let carded = carded.filter(|_| !menu);
        let named = named.filter(|_| !menu);
        let pointer = pointer.filter(|_| !menu);

        for (index, rect) in self.slots(&order, offset, content) {
            if drag.is_some_and(|d| d.index == index) {
                tab_model::paint_drop_slot(cx, rect);
                continue;
            }
            let mut states = Vec::new();
            let hovered = drag.is_none() && pointer.is_some_and(|(x, y)| rect.contains(x, y));
            if hovered || carded == Some(RailRow::Tab(index)) || menu_tab == Some(index) {
                states.push(State::Hover);
            }
            if self.collapsed && !menu && (hovered || carded.or(named) == Some(RailRow::Tab(index))) {
                cx.memory::<RailMemory>().hint = Some(RailRow::Tab(index));
                cx.request_overlay(rect);
            }
            if index == active {
                states.push(State::Selected);
                if focused {
                    states.push(State::Focus);
                }
            }
            self.paint_row(cx, rect, index, &states, None);
        }

        if let Some(rect) = self.add_rect(content, offset) {
            let hovered =
                drag.is_none() && (pointer.is_some_and(|(x, y)| rect.contains(x, y)) || carded == Some(RailRow::Add));
            if hovered && self.collapsed && !menu {
                cx.memory::<RailMemory>().hint = Some(RailRow::Add);
                cx.request_overlay(rect);
            }
            self.paint_add(cx, rect, hovered);
        }

        if let Some(drag) = drag {
            let height = resting.first().map_or(1, |(_, rect)| rect.height);
            let top = content.y;
            let bottom = (content.bottom() - i32::from(height)).max(top);
            let y = (drag.pointer.1 - drag.grab.1).clamp(top, bottom);
            let rect = Rect::new(content.x, y, content.width, height);
            tab_model::paint_ghost_surface(cx, rect);
            self.paint_row(cx, rect, drag.index, &[], Some("ghost"));
        }

        if metrics.overflows() {
            let bar = Rect::new(area.right() - 1, area.y, 1, area.height);
            // A dragged tab held against an end that can still scroll lights the scrollbar, the
            // rail's own sign that it is moving.
            let held =
                drag.and_then(|drag| self.drag_zone(area, content, metrics, drag.pointer.1)).is_some_and(|zone| {
                    match zone.edge {
                        Edge::Back => offset > 0,
                        Edge::Forward => offset < metrics.max_offset(),
                    }
                });
            let active = held
                || cx.memory::<RailMemory>().dragging_scrollbar
                || (carded.is_none() && pointer.is_some_and(|(x, _)| x == bar.x));
            scrollbar::paint(cx, bar, metrics, active, None);
        }
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        if self.model.paint_menu(cx, anchor) {
            return;
        }
        let Some(row) = cx.memory::<RailMemory>().hint else {
            return;
        };
        let (name, badge, closable) = match row {
            RailRow::Tab(index) => {
                (self.tabs[index].name.clone(), self.tabs[index].badge.clone(), self.model.closable(index))
            }
            RailRow::Add => (cx.env().i18n().translate("quvyta.tab-rail.add", &[]), None, false),
        };
        let card = Self::card(cx.clip(), anchor, &name, badge.as_deref(), closable);
        let line = Self::middle(card);
        let close = closable.then(|| close_mark::rect(card.right() - i32::from(close_mark::WIDTH), line));
        let on_card = cx.pointer_anywhere().is_some_and(|(x, y)| card.contains(x, y));
        let on_close = cx.pointer_anywhere().is_some_and(|(x, y)| close.is_some_and(|close| close.contains(x, y)));
        let states = if on_card && !on_close { vec![State::Hover] } else { Vec::new() };
        let style = cx.style("rail-hint", None, &states).text();
        cx.clear(card, style.bg.unwrap_or_else(|| cx.color("overlay")));
        cx.register_hit(card);

        // The name, then the badge and the close mark anchored to the card's right end, as in a
        // wide rail.
        let mut right = card.right() - i32::from(close.map_or(1, |close| close.width));
        if let Some(badge) = &badge {
            let badge_style = cx.style("rail-badge", None, &states).text();
            let width = text::width(badge);
            right -= i32::from(width);
            cx.text(right, line, badge, CellStyle { bg: None, ..badge_style }, width);
            right -= 2;
        }
        let budget = clamp_u16(right - (card.x + 1));
        let shown = text::truncate(&name, budget).into_owned();
        cx.text(card.x + 1, line, &shown, CellStyle { bg: None, ..style }, budget);
        if let Some(close) = close {
            close_mark::paint(cx, close.x, close.y, true);
        }
        cx.memory::<RailMemory>().card = Some((row, card));
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.rows() == 0 {
            return false;
        }
        let area = cx.area();
        if self.menu_event(cx, event) {
            return true;
        }
        match event {
            Event::Key(_) if self.tabs.is_empty() => false,
            Event::Key(key) => {
                if key.is_plain(Key::Home) {
                    return self.model.open(cx, 0);
                }
                if key.is_plain(Key::End) {
                    return self.model.open(cx, self.tabs.len() - 1);
                }
                self.model.key(cx, key, Direction::Down)
            }
            Event::Mouse(mouse) => {
                let offset = cx.memory::<RailMemory>().offset;
                let (content, metrics) = self.content(area, offset);
                if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) {
                    let memory = cx.memory::<RailMemory>();
                    memory.offset = if mouse.kind == MouseKind::ScrollUp {
                        memory.offset.saturating_sub(1)
                    } else {
                        (memory.offset + 1).min(metrics.max_offset())
                    };
                    return true;
                }
                // The name card starts over the scrollbar column and is on top of it, except while
                // the scrollbar is being dragged.
                let card = cx.memory::<RailMemory>().card.filter(|(_, rect)| rect.contains(mouse.x, mouse.y));
                if (card.is_none() || cx.memory::<RailMemory>().dragging_scrollbar)
                    && self.scrollbar(cx, mouse, metrics)
                {
                    return true;
                }
                // Presses on the add row (or its card) are its own; drags and releases pass on, so a
                // tab dragged down onto it still lands.
                let on_add = match card {
                    Some((row, _)) => row == RailRow::Add,
                    None => self.add_rect(content, offset).is_some_and(|add| add.contains(mouse.x, mouse.y)),
                };
                if on_add && let MouseKind::Down(button) = mouse.kind {
                    if button == MouseButton::Left
                        && let Some(message) = &self.on_add
                    {
                        cx.emit(message());
                    }
                    return true;
                }
                if self.tabs.is_empty() {
                    return false;
                }
                let slots = self.slots(&self.identity(), offset, content);
                let hit = match card {
                    Some((RailRow::Tab(index), rect)) if index < self.tabs.len() => {
                        Some(Self::card_hit(rect, index, self.model.closable(index), mouse.x, mouse.y))
                    }
                    _ => self.hit(&slots, mouse.x, mouse.y),
                };
                let used = self.model.pointer(cx, mouse, hit, &slots, Direction::Down);
                if mouse.kind == MouseKind::Drag(MouseButton::Left) {
                    let zone = self.drag_zone(area, content, metrics, mouse.y);
                    self.model.edge_scroll(cx, zone, |cx, edge| {
                        let memory = cx.memory::<RailMemory>();
                        memory.offset = match edge {
                            Edge::Back => memory.offset.checked_sub(1)?,
                            Edge::Forward => Some(memory.offset + 1).filter(|next| *next <= metrics.max_offset())?,
                        };
                        Some(memory.offset)
                    });
                }
                used
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.tabs.is_empty()
    }
}

impl<Msg: 'static> TabRail<Msg> {
    /// Offers `event` to the context menu with the tab under the pointer (on a row or on its name
    /// card) and the open tab's block; true when the menu used it. The menu key first scrolls the
    /// open tab into view, so its menu opens below its block even after the wheel moved it away.
    fn menu_event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if let Event::Key(key) = event
            && self.model.has_menu()
            && context_menu::is_menu_key(key)
            && !context_menu::is_open_in(cx)
        {
            let visible = self.visible_blocks(cx.area().height);
            cx.memory::<RailMemory>().reveal(self.model.active(), visible);
        }
        let offset = cx.memory::<RailMemory>().offset;
        let (content, _) = self.content(cx.area(), offset);
        let slots = self.slots(&self.identity(), offset, content);
        let hit = match event {
            Event::Mouse(mouse) => match cx.memory::<RailMemory>().card {
                Some((RailRow::Tab(index), rect)) if rect.contains(mouse.x, mouse.y) && index < self.tabs.len() => {
                    Some(Self::card_hit(rect, index, self.model.closable(index), mouse.x, mouse.y))
                }
                Some((RailRow::Add, rect)) if rect.contains(mouse.x, mouse.y) => None,
                _ => self.hit(&slots, mouse.x, mouse.y),
            },
            _ => None,
        };
        let open_tab = slots.iter().find(|(index, _)| *index == self.model.active()).map(|(_, rect)| *rect);
        self.model.menu_event(cx, event, hit, open_tab)
    }

    /// Clicks and drags on the scrollbar column; true when the event was for the scrollbar.
    fn scrollbar(&self, cx: &mut EventCx<'_, Msg>, mouse: &MouseEvent, metrics: ScrollMetrics) -> bool {
        let area = cx.area();
        let row = clamp_u16(mouse.y - area.y);
        match mouse.kind {
            MouseKind::Down(MouseButton::Left) if metrics.overflows() && mouse.x == area.right() - 1 => {
                cx.capture_pointer();
                let memory = cx.memory::<RailMemory>();
                memory.dragging_scrollbar = true;
                memory.offset = metrics.offset_at(row, area.height);
                true
            }
            MouseKind::Drag(MouseButton::Left) if cx.memory::<RailMemory>().dragging_scrollbar => {
                cx.memory::<RailMemory>().offset = metrics.offset_at(row, area.height);
                true
            }
            MouseKind::Up(MouseButton::Left) if cx.memory::<RailMemory>().dragging_scrollbar => {
                cx.memory::<RailMemory>().dragging_scrollbar = false;
                true
            }
            _ => false,
        }
    }
}
