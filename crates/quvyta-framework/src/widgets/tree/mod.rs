//! Trees: nested rows that open and close, flattened to what is visible and virtualised.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::{CellStyle, WidgetStyle};
use crate::text;
use crate::theme::State;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::delayed::DelayedIndicator;
use super::row::{self, LEAD};
use super::rows::{self, RowScroll, Step};
use super::{ContextItem, SpinnerStyle, tab_model};

mod edit;
#[cfg(test)]
mod tests;

use edit::Arrange;
pub use edit::TreeMove;

/// Cells of indentation per level.
const INDENT: u16 = 2;

/// One node of a [`Tree`], identified by a key that stays the same while the tree changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    key: String,
    label: String,
    icon: Option<String>,
    icon_color: Option<String>,
    detail: Option<String>,
    children: Vec<TreeNode>,
    expandable: bool,
    expanded: bool,
    loading: bool,
    faint: bool,
}

impl TreeNode {
    /// A leaf named `label`, identified by `key`.
    #[must_use]
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            icon: None,
            icon_color: None,
            detail: None,
            children: Vec::new(),
            expandable: false,
            expanded: false,
            loading: false,
            faint: false,
        }
    }

    /// Child nodes; a node with children can be opened.
    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        self.children = children.into_iter().collect();
        self.expandable = self.expandable || !self.children.is_empty();
        self
    }

    /// Marks a node as openable before its children are known, for children loaded when it
    /// opens. Opening it sends [`Tree::on_expand`]; supply the children in a later frame.
    #[must_use]
    pub fn expandable(mut self, expandable: bool) -> Self {
        self.expandable = expandable || !self.children.is_empty();
        self
    }

    /// Whether the node is open and shows its children.
    #[must_use]
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Marks the node's children as being loaded. A load that takes longer than about 300 ms
    /// shows a spinner in place of the chevron, which then stays at least about 500 ms; quicker
    /// loads keep the chevron, so they never flash a spinner.
    #[must_use]
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Icon key drawn before the label, optionally in theme colour `color`.
    #[must_use]
    pub fn icon(mut self, key: impl Into<String>, color: Option<&str>) -> Self {
        self.icon = Some(key.into());
        self.icon_color = color.map(str::to_owned);
        self
    }

    /// Faint text aligned right, e.g. a count.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Draws the node faint while keeping it selectable.
    #[must_use]
    pub fn faint(mut self, faint: bool) -> Self {
        self.faint = faint;
        self
    }
}

/// A visible row of the flattened tree.
struct Flat<'a> {
    node: &'a TreeNode,
    depth: u16,
    parent: Option<usize>,
    /// The node's position among its siblings as the application gave them.
    index: usize,
}

/// How one row is drawn besides its node.
#[derive(Debug, Clone, Copy)]
struct RowFlags {
    hovered: bool,
    selected: bool,
    focused: bool,
    pressed: bool,
    spinning: bool,
}

/// Delayed loading spinners of the nodes on their way, by node key, kept in runtime memory.
#[derive(Debug, Default)]
struct LoadingMarks(Vec<(String, DelayedIndicator)>);

/// Builds a message from a node key.
type KeyMessage<Msg> = Box<dyn Fn(&str) -> Msg>;

/// Builds a message from a node key and whether it should open.
type ExpandMessage<Msg> = Box<dyn Fn(&str, bool) -> Msg>;

/// Builds a message from a move among siblings.
type MoveMessage<Msg> = Box<dyn Fn(TreeMove) -> Msg>;

/// Builds the context menu entries of a node from its key.
type MenuItems<Msg> = Box<dyn Fn(&str) -> Vec<ContextItem<Msg>>>;

