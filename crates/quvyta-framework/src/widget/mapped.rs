//! Part of a view built with another message type, see [`View::map`](super::View::map).
//!
//! The runtime walks the tree of one message type: it finds the widget an event is for, and the
//! widget whose overlay it paints, by id. A screen's widgets send the screen's messages, so they
//! sit in a subtree of that type under a [`Mapped`] node. The node lays the subtree out and paints
//! it as a column, gives it ids, and lets the walks reach into it: a widget found there handles
//! its event with a context of the screen's messages, and what it sends goes out converted.

use super::context::{EventCx, MeasureCx, PaintCx};
use super::{Flex, LayoutProps, Node, Widget, WidgetId};
use crate::event::Event;
use crate::geometry::{Rect, Size};

/// A node of the tree the runtime reached by id, whatever message type it was built with.
pub(crate) trait Reached<Msg> {
    /// The node's id.
    fn id(&self) -> WidgetId;
    /// The node's layout properties.
    fn layout(&self) -> LayoutProps;
    /// Offers `event` to the node's widget; what it sends arrives in `cx` as `Msg`.
    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool;
    /// Paints the widget's overlay.
    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect);
}

impl<Msg: 'static> Reached<Msg> for &Node<Msg> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self) -> LayoutProps {
        self.layout
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        self.widget.event(cx, event)
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        self.widget.paint_overlay(cx, anchor);
    }
}

/// A node found inside a subtree of message type `Inner`, seen as a node of `Msg`.
struct ReachedThrough<'a, Inner, Msg> {
    inner: Box<dyn Reached<Inner> + 'a>,
    map: &'a dyn Fn(Inner) -> Msg,
}

impl<Inner, Msg> Reached<Msg> for ReachedThrough<'_, Inner, Msg> {
    fn id(&self) -> WidgetId {
        self.inner.id()
    }

    fn layout(&self) -> LayoutProps {
        self.inner.layout()
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let (used, messages) = cx.with_messages(|inner| self.inner.event(inner, event));
        for message in messages {
            cx.emit((self.map)(message));
        }
        used
    }

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        self.inner.paint_overlay(cx, anchor);
    }
}

/// The widgets of a subtree built with messages of type `Inner`, in a column, with their
/// conversion.
struct Subtree<Inner, Msg> {
    column: Flex<Inner>,
    map: Box<dyn Fn(Inner) -> Msg>,
}

/// A [`Subtree`] as the node of `Msg` holding it sees it. The node's type cannot name the
/// screen's message type, so this erases it; it is the only thing the node needs.
trait Erased<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size;
    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect);
    fn assign_ids(&mut self, parent: WidgetId);
    fn find(&self, id: WidgetId) -> Option<Box<dyn Reached<Msg> + '_>>;
    fn count_focusable(&self) -> usize;
}

impl<Inner: 'static, Msg: 'static> Erased<Msg> for Subtree<Inner, Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.column.measure(cx, available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        self.column.paint(cx, area);
    }

    fn assign_ids(&mut self, parent: WidgetId) {
        for child in self.column.children_mut() {
            child.assign_ids(parent);
        }
    }

    fn find(&self, id: WidgetId) -> Option<Box<dyn Reached<Msg> + '_>> {
        let found = self.column.children().iter().find_map(|child| child.find(id))?;
        Some(Box::new(ReachedThrough { inner: found, map: &*self.map }))
    }

    fn count_focusable(&self) -> usize {
        self.column.children().iter().map(Node::count_focusable).sum()
    }
}

/// The node of a subtree built with another message type: a column whose children send their
/// messages through a conversion.
pub(crate) struct Mapped<Msg> {
    subtree: Box<dyn Erased<Msg>>,
}

impl<Msg: 'static> Mapped<Msg> {
    /// A column of `children`, whose messages `map` converts.
    pub(crate) fn new<Inner: 'static>(children: Vec<Node<Inner>>, map: impl Fn(Inner) -> Msg + 'static) -> Self {
        let column = Flex::new(super::Axis::Column, children);
        Self { subtree: Box::new(Subtree { column, map: Box::new(map) }) }
    }

    /// Gives the subtree's nodes their ids below the mapped node `parent`.
    pub(crate) fn assign_ids(&mut self, parent: WidgetId) {
        self.subtree.assign_ids(parent);
    }

    /// The node with `id` in the subtree.
    pub(crate) fn find(&self, id: WidgetId) -> Option<Box<dyn Reached<Msg> + '_>> {
        self.subtree.find(id)
    }

    /// How many focusable widgets the subtree holds.
    pub(crate) fn count_focusable(&self) -> usize {
        self.subtree.count_focusable()
    }
}

impl<Msg: 'static> Widget<Msg> for Mapped<Msg> {
    fn measure(&self, cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.subtree.measure(cx, available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        self.subtree.paint(cx, area);
    }
}
