//! Building the view tree.

use super::flex::{Axis, Flex};
use super::{Align, Container, Key, Length, Node, Widget};
use crate::env::Env;
use crate::geometry::Padding;

/// Collects the nodes of one container while an application's `view` runs.
pub struct View<'a, Msg> {
    nodes: &'a mut Vec<Node<Msg>>,
    env: &'a Env,
}

impl<'a, Msg: 'static> View<'a, Msg> {
    pub(crate) fn new(nodes: &'a mut Vec<Node<Msg>>, env: &'a Env) -> Self {
        Self { nodes, env }
    }

    /// The environment: theme, icons, language and keymap.
    #[must_use]
    pub fn env(&self) -> &Env {
        self.env
    }

    /// Adds a widget.
    pub fn add<W: Widget<Msg>>(&mut self, widget: W) -> NodeMut<'_, Msg> {
        let index = self.nodes.len();
        self.nodes.push(Node::new(widget, index));
        NodeMut { node: self.nodes.last_mut().expect("a node was just pushed") }
    }

    /// Adds a widget that contains other widgets, built by `build`.
    pub fn add_with<W: Container<Msg>>(
        &mut self,
        mut widget: W,
        build: impl FnOnce(&mut View<'_, Msg>),
    ) -> NodeMut<'_, Msg> {
        let mut children = Vec::new();
        build(&mut View::new(&mut children, self.env));
        widget.set_children(children);
        self.add(widget)
    }

    /// Adds a column whose children are built by `build`.
    pub fn column(&mut self, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        self.container(Axis::Column, build)
    }

    /// Adds a row whose children are built by `build`.
    pub fn row(&mut self, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        self.container(Axis::Row, build)
    }

    /// Adds a stack: children are drawn on top of each other in the same area, later ones on top.
    pub fn stack(&mut self, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        self.container(Axis::Stack, build)
    }

    /// Adds a column that remembers its widgets' state (focus, scroll, cursors) while it is not
    /// shown. Give every page of a router its own `key`.
    pub fn page(&mut self, key: impl Into<String>, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        let node = self.container(Axis::Column, build);
        node.node.persistent = true;
        node.node.key = Key::Named(key.into());
        node.fill()
    }

    /// Adds empty space that takes the room left in a row or column.
    pub fn spacer(&mut self) -> NodeMut<'_, Msg> {
        self.container(Axis::Stack, |_| {}).fill()
    }

    fn container(&mut self, axis: Axis, build: impl FnOnce(&mut View<'_, Msg>)) -> NodeMut<'_, Msg> {
        let mut children = Vec::new();
        build(&mut View::new(&mut children, self.env));
        self.add(Flex::new(axis, children))
    }
}

/// Adjusts the node just added. Every method changes the node in place, so the result can be
/// ignored or chained.
pub struct NodeMut<'a, Msg> {
    node: &'a mut Node<Msg>,
}

impl<'a, Msg> NodeMut<'a, Msg> {
    /// Names the node. Name widgets whose position among their siblings can change (list rows,
    /// optional widgets) so their state and focus follow them.
    pub fn id(self, name: impl Into<String>) -> Self {
        self.node.key = Key::Named(name.into());
        self
    }

    /// Sets the width.
    pub fn width(self, width: Length) -> Self {
        self.node.layout.width = width;
        self
    }

    /// Sets the height.
    pub fn height(self, height: Length) -> Self {
        self.node.layout.height = height;
        self
    }

    /// Takes all space left in both directions.
    pub fn fill(self) -> Self {
        self.width(Length::Fill(1)).height(Length::Fill(1))
    }

    /// Takes all width left.
    pub fn fill_width(self) -> Self {
        self.width(Length::Fill(1))
    }

    /// Takes all height left.
    pub fn fill_height(self) -> Self {
        self.height(Length::Fill(1))
    }

    /// Keeps `padding` free inside the node.
    pub fn padding(self, padding: Padding) -> Self {
        self.node.layout.padding = padding;
        self
    }

    /// Leaves `cells` between the children of a row or column.
    pub fn gap(self, cells: u16) -> Self {
        self.node.layout.gap = cells;
        self
    }

    /// Places children along the main axis of a row or column (both axes of a stack).
    pub fn justify(self, align: Align) -> Self {
        self.node.layout.justify = align;
        self
    }

    /// Whether a mouse drag may select text in this node. Nothing is selectable unless asked:
    /// `true` makes the node a selection region, so a drag that starts inside it selects text
    /// within the node only (widgets such as `CodeView` and `Markdown` are regions by
    /// themselves). `false` keeps selection out of the node and everything inside it, also out
    /// of regions within it, e.g. for a secret shown inside a selectable log.
    pub fn selectable(self, selectable: bool) -> Self {
        self.node.selectable = Some(selectable);
        self
    }

    /// Places children across the main axis of a row or column.
    pub fn align(self, align: Align) -> Self {
        self.node.layout.align = align;
        self
    }
}