/// Nested rows that open and close, like folders.
///
/// The application owns the nodes, which are open and which one is selected, identified by
/// node keys; the tree reports changes through messages. Only the open part of the tree is
/// flattened and only the rows on screen are drawn, so large trees stay fast. Children can be
/// loaded when a node opens: mark it [`TreeNode::expandable`], answer [`Tree::on_expand`] with a
/// background command and mark the node [`TreeNode::loading`] meanwhile; its spinner only shows
/// when the load is slow.
///
/// Rows are indented by space; openable rows carry a chevron. A hovered or selected row raises
/// its surface and shows the pillar; only its icon and label slide one cell right. The
/// indentation, the chevron (or the loading spinner in its place) and the detail never move, so
/// the chevron is always where the pointer clicks it.
///
/// Keys while focused: ↑/↓ or k/j, PgUp/PgDn, Home/End move; → opens a node or moves to its
/// first child; ← closes it or moves to its parent; Enter opens or closes a node with children
/// and activates a leaf; Space activates. A click selects a row and opens, closes or activates
/// it like Enter; a click on the chevron only opens or closes.
///
/// Two capabilities are off until asked for:
///
/// - [`reorderable`](Self::reorderable): drag a node to move it among its siblings; the siblings
///   make room, a ghost row follows the pointer and a tinted slot shows where it lands, while the
///   dragged node's own children fold away. Ctrl+Shift+↑/↓ moves the selected node one place. A
///   node keeps its parent: moving under another parent is the application's own action, offered
///   in the context menu. Held on the top or bottom row, or past them, a drag scrolls the tree one
///   row after 400 ms and then every 150 ms. With reordering on, a click opens, closes or
///   activates on release, so pressing a row to drag it does not open it.
/// - [`context_menu`](Self::context_menu): a right click on a row opens a menu of actions for that
///   node at the pointer and keeps the row raised while it is open; the menu key or Shift+F10
///   opens the menu of the selected node below its row. Without it a right click does nothing.
///
/// Style keys: rows use `list-item` (`hover`, `selected`, `focus`, `pressed`), `list-item.faint`,
/// `list-detail` and `list-header` (empty text) like [`List`](super::List); `tree-chevron`
/// (`fg`) with `hover` and `selected`; `spinner` for loading nodes; `scrollbar`. Icons:
/// `tree-collapsed`, `tree-expanded`, `spinner`. A drag uses `tab-drop` for the landing slot and
/// `tab-ghost` for the row following the pointer, like the tabs; the menu uses the keys of
/// [`ContextItem`].
pub struct Tree<Msg> {
    roots: Vec<TreeNode>,
    selected: Option<String>,
    empty: String,
    on_select: Option<KeyMessage<Msg>>,
    on_activate: Option<KeyMessage<Msg>>,
    on_expand: Option<ExpandMessage<Msg>>,
    on_move: Option<MoveMessage<Msg>>,
    menu: Option<MenuItems<Msg>>,
}

