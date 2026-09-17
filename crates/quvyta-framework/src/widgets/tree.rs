//! Trees: nested rows that open and close, flattened to what is visible and virtualised.

use crate::event::{Event, MouseButton, MouseKind};
use crate::geometry::{Rect, Size, clamp_u16};
use crate::keymap::Key;
use crate::style::CellStyle;
use crate::text;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::SpinnerStyle;
use super::delayed::DelayedIndicator;
use super::row::{self, LEAD};
use super::rows::{self, RowScroll, Step};

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
}

/// Delayed loading spinners of the nodes on their way, by node key, kept in runtime memory.
#[derive(Debug, Default)]
struct LoadingMarks(Vec<(String, DelayedIndicator)>);

/// Builds a message from a node key.
type KeyMessage<Msg> = Box<dyn Fn(&str) -> Msg>;

/// Builds a message from a node key and whether it should open.
type ExpandMessage<Msg> = Box<dyn Fn(&str, bool) -> Msg>;

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
/// Style keys: rows use `list-item` (`hover`, `selected`, `focus`, `pressed`), `list-item.faint`,
/// `list-detail` and `list-header` (empty text) like [`List`](super::List); `tree-chevron`
/// (`fg`) with `hover` and `selected`; `spinner` for loading nodes; `scrollbar`. Icons:
/// `tree-collapsed`, `tree-expanded`, `spinner`.
pub struct Tree<Msg> {
    roots: Vec<TreeNode>,
    selected: Option<String>,
    empty: String,
    on_select: Option<KeyMessage<Msg>>,
    on_activate: Option<KeyMessage<Msg>>,
    on_expand: Option<ExpandMessage<Msg>>,
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

