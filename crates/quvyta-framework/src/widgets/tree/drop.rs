//! Dropping dragged nodes into another node, such as files into a folder.
//!
//! A drag carries the selection. The node under the pointer is the target when the application
//! accepts drops on it; a node cannot take itself or one of its own descendants, and a folder
//! cannot take nodes that are already all in it. Every decision is made on the layout the drag
//! started from, so painting a target never moves the rows the pointer is deciding on.

use std::time::Duration;

use crate::geometry::Rect;
use crate::widget::EventCx;

use super::super::edge_scroll::DELAY;
use super::{Flat, Tree, TreeNode};

/// A drop of nodes into another node that a [droppable](Tree::droppable) tree asks for. The tree
/// never changes the application's nodes; the application moves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeDrop {
    /// The keys of the nodes that move, in the order they have in the tree. A node inside another
    /// node that moves is left out: it travels inside it.
    pub keys: Vec<String>,
    /// The key of the node they move into, or `None` for the top level (a drop on the free space
    /// below the last row).
    pub into: Option<String>,
}

/// Builds a message from a drop.
type DropMessage<Msg> = Box<dyn Fn(TreeDrop) -> Msg>;

/// Tells whether the node with a key takes drops.
type DropFilter = Box<dyn Fn(&str) -> bool>;

/// What a droppable tree does with a drop.
pub(super) struct Dropping<Msg> {
    pub(super) message: DropMessage<Msg>,
    pub(super) accepts: DropFilter,
}

impl<Msg> Dropping<Msg> {
    pub(super) fn new(message: impl Fn(TreeDrop) -> Msg + 'static, accepts: impl Fn(&str) -> bool + 'static) -> Self {
        Self { message: Box::new(message), accepts: Box::new(accepts) }
    }
}

/// What the pointer of a drag is over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Aim {
    /// A node, or the top level, that takes the dragged nodes.
    Into(Option<String>),
    /// A node that takes drops but not these: the dragged nodes themselves, one of their
    /// descendants, or the node they are all in already.
    Refused(String),
    /// A place among the dragged node's siblings, as the sibling positions a reorder moves
    /// between; `None` while the pointer is outside the tree.
    Reorder(Option<(usize, usize)>),
    /// Nothing that takes the drag.
    Nothing,
}

/// A closed node that opens when a drag rests on it, kept in the tree's memory.
#[derive(Debug, Default)]
pub(super) struct Spring {
    /// The node the drag rests on and when it opens; `None` once it has.
    pending: Option<(String, Option<Duration>)>,
}

