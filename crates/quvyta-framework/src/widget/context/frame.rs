//! What a painted frame leaves behind for routing input, and the interaction state the runtime
//! keeps between frames.

use std::time::Duration;

use crate::geometry::{Rect, Size};
use crate::keymap::KeyChord;
use crate::widget::{IdMap, WidgetId};

/// What was painted in a frame; used to route the next events.
#[derive(Debug, Default)]
pub(crate) struct Frame {
    pub(crate) rects: IdMap<WidgetId, Rect>,
    pub(crate) parents: IdMap<WidgetId, WidgetId>,
    pub(crate) names: IdMap<WidgetId, String>,
    pub(crate) scopes: IdMap<WidgetId, WidgetId>,
    pub(crate) hits: Vec<(Rect, WidgetId)>,
    pub(crate) focusable: Vec<WidgetId>,
    pub(crate) overlays: Vec<(WidgetId, Rect)>,
    pub(crate) next_frame: Option<Duration>,
    /// Chords widgets listen to without focus; see [`PaintCx::listen_key`](crate::widget::PaintCx::listen_key).
    pub(crate) listeners: Vec<(KeyChord, WidgetId)>,
    /// Areas where a mouse press never starts a text selection.
    pub(crate) unselectable: Vec<Rect>,
    /// Visible areas where a mouse drag selects text, with the widget each belongs to, in paint
    /// order; see [`PaintCx::selectable`](crate::widget::PaintCx::selectable).
    pub(crate) selectable: Vec<(Rect, WidgetId)>,
    /// Cells that decorate rather than hold content (pillars, scrollbars); clean copies of a
    /// text selection leave them out. See [`PaintCx::decoration`](crate::widget::PaintCx::decoration).
    pub(crate) decorations: Vec<Rect>,
    /// Every layer painted this frame, in paint order (the last one is on top): dismissable
    /// layers such as popovers and modal layers such as dialogs share one stack.
    pub(crate) layers: Vec<LayerEntry>,
    /// Where keyboard focus should move once the frame is painted.
    pub(crate) focus_request: Option<FocusRequest>,
    /// Sizes measured while painting this frame; see [`MeasureKey`].
    pub(crate) measures: IdMap<MeasureKey, Size>,
}

/// A node measured in a frame: its id, its address in the view tree and the space it was
/// offered. Measuring depends only on the node, the environment and that space, none of which
/// change while a frame is painted, so containers that measure their children again (a flex
/// row before painting, a scroll view for its content height) reuse the first answer instead of
/// measuring whole subtrees once per level of nesting. The address tells apart nodes that share
/// an id by mistake.
pub(crate) type MeasureKey = (WidgetId, usize, Size);

/// One layer of the frame's layer stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerEntry {
    pub(crate) id: WidgetId,
    /// Modal layers trap focus and input; see [`PaintCx::open_layer`](crate::widget::PaintCx::open_layer). Other layers only close
    /// on presses outside them and unused Esc; see [`PaintCx::register_dismissable`](crate::widget::PaintCx::register_dismissable).
    pub(crate) modal: bool,
    /// Where a modal layer's surface sits, once the layer said so; toasts keep clear of it.
    /// `None` counts as the whole screen.
    pub(crate) surface: Option<Rect>,
}

/// A focus change asked for while painting, applied after the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusRequest {
    /// The first focusable widget inside this one, unless focus is already inside.
    Within(WidgetId),
    /// Exactly this widget, when it is focusable.
    Exact(WidgetId),
}

impl Frame {
    /// Empties the frame for painting the next one, keeping the memory its maps and lists
    /// already grew, so a steady view allocates nothing for them.
    pub(crate) fn clear(&mut self) {
        // Naming every field makes a new field impossible to forget here.
        let Self {
            rects,
            parents,
            names,
            scopes,
            hits,
            focusable,
            overlays,
            next_frame,
            listeners,
            unselectable,
            selectable,
            decorations,
            layers,
            focus_request,
            measures,
        } = self;
        rects.clear();
        parents.clear();
        names.clear();
        scopes.clear();
        hits.clear();
        focusable.clear();
        overlays.clear();
        *next_frame = None;
        listeners.clear();
        unselectable.clear();
        selectable.clear();
        decorations.clear();
        layers.clear();
        *focus_request = None;
        measures.clear();
    }