    fn flatten(&self) -> Vec<Flat<'_>> {
        fn walk<'a>(nodes: &'a [TreeNode], depth: u16, parent: Option<usize>, out: &mut Vec<Flat<'a>>) {
            for node in nodes {
                let index = out.len();
                out.push(Flat { node, depth, parent });
                if node.expanded {
                    walk(&node.children, depth.saturating_add(1), Some(index), out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.roots, 0, None, &mut out);
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

    fn paint_row(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        index: usize,
        row: &Flat<'_>,
        selected: bool,
        flags: (bool, bool, bool),
    ) {
        let (focused, pressed, spinning) = flags;
        let node = row.node;
        let hovered = cx.pointer().is_some_and(|(x, y)| rect.contains(x, y));
        let flashed = cx.memory::<RowScroll>().flashed == Some(index);
        let states = rows::row_states(hovered, selected, focused, pressed && flashed);
        let style = cx.style("list-item", node.faint.then_some("faint"), &states);
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
            (glyph, cx.style("tree-chevron", None, &states).text())
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
        row::paint_parts(cx, rect, &style, rows::slide(cx, &states) > 0, &parts);
        if let Some(detail) = &node.detail {
            let detail_style = cx.style("list-detail", None, &states).text();
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
        let flat = self.flatten();
        if flat.is_empty() {
            let faint = cx.style("list-header", None, &[]).text();
            cx.text(area.x + i32::from(LEAD), area.y, &self.empty, faint, area.width.saturating_sub(LEAD));
            return;
        }
        let focused = cx.is_focused();
        let pressed = cx.is_pressed();
        let selected = self.selected_index(&flat);
        let visible = usize::from(area.height);
        let offset = cx.memory::<RowScroll>().follow(selected, flat.len(), visible);
        let width = area.width.saturating_sub(u16::from(flat.len() > visible));
        let spinning = Self::loading_marks(cx, &flat);
        for (row, index) in (offset..flat.len()).take(visible).enumerate() {
            let rect = Rect::new(area.x, area.y + i32::try_from(row).unwrap_or(0), width, 1);
            let flags = (focused, pressed, spinning.contains(&index));
            self.paint_row(cx, rect, index, &flat[index], selected == Some(index), flags);
        }
        rows::paint_scrollbar(cx, area, flat.len(), offset, None);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let area = cx.area();
        let flat = self.flatten();
        let current = self.selected_index(&flat);
        match event {
            Event::Key(key) => {
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
                if mouse.kind != MouseKind::Down(MouseButton::Left) {
                    return false;
                }
                let offset = cx.memory::<RowScroll>().offset;
                let Some(index) =
                    usize::try_from(mouse.y - area.y).ok().map(|r| offset + r).filter(|i| *i < flat.len())
                else {
                    return false;
                };
                let row = &flat[index];
                let chevron = Self::chevron_x(area, row.depth);
                if row.node.expandable && (chevron..=chevron + 1).contains(&mouse.x) {
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::time::Duration;

    use super::*;
    use crate::icons::GlyphMode;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;

    #[derive(Default)]
    struct Demo {
        open: BTreeSet<String>,
        selected: Option<String>,
        activated: Vec<String>,
        loaded: bool,
        many: usize,
        icons: bool,
    }

    /// Gives `node` and everything below it the file icon.
    fn with_icons(mut node: TreeNode) -> TreeNode {
        node.icon = Some("file".to_owned());
        node.children = node.children.into_iter().map(with_icons).collect();
        node
    }

    #[derive(Clone)]
    enum Msg {
        Select(String),
        Activate(String),
        Expand(String, bool),
        /// The lazy node's children arrived.
        Loaded,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Select(key) => self.selected = Some(key),
                Msg::Loaded => self.loaded = true,
                Msg::Activate(key) => self.activated.push(key),
                Msg::Expand(key, true) => {
                    self.open.insert(key);
                }
                Msg::Expand(key, false) => {
                    self.open.remove(&key);
                }
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            let open = |key: &str| self.open.contains(key);
            let nodes = if self.many > 0 {
                vec![
                    TreeNode::new("logs", "logs")
                        .children((0..self.many).map(|i| TreeNode::new(format!("l{i}"), format!("day-{i}.log"))))
                        .expanded(open("logs")),
                ]
            } else {
                let lazy = TreeNode::new("remote", "remote").expandable(true).expanded(open("remote"));
                let lazy = if self.loaded {
                    lazy.children([TreeNode::new("remote/a", "a.txt")])
                } else {
                    lazy.loading(open("remote"))
                };
                vec![
                    TreeNode::new("src", "src").expanded(open("src")).detail("2").children([
                        TreeNode::new("src/widgets", "widgets")
                            .expanded(open("src/widgets"))
                            .children([TreeNode::new("src/widgets/tree.rs", "tree.rs")]),
                        TreeNode::new("src/lib.rs", "lib.rs"),
                    ]),
                    lazy,
                    TreeNode::new("Cargo.toml", "Cargo.toml"),
                ]
            };
            let nodes: Vec<TreeNode> = if self.icons { nodes.into_iter().map(with_icons).collect() } else { nodes };
            let tree = Tree::new(nodes)
                .selected(self.selected.as_deref())
                .on_select(|key| Msg::Select(key.to_owned()))
                .on_activate(|key| Msg::Activate(key.to_owned()))
                .on_expand(|key, open| Msg::Expand(key.to_owned(), open));
            ui.add(tree).fill().id("tree");
        }
    }

    fn harness(app: Demo, height: u16) -> Harness<Demo> {
        let mut h = Harness::new(app, 26, height);
        h.set_glyph_mode(GlyphMode::Unicode);
        h
    }

    #[test]
    fn opens_with_keys_and_indents_children() {
        let mut h = harness(Demo::default(), 6);
        assert_eq!(h.screen(), "  ▸ src                 2\n  ▸ remote\n    Cargo.toml\n\n\n\n");
        h.press("tab").press("down").press("right");
        assert!(h.app().open.contains("src"));
        h.press("right");
        assert_eq!(h.app().selected.as_deref(), Some("src/widgets"));
        h.press("enter").press("down");
        assert_eq!(
            h.screen(),
            "  ▾ src                 2\n    ▾ widgets\n▌        tree.rs\n      lib.rs\n  ▸ remote\n    Cargo.toml\n"
        );
        h.press("enter");
        assert_eq!(h.app().activated, vec!["src/widgets/tree.rs".to_owned()]);
        h.press("left");
        assert_eq!(h.app().selected.as_deref(), Some("src/widgets"));
        h.press("left");
        assert!(!h.app().open.contains("src/widgets"));
    }

    #[test]
    fn chevron_click_only_toggles_and_row_click_selects() {
        let mut h = harness(Demo::default(), 6);
        h.click(2, 0);
        assert!(h.app().open.contains("src"));
        assert_eq!(h.app().selected, None);
        h.click_text("lib.rs");
        assert_eq!(h.app().selected.as_deref(), Some("src/lib.rs"));
        assert_eq!(h.app().activated, vec!["src/lib.rs".to_owned()]);
    }

    #[test]
    fn lazy_nodes_spin_until_children_arrive() {
        let mut h = harness(Demo::default(), 6);
        h.click_text("remote");
        assert!(h.app().open.contains("remote"));
        // A quick load keeps the chevron: no spinner for the first 300 ms.
        assert_eq!(h.screen().lines().nth(1), Some("▌ ▾  remote"), "{}", h.screen());
        h.advance(Duration::from_millis(299));
        assert_eq!(h.screen().lines().nth(1), Some("▌ ▾  remote"), "{}", h.screen());
        // The spinner takes the chevron's place and, like it, stays put while the label slides.
        h.advance(Duration::from_millis(1));
        assert_eq!(h.screen().lines().nth(1), Some("▌ ⠸  remote"), "{}", h.screen());
        h.advance(Duration::from_millis(90));
        assert_eq!(h.screen().lines().nth(1), Some("▌ ⠼  remote"), "{}", h.screen());
        let mut app = Demo { loaded: true, ..Demo::default() };
        app.open.insert("remote".to_owned());
        let h = harness(app, 6);
        assert_eq!(h.screen(), "  ▸ src                 2\n  ▾ remote\n      a.txt\n    Cargo.toml\n\n\n");
    }

    /// Whether the lazy node's row shows a spinner frame.
    fn remote_spins(h: &Harness<Demo>) -> bool {
        h.screen().lines().nth(1).is_some_and(|line| line.contains(|c| "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".contains(c)))
    }

    #[test]
    fn quick_loads_never_spin_and_slow_ones_spin_at_least_the_minimum() {
        let mut h = harness(Demo::default(), 6);
        h.click_text("remote").advance(Duration::from_millis(120)).send(Msg::Loaded);
        assert_eq!(h.screen(), "  ▸ src                 2\n▌ ▾  remote\n      a.txt\n    Cargo.toml\n\n\n");
        h.advance(Duration::from_millis(400));
        assert!(!remote_spins(&h), "{}", h.screen());

        let mut h = harness(Demo::default(), 6);
        h.click_text("remote").advance(Duration::from_millis(300)).advance(Duration::from_millis(50)).send(Msg::Loaded);
        // The children are in; the spinner shown at 300 ms stays until 800 ms.
        assert!(remote_spins(&h) && h.screen().contains("a.txt"), "{}", h.screen());
        h.advance(Duration::from_millis(449));
        assert!(remote_spins(&h), "{}", h.screen());
        h.advance(Duration::from_millis(1));
        assert_eq!(h.screen().lines().nth(1), Some("▌ ▾  remote"), "{}", h.screen());
    }

    #[test]
    fn chevrons_and_indentation_stay_put_while_icon_and_label_slide() {
        let mut open = BTreeSet::new();
        open.insert("src".to_owned());
        let mut h = harness(Demo { icons: true, open: open.clone(), ..Demo::default() }, 5);
        assert_eq!(
            h.screen(),
            "  ▾ ▪ src               2\n    ▸ ▪ widgets\n      ▪ lib.rs\n  ▸ ▪ remote\n    ▪ Cargo.toml\n"
        );
        h.hover(12, 1);
        assert_eq!(h.screen().lines().nth(1), Some("▌   ▸  ▪ widgets"), "{}", h.screen());
        h.hover(12, 0);
        assert_eq!(h.screen().lines().next(), Some("▌ ▾  ▪ src              2"), "{}", h.screen());
        let theme = h.env().theme();
        assert_eq!(h.fg(2, 0), theme.color("dim"), "a hovered row's chevron brightens in place");
        assert_eq!(h.fg(2, 3), theme.color("muted"));
        let chevron = |line: &str, glyph: char| line.chars().position(|c| c == glyph);
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(chevron(lines[0], '▾'), Some(2), "hovered or not, a chevron keeps its column");
        assert_eq!(chevron(lines[1], '▸'), Some(4));
        assert_eq!(chevron(lines[3], '▸'), Some(2));

        let mut env = crate::env::Env::builtin();
        env.set_slide(false);
        let mut h = Harness::with_env(Demo { icons: true, open, ..Demo::default() }, env, 26, 5);
        h.set_glyph_mode(GlyphMode::Unicode);
        h.hover(12, 1);
        assert_eq!(h.screen().lines().nth(1), Some("▌   ▸ ▪ widgets"), "{}", h.screen());
    }

    #[test]
    fn a_click_on_a_sliding_row_still_finds_its_chevron() {
        let mut h = harness(Demo { icons: true, ..Demo::default() }, 6);
        h.hover(10, 0);
        h.click(2, 0);
        assert!(h.app().open.contains("src"), "the chevron is where it was drawn");
        assert_eq!(h.app().selected, None, "the chevron only opens");
        h.click(3, 0);
        assert!(!h.app().open.contains("src"), "the cell after the chevron closes it again");
        h.click(5, 0);
        assert_eq!(h.app().selected.as_deref(), Some("src"), "the icon and label select");
    }

    #[test]
    fn labels_are_cut_at_the_same_place_resting_and_sliding() {
        let mut h = Harness::new(Demo { icons: true, ..Demo::default() }, 14, 3);
        h.set_glyph_mode(GlyphMode::Unicode);
        assert_eq!(h.screen().lines().nth(2), Some("    ▪ Cargo…"), "{}", h.screen());
        h.hover(8, 2);
        assert_eq!(h.screen().lines().nth(2), Some("▌    ▪ Cargo…"), "{}", h.screen());
    }

    #[test]
    fn large_open_trees_are_virtualised() {
        let mut app = Demo { many: 50_000, ..Demo::default() };
        app.open.insert("logs".to_owned());
        let mut h = harness(app, 5);
        h.press("tab").press("end");
        assert_eq!(h.app().selected.as_deref(), Some("l49999"));
        let screen = h.screen();
        assert!(screen.contains("day-49999.log"), "{screen}");
        assert!(!super::super::scrollbar::column(&h, 25).contains(' '), "{screen}");
    }

    #[test]
    fn ascii_chevrons_and_empty_text() {
        let mut h = harness(Demo::default(), 3);
        h.set_glyph_mode(GlyphMode::Ascii);
        assert!(h.screen().starts_with("  + src"), "{}", h.screen());
        struct Empty;
        impl App for Empty {
            type Msg = ();
            fn update(&mut self, _: ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                ui.add(Tree::new(Vec::new()).empty_text("No folders")).fill();
            }
        }
        assert_eq!(Harness::new(Empty, 20, 1).screen(), "  No folders\n");
    }

    #[test]
    fn labels_wider_than_any_screen_do_not_overflow_the_measure() {
        struct Wide;

        impl App for Wide {
            type Msg = ();
            fn update(&mut self, (): ()) -> Command<()> {
                Command::none()
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                let long = "n".repeat(70_000);
                ui.add(Tree::new([TreeNode::new("wide", long.clone()).detail(long)]));
            }
        }

        let h = Harness::new(Wide, 12, 1);
        // The detail keeps its right anchor and covers the row; the label has no room left.
        assert_eq!(h.screen(), "nnnnnnnnnnn\n");
    }
}
