//! The widget model: the [`Widget`] trait, view nodes, layout properties and the contexts
//! widgets measure, paint and handle events with.
//!
//! An application's `view` builds a fresh tree of [`Node`]s every frame through [`View`].
//! The runtime assigns every node a [`WidgetId`], lays the tree out while painting it, and
//! keeps the tree until the next frame so input can reach the widgets that were on screen.

mod context;
mod flex;
mod id;
mod idle;
mod mapped;
#[cfg(test)]
mod mapped_rules;
mod memory;
mod view;

use std::any::{Any, type_name};

pub use context::{EventCx, MeasureCx, PaintCx};
pub use id::WidgetId;
pub use view::{NodeMut, View};

pub(crate) use context::{Effects, FocusRequest, Frame, Interaction, LayerRecord};
pub(crate) use flex::{Axis, Flex};
pub(crate) use id::{IdMap, Key};
pub(crate) use idle::{IdleScope, IdleWatch};
pub(crate) use mapped::Reached;
pub(crate) use memory::Memory;

use mapped::Mapped;

use crate::event::Event;
use crate::geometry::{Padding, Rect, Size};

/// Something that can be laid out, painted and interacted with.
///
/// Widgets are plain values created in `view` every frame. Anything that must survive between
/// frames and is not application data (a cursor position, a scroll offset) is kept in runtime
/// memory through [`PaintCx::memory`] and [`EventCx::memory`].
pub trait Widget<Msg>: 'static {
    /// The size the widget wants when it may use up to `available`.
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size;

    /// Draws the widget into `area`.
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect);

    /// Draws the widget's overlay after the whole view was painted, when it asked for one with
    /// [`PaintCx::request_overlay`]. `anchor` is the area the widget was painted in.
    fn paint_overlay(&self, _cx: &mut PaintCx<'_>, _anchor: Rect) {}

    /// Handles input. Returns `true` when the event was used; unused key and scroll events
    /// bubble to the parent widget.
    fn event(&self, _cx: &mut EventCx<'_, Msg>, _event: &Event) -> bool {
        false
    }

    /// Whether the widget can take keyboard focus.
    fn focusable(&self) -> bool {
        false
    }

    /// Child nodes, for widgets that contain other widgets.
    fn children(&self) -> &[Node<Msg>] {
        &[]
    }

    /// Mutable child nodes, used to assign ids.
    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut []
    }
}

/// A widget as a node stores it: one that can also be told apart by its type, so the tree walks
/// recognise the node of a part built with another message type.
pub(crate) trait StoredWidget<Msg>: Widget<Msg> + Any {}

impl<Msg, W: Widget<Msg>> StoredWidget<Msg> for W {}

/// A widget whose children are built with a closure, see [`View::add_with`].
pub trait Container<Msg>: Widget<Msg> {
    /// Receives the children built for this widget.
    fn set_children(&mut self, children: Vec<Node<Msg>>);
}

/// How much space a node takes along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Length {
    /// As much as the widget measures.
    #[default]
    Auto,
    /// Exactly this many cells.
    Cells(u16),
    /// A share of the space left after `Auto` and `Cells` siblings, by weight.
    Fill(u16),
}

/// Where children sit along an axis when there is room left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// At the start.
    #[default]
    Start,
    /// In the middle.
    Center,
    /// At the end.
    End,
}

/// Layout properties of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayoutProps {
    /// Width.
    pub width: Length,
    /// Height.
    pub height: Length,
    /// Space kept free inside the node.
    pub padding: Padding,
    /// Cells between children of a row or column.
    pub gap: u16,
    /// Placement of children along the main axis of a row or column, or both axes of a stack.
    pub justify: Align,
    /// Placement of children across the main axis.
    pub align: Align,
}

/// One widget in the view tree with its layout properties.
pub struct Node<Msg> {
    pub(crate) key: Key,
    pub(crate) id: WidgetId,
    pub(crate) type_name: &'static str,
    pub(crate) layout: LayoutProps,
    pub(crate) persistent: bool,
    /// `Some(true)` makes the node a text selection region, `Some(false)` keeps selection out.
    pub(crate) selectable: Option<bool>,
    pub(crate) widget: Box<dyn StoredWidget<Msg>>,
}

impl<Msg: 'static> Node<Msg> {
    pub(crate) fn new<W: Widget<Msg>>(widget: W, index: usize) -> Self {
        Self {
            key: Key::Index(index),
            id: WidgetId::ROOT,
            type_name: type_name::<W>(),
            layout: LayoutProps::default(),
            persistent: false,
            selectable: None,
            widget: Box::new(widget),
        }
    }

    /// The node's id; valid once the view has been built.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// The node's layout properties.
    #[must_use]
    pub fn layout(&self) -> LayoutProps {
        self.layout
    }

    /// Gives this node and its descendants their ids.
    pub(crate) fn assign_ids(&mut self, parent: WidgetId) {
        self.id = parent.child(&self.key, self.type_name);
        let id = self.id;
        if let Some(mapped) = (&mut *self.widget as &mut dyn Any).downcast_mut::<Mapped<Msg>>() {
            mapped.assign_ids(id);
            return;
        }
        for child in self.widget.children_mut() {
            child.assign_ids(id);
        }
    }

    /// Finds the node with `id` in this subtree, also inside parts built with another message
    /// type.
    pub(crate) fn find(&self, id: WidgetId) -> Option<Box<dyn Reached<Msg> + '_>> {
        if self.id == id {
            return Some(Box::new(self));
        }
        if let Some(mapped) = self.mapped() {
            return mapped.find(id);
        }
        self.widget.children().iter().find_map(|child| child.find(id))
    }

    /// How many focusable widgets this subtree holds, also inside parts built with another
    /// message type.
    pub(crate) fn count_focusable(&self) -> usize {
        let own = usize::from(self.widget.focusable());
        match self.mapped() {
            Some(mapped) => own + mapped.count_focusable(),
            None => own + self.widget.children().iter().map(Self::count_focusable).sum::<usize>(),
        }
    }

    /// The part built with another message type this node holds, if it is one.
    fn mapped(&self) -> Option<&Mapped<Msg>> {
        (&*self.widget as &dyn Any).downcast_ref::<Mapped<Msg>>()
    }
}