impl<Msg: 'static> Tree<Msg> {
    /// A tree with top-level nodes `roots`.
    #[must_use]
    pub fn new(roots: impl IntoIterator<Item = TreeNode>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
            selected: None,
            empty: String::new(),
            on_select: None,
            on_activate: None,
            on_expand: None,
            on_move: None,
            menu: None,
        }
    }

    /// The key of the selected node.
    #[must_use]
    pub fn selected(mut self, key: Option<&str>) -> Self {
        self.selected = key.map(str::to_owned);
        self
    }

    /// Text shown when there are no nodes.
    #[must_use]
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    /// Message for moving the selection to a node.
    #[must_use]
    pub fn on_select(mut self, message: impl Fn(&str) -> Msg + 'static) -> Self {
        self.on_select = Some(Box::new(message));
        self
    }

    /// Message for activating a node: Enter on a leaf, Space, a click on a leaf.
    #[must_use]
    pub fn on_activate(mut self, message: impl Fn(&str) -> Msg + 'static) -> Self {
        self.on_activate = Some(Box::new(message));
        self
    }

    /// Message asking to open (`true`) or close (`false`) a node.
    #[must_use]
    pub fn on_expand(mut self, message: impl Fn(&str, bool) -> Msg + 'static) -> Self {
        self.on_expand = Some(Box::new(message));
        self
    }

    /// Makes nodes reorderable among their siblings: `message(TreeMove)` asks the application to
    /// move one. [`TreeMove::apply`] applies it to the application's list of siblings.
    #[must_use]
    pub fn reorderable(mut self, message: impl Fn(TreeMove) -> Msg + 'static) -> Self {
        self.on_move = Some(Box::new(message));
        self
    }

    /// Gives every node a context menu: `items(key)` builds the entries for the node with that key,
    /// such as Rename, Archive or Move to. Choosing an entry sends its message.
    #[must_use]
    pub fn context_menu(mut self, items: impl Fn(&str) -> Vec<ContextItem<Msg>> + 'static) -> Self {
        self.menu = Some(Box::new(items));
        self
    }

    fn flatten(&self) -> Vec<Flat<'_>> {
        self.flatten_with(None)
    }

    /// The visible rows, laid out for a drag when `arrange` is given.
    fn flatten_with(&self, arrange: Option<&Arrange<'_>>) -> Vec<Flat<'_>> {
        fn walk<'a>(
            nodes: &'a [TreeNode],
            parent_key: Option<&str>,
            (depth, parent): (u16, Option<usize>),
            arrange: Option<&Arrange<'_>>,
            out: &mut Vec<Flat<'a>>,
        ) {
            let preview = arrange.filter(|arrange| arrange.parent == parent_key).and_then(|arrange| arrange.order);
            for index in tab_model::preview_order(nodes.len(), preview) {
                let node = &nodes[index];
                let at = out.len();
                out.push(Flat { node, depth, parent, index });
                let folded = arrange.is_some_and(|arrange| arrange.key == node.key);
                if node.expanded && !folded {
                    walk(&node.children, Some(&node.key), (depth.saturating_add(1), Some(at)), arrange, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.roots, None, (0, None), arrange, &mut out);
        out
    }

    fn selected_index(&self, flat: &[Flat<'_>]) -> Option<usize> {
        let key = self.selected.as_deref()?;
        flat.iter().position(|row| row.node.key == key)
    }

    fn select(&self, cx: &mut EventCx<'_, Msg>, flat: &[Flat<'_>], index: usize) {
        let Some(row) = flat.get(index) else { return };
        if self.selected.as_deref() != Some(row.node.key.as_str())
            && let Some(message) = &self.on_select
        {
            cx.emit(message(&row.node.key));
        }
    }

    fn expand(&self, cx: &mut EventCx<'_, Msg>, node: &TreeNode, open: bool) -> bool {
        match &self.on_expand {
            Some(message) if node.expandable && node.expanded != open => {
                cx.emit(message(&node.key, open));
                true
            }
            _ => false,
        }
    }

    fn activate(&self, cx: &mut EventCx<'_, Msg>, index: usize, node: &TreeNode) -> bool {
        let Some(message) = &self.on_activate else {
            return false;
        };
        cx.memory::<RowScroll>().flashed = Some(index);
        cx.flash();
        cx.emit(message(&node.key));
        true
    }

    /// Enter and click: a node with children opens or closes, a leaf activates.
    fn open_or_activate(&self, cx: &mut EventCx<'_, Msg>, index: usize, node: &TreeNode) -> bool {
        if node.expandable { self.expand(cx, node, !node.expanded) } else { self.activate(cx, index, node) }
    }

    /// Moves the delayed spinner of every open row on to this frame and returns the rows whose
    /// spinner shows. Marks of nodes that are gone or idle are forgotten.
    fn loading_marks(cx: &mut PaintCx<'_>, flat: &[Flat<'_>]) -> Vec<usize> {
        let now = cx.now();
        let mut marks = std::mem::take(&mut cx.memory::<LoadingMarks>().0);
        let mut kept = Vec::new();
        let mut spinning = Vec::new();
        let mut next: Option<std::time::Duration> = None;
        for (index, row) in flat.iter().enumerate() {
            let node = row.node;
            let known = marks.iter().position(|(key, _)| *key == node.key);
            if !node.expandable || (!node.loading && known.is_none()) {
                continue;
            }
            let mut mark = known.map(|at| marks.swap_remove(at).1).unwrap_or_default();
            if mark.update(node.loading, now) {
                spinning.push(index);
            }
            if let Some(change) = mark.next_change(node.loading, now) {
                next = Some(next.map_or(change, |soonest| soonest.min(change)));
            }
            if !mark.is_idle() {
                kept.push((node.key.clone(), mark));
            }
        }
        if let Some(delay) = next {
            cx.request_frame_in(delay);
        }
        cx.memory::<LoadingMarks>().0 = kept;
        spinning
    }

    /// Where the chevron of a row at `depth` starts, before any slide.
    fn chevron_x(area: Rect, depth: u16) -> i32 {
        area.x + i32::from(LEAD) + i32::from(depth.saturating_mul(INDENT))
    }

    fn paint_row(&self, cx: &mut PaintCx<'_>, rect: Rect, index: usize, row: &Flat<'_>, flags: RowFlags) {
        let flashed = cx.memory::<RowScroll>().flashed == Some(index);
        let states = rows::row_states(flags.hovered, flags.selected, flags.focused, flags.pressed && flashed);
        let style = cx.style("list-item", row.node.faint.then_some("faint"), &states);
        let slide = rows::slide(cx, &states) > 0;
        Self::paint_node(cx, rect, row, (&style, &states), flags.spinning, slide);
    }

    /// Paints the node of `row` into `rect` in `style`, the look of a row in `states`.
    fn paint_node(
        cx: &mut PaintCx<'_>,
        rect: Rect,
        row: &Flat<'_>,
        (style, states): (&WidgetStyle, &[State]),
        spinning: bool,
        slide: bool,
    ) {
        let node = row.node;
        let text_style = style.text();
        let detail_width = node.detail.as_deref().map_or(0, |d| text::width(d).saturating_add(2));

        // The chevron is a fixed mark: it stays in its column while the icon and label slide.
        // Leaves keep the chevron's column empty so labels of one level line up.
        let chevron = if !node.expandable {
            (" ".to_owned(), CellStyle::default())
        } else if spinning {
            let style = cx.style("spinner", None, &[]).text();
            let cell = cx.animation(SpinnerStyle::Dots.animation(), style, Some(std::time::Duration::ZERO));
            (text::truncate(&cell.glyph, 1).into_owned(), cell.style)
        } else {
            let key = if node.expanded { "tree-expanded" } else { "tree-collapsed" };
            let glyph = text::truncate(&cx.env().icons().glyph(key), 1).into_owned();
            (glyph, cx.style("tree-chevron", None, states).text())
        };
        let icon: Vec<row::Mark> =
            node.icon.iter().map(|key| row::icon(cx, key, node.icon_color.as_deref(), text_style.fg)).collect();
        let parts = row::Parts {
            indent: row.depth.saturating_mul(INDENT),
            fixed: &[chevron],
            sliding: &icon,
            label: &node.label,
            trailing: detail_width,
        };
        row::paint_parts(cx, rect, style, slide, &parts);
        if let Some(detail) = &node.detail {
            let detail_style = cx.style("list-detail", None, states).text();
            row::paint_trailing(cx, rect, detail, detail_style);
        }
    }
}

impl<Msg: 'static> Widget<Msg> for Tree<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        let flat = self.flatten();
        let widest = flat
            .iter()
            .map(|row| {
                // Saturating: labels and details can be wider than any screen.
                [
                    LEAD,
                    row.depth.saturating_mul(INDENT),
                    2,
                    row.node.icon.as_ref().map_or(0, |_| 2),
                    text::width(&row.node.label),
                    row.node.detail.as_deref().map_or(0, |d| text::width(d).saturating_add(2)),
                    2,
                ]
                .into_iter()
                .fold(0, u16::saturating_add)
            })
            .max()
            .unwrap_or_else(|| text::width(&self.empty).saturating_add(LEAD));
        let rows = clamp_u16(i32::try_from(flat.len().max(1)).unwrap_or(i32::MAX));
        Size::new(widest, rows).min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        cx.register_hit(area);
        if self.roots.is_empty() {
            let faint = cx.style("list-header", None, &[]).text();
            cx.text(area.x + i32::from(LEAD), area.y, &self.empty, faint, area.width.saturating_sub(LEAD));
            return;
        }
        let drag = self.drag(cx);
        // An open context menu takes the overlay and the pointer: only the row it acts on stays
        // raised.
        let menu_node = self.menu_node(cx);
        if menu_node.is_some() {
            cx.request_overlay(area);
        }
        let flat = match &drag {
            Some(drag) => {
                let offset = cx.memory::<RowScroll>().offset;
                let rows = Self::rows_area(area, self.flatten().len());
                let order = self.landing(&drag.key, drag.pointer, rows, offset);
                let parent = self.siblings(&drag.key).and_then(|(parent, _)| parent);
                let arrange = Arrange { key: &drag.key, parent, order };
                self.flatten_with(Some(&arrange))
            }
            None => self.flatten(),
        };
        let focused = cx.is_focused();
        let pressed = cx.is_pressed();
        let selected = self.selected_index(&flat);
        let visible = usize::from(area.height);
        let offset = cx.memory::<RowScroll>().follow(selected, flat.len(), visible);
        let width = Self::rows_area(area, flat.len()).width;
        let spinning = Self::loading_marks(cx, &flat);
        let pointer = cx.pointer().filter(|_| drag.is_none() && menu_node.is_none());
        for (row, index) in (offset..flat.len()).take(visible).enumerate() {
            let rect = Rect::new(area.x, area.y + i32::try_from(row).unwrap_or(0), width, 1);
            let key = flat[index].node.key.as_str();
            if drag.as_ref().is_some_and(|drag| drag.key == key) {
                tab_model::paint_drop_slot(cx, rect);
                continue;
            }
            let hovered = pointer.is_some_and(|(x, y)| rect.contains(x, y)) || menu_node.as_deref() == Some(key);
            let flags = RowFlags {
                hovered,
                selected: selected == Some(index),
                focused,
                pressed,
                spinning: spinning.contains(&index),
            };
            self.paint_row(cx, rect, index, &flat[index], flags);
        }
        // The dragged node follows the pointer as a ghost row, kept inside the tree.
        if let Some(drag) = &drag
            && let Some(row) = flat.iter().find(|row| row.node.key == drag.key)
            && !area.is_empty()
        {
            let y = drag.pointer.1.clamp(area.y, area.bottom() - 1);
            let rect = Rect::new(area.x, y, width, 1);
            // The ghost covers the row under it, so its surface is cleared first.
            tab_model::paint_ghost_surface(cx, rect);
            let ghost = cx.style("tab-ghost", None, &[]);
            Self::paint_node(cx, rect, row, (&ghost, &[]), false, false);
        }
        rows::paint_scrollbar(cx, area, flat.len(), offset, None);
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        self.paint_menu(cx, anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let area = cx.area();
        let flat = self.flatten();
        if self.menu_event(cx, event, &flat) {
            return true;
        }
        let current = self.selected_index(&flat);
        match event {
            Event::Key(key) => {
                if self.move_key(cx, key) {
                    return true;
                }
                if let Some(step) = Step::from_key(key) {
                    let Some(target) = step.apply(current, flat.len(), usize::from(area.height)) else {
                        return false;
                    };
                    self.select(cx, &flat, target);
                    return true;
                }
                let Some(index) = current else { return false };
                let row = &flat[index];
                if key.is_plain(Key::Right) || key.is_plain(Key::Char('l')) {
                    if !row.node.expanded {
                        return self.expand(cx, row.node, true);
                    }
                    if !row.node.children.is_empty() {
                        self.select(cx, &flat, index + 1);
                        return true;
                    }
                    return false;
                }
                if key.is_plain(Key::Left) || key.is_plain(Key::Char('h')) {
                    if row.node.expanded {
                        return self.expand(cx, row.node, false);
                    }
                    return row.parent.is_some_and(|parent| {
                        self.select(cx, &flat, parent);
                        true
                    });
                }
                if key.is_plain(Key::Enter) {
                    return self.open_or_activate(cx, index, row.node);
                }
                if key.is_plain(Key::Space) {
                    return self.activate(cx, index, row.node);
                }
                false
            }
            Event::Mouse(mouse) => {
                if rows::scroll_mouse(cx, mouse, area, flat.len()) {
                    return true;
                }
                let offset = cx.memory::<RowScroll>().offset;
                let index = usize::try_from(mouse.y - area.y).ok().map(|r| offset + r).filter(|i| *i < flat.len());
                let on_chevron = index.is_some_and(|index| {
                    let row = &flat[index];
                    let chevron = Self::chevron_x(area, row.depth);
                    row.node.expandable && (chevron..=chevron + 1).contains(&mouse.x)
                });
                if self.on_move.is_some()
                    && !on_chevron
                    && let Some(used) = self.reorder_pointer(cx, mouse, &flat, index)
                {
                    return used;
                }
                if mouse.kind != MouseKind::Down(MouseButton::Left) {
                    return false;
                }
                let Some(index) = index else {
                    return false;
                };
                let row = &flat[index];
                if on_chevron {
                    return self.expand(cx, row.node, !row.node.expanded);
                }
                self.select(cx, &flat, index);
                self.open_or_activate(cx, index, row.node);
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        !self.roots.is_empty()
    }
}