    /// The topmost widget whose hit area contains the cell.
    pub(crate) fn hit(&self, x: i32, y: i32) -> Option<WidgetId> {
        self.hits.iter().rev().find(|(rect, _)| rect.contains(x, y)).map(|(_, id)| *id)
    }

    /// `id` and its ancestors, innermost first.
    pub(crate) fn ancestry(&self, id: WidgetId) -> Vec<WidgetId> {
        let mut chain = vec![id];
        let mut current = id;
        while let Some(parent) = self.parents.get(&current) {
            chain.push(*parent);
            current = *parent;
        }
        chain
    }

    /// The topmost modal layer, if one is open.
    pub(crate) fn top_layer(&self) -> Option<WidgetId> {
        self.layers.iter().rev().find(|layer| layer.modal).map(|layer| layer.id)
    }

    /// Whether `id` may receive input while modal layers are open: it lies inside the topmost
    /// modal layer, or there is none.
    pub(crate) fn reachable(&self, id: WidgetId) -> bool {
        self.top_layer().is_none_or(|layer| self.is_within(id, layer))
    }

    /// `id` and its ancestors up to and including the topmost modal layer, so input inside a
    /// layer never bubbles to the widgets beneath it. Without a layer this is the whole ancestry.
    pub(crate) fn routed_ancestry(&self, id: WidgetId) -> Vec<WidgetId> {
        let mut chain = self.ancestry(id);
        if let Some(layer) = self.top_layer() {
            match chain.iter().position(|ancestor| *ancestor == layer) {
                Some(index) => chain.truncate(index + 1),
                None => chain = vec![layer],
            }
        }
        chain
    }

    /// Whether `descendant` is `ancestor` or lies inside it.
    pub(crate) fn is_within(&self, descendant: WidgetId, ancestor: WidgetId) -> bool {
        self.ancestry(descendant).contains(&ancestor)
    }

    /// Asks for a frame at `at`, keeping an earlier request.
    pub(super) fn schedule(&mut self, at: Duration) {
        self.next_frame = Some(self.next_frame.map_or(at, |current| current.min(at)));
    }
}

/// Pointer and keyboard state the runtime tracks for widgets.
#[derive(Debug, Default, Clone)]
pub(crate) struct Interaction {
    pub(crate) pointer: Option<(i32, i32)>,
    pub(crate) hovered: Option<WidgetId>,
    /// The hovered widget and its ancestors, innermost first.
    pub(crate) hovered_chain: Vec<WidgetId>,
    pub(crate) focused: Option<WidgetId>,
    pub(crate) pressed: Option<(WidgetId, Duration)>,
    pub(crate) pointer_capture: Option<WidgetId>,
    pub(crate) key_capture: Option<WidgetId>,
    /// Whether the last input was a mouse press, so focus came from the pointer; pressables
    /// then keep a quiet focus look and breathe only when reached with the keyboard.
    pub(crate) focus_by_pointer: bool,
    /// Open modal layers, bottom first.
    pub(crate) layers: Vec<LayerRecord>,
    /// Whether the clipboard had text when it was read last, or the application copied since:
    /// a Paste menu entry is enabled only then.
    pub(crate) can_paste: bool,
}

/// A modal layer the runtime has seen open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerRecord {
    pub(crate) id: WidgetId,
    /// The widget focused before the layer opened; focus returns there when it closes.
    pub(crate) previous_focus: Option<WidgetId>,
    /// When the layer first appeared.
    pub(crate) opened: Duration,
}