impl<Msg: 'static> Tree<Msg> {
    /// The node with `key`, anywhere in the tree.
    fn find(&self, key: &str) -> Option<&TreeNode> {
        fn walk<'a>(nodes: &'a [TreeNode], key: &str) -> Option<&'a TreeNode> {
            nodes.iter().find_map(|node| if node.key == key { Some(node) } else { walk(&node.children, key) })
        }
        walk(&self.roots, key)
    }

    /// The nodes a press on `key` drags: the whole selection of a droppable tree when `key` is in
    /// it, in tree order and without the nodes inside other dragged nodes; otherwise `key` alone.
    pub(super) fn carried(&self, key: &str) -> Vec<String> {
        fn walk(nodes: &[TreeNode], chosen: &[String], out: &mut Vec<String>) {
            for node in nodes {
                if chosen.contains(&node.key) {
                    out.push(node.key.clone());
                } else {
                    walk(&node.children, chosen, out);
                }
            }
        }
        if self.dropping.is_none() || !self.is_multi() || !self.is_chosen(key) {
            return vec![key.to_owned()];
        }
        let mut out = Vec::new();
        walk(&self.roots, &self.chosen, &mut out);
        out
    }

    /// Whether a drag of `keys` reorders siblings rather than only dropping into nodes: a
    /// reorderable tree dragging one node.
    pub(super) fn reorders(&self, keys: &[String]) -> bool {
        self.on_move.is_some() && keys.len() == 1
    }

    /// The rows a drag of `keys` decides on: the dragged node folded among its resting siblings
    /// when the drag reorders, the tree as it is otherwise.
    pub(super) fn drag_layout(&self, keys: &[String]) -> Vec<Flat<'_>> {
        match keys {
            [key] if self.reorders(keys) => self.resting(key),
            _ => self.flatten(),
        }
    }

    /// Whether `into` cannot take `keys`: it is one of them or inside one of them, or they are
    /// all in it already.
    fn refuses(&self, keys: &[String], into: Option<&str>) -> bool {
        fn holds(node: &TreeNode, key: &str) -> bool {
            node.children.iter().any(|child| child.key == key || holds(child, key))
        }
        let parent = |key: &str| self.siblings(key).map(|(parent, _)| parent);
        if keys.iter().all(|key| parent(key) == Some(into)) {
            return true;
        }
        into.is_some_and(|into| {
            keys.iter().any(|key| key == into || self.find(key).is_some_and(|node| holds(node, into)))
        })
    }

    /// What a drag of `keys` with the pointer at `pointer` is over, with the tree's rows in `area`
    /// scrolled by `offset`. A node that takes drops wins over a place among siblings, except the
    /// dragged node's own row, which is where a reorder leaves it.
    pub(super) fn aim(&self, keys: &[String], pointer: (i32, i32), area: Rect, offset: usize) -> Aim {
        if let Some(dropping) = &self.dropping
            && area.contains(pointer.0, pointer.1)
        {
            let flat = self.drag_layout(keys);
            let index = offset + usize::try_from(pointer.1 - area.y).unwrap_or(0);
            match flat.get(index) {
                Some(row) => {
                    let key = &row.node.key;
                    let own_slot = self.reorders(keys) && keys.first() == Some(key);
                    if (dropping.accepts)(key) && !own_slot {
                        return if self.refuses(keys, Some(key)) {
                            Aim::Refused(key.clone())
                        } else {
                            Aim::Into(Some(key.clone()))
                        };
                    }
                }
                None if !self.refuses(keys, None) => return Aim::Into(None),
                None => {}
            }
        }
        match keys {
            [key] if self.reorders(keys) => Aim::Reorder(self.landing(key, pointer, area, offset)),
            _ => Aim::Nothing,
        }
    }

    /// Sends the drop or the reorder a drag of `keys` released at `aim` asks for: a copy rather
    /// than a move when `copy` (Ctrl held at the release) and the tree copies.
    pub(super) fn release(&self, cx: &mut EventCx<'_, Msg>, keys: Vec<String>, aim: Aim, copy: bool) {
        match aim {
            Aim::Into(into) => {
                let drop = TreeDrop { keys, into };
                match (&self.copy_drop, &self.dropping) {
                    (Some(copying), Some(_)) if copy => cx.emit(copying(drop)),
                    (_, Some(dropping)) => cx.emit((dropping.message)(drop)),
                    (_, None) => {}
                }
            }
            Aim::Reorder(Some((from, to))) => {
                let Some(key) = keys.first() else { return };
                let parent = self.siblings(key).and_then(|(parent, _)| parent.map(str::to_owned));
                self.move_node(cx, key, parent.as_deref(), from, to);
            }
            Aim::Reorder(None) | Aim::Refused(_) | Aim::Nothing => {}
        }
    }

    /// Opens a closed node a drag rests on for [`DELAY`], the wait desktop file managers give a
    /// folder before it springs open, so a drop can reach nodes inside it. Passing over a node on
    /// the way does not open it. Returns when the pointer should be woken next for it, since
    /// terminals send nothing while the pointer is held still.
    pub(super) fn spring(&self, cx: &mut EventCx<'_, Msg>, aim: &Aim) -> Option<Duration> {
        let now = cx.now();
        let node = match aim {
            Aim::Into(Some(key)) => self.find(key).filter(|node| node.expandable && !node.expanded),
            _ => None,
        };
        let Some(node) = node else {
            cx.memory::<Spring>().pending = None;
            return None;
        };
        let pending = cx.memory::<Spring>().pending.clone();
        match pending {
            Some((key, due)) if key == node.key => {
                let due = due?;
                if now < due {
                    return Some(due - now);
                }
                cx.memory::<Spring>().pending = Some((key, None));
                self.expand(cx, node, true);
                None
            }
            _ => {
                cx.memory::<Spring>().pending = Some((node.key.clone(), Some(now + DELAY)));
                Some(DELAY)
            }
        }
    }
}
