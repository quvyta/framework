//! Keeping focus, hover, captures and layers consistent with the frame just painted.

use std::time::Duration;

use super::Engine;
use crate::runtime::App;
use crate::widget::{FocusRequest, LayerRecord, WidgetId};

impl<A: App> Engine<A> {
    /// Keeps focus, hover and captures pointing at widgets that still exist.
    pub(super) fn settle_interaction(&mut self) {
        if let Some(name) = self.pending_focus.take()
            && let Some(target) = self.named_focusable(&name)
            && self.interaction.focused != Some(target)
        {
            // The frame just painted showed the widget without focus; the next one shows it.
            self.interaction.focused = Some(target);
            self.dirty = true;
        }
        let focusable = &self.frame.focusable;
        if let Some(focused) = self.interaction.focused {
            if focusable.contains(&focused) {
                if let Some(scope) = self.frame.scopes.get(&focused) {
                    self.scope_focus.insert(*scope, focused);
                }
            } else {
                self.interaction.focused = None;
            }
        }
        if self.interaction.focused.is_none() {
            // Several visible scopes may remember a focus; the one earliest in the focus order wins,
            // so the result never depends on hash order.
            let scopes = &self.frame.scopes;
            self.interaction.focused = focusable
                .iter()
                .find(|id| scopes.get(id).and_then(|scope| self.scope_focus.get(scope)) == Some(id))
                .copied();
        }
        for capture in [&mut self.interaction.key_capture, &mut self.interaction.pointer_capture] {
            if capture.is_some_and(|id| !self.frame.rects.contains_key(&id)) {
                *capture = None;
            }
        }
        let hovered = self.pointer.and_then(|(x, y)| self.frame.hit(x, y));
        self.interaction.hovered_chain = hovered.map(|id| self.frame.ancestry(id)).unwrap_or_default();
        if hovered != self.interaction.hovered {
            self.interaction.hovered = hovered;
            self.dirty = true;
        }
    }

    /// Matches the open modal layers with the ones painted this frame. A new layer remembers
    /// the focus of the moment; when layers close, focus is requested back for the widget that
    /// had it before the bottommost closed layer opened, unless a widget asked for focus itself.
    pub(super) fn settle_layers(&mut self, now: Duration) {
        let painted: Vec<WidgetId> =
            self.frame.layers.iter().filter(|layer| layer.modal).map(|layer| layer.id).collect();
        let mut restore = None;
        while let Some(top) = self.interaction.layers.last()
            && !painted.contains(&top.id)
        {
            restore = top.previous_focus;
            self.interaction.layers.pop();
        }
        self.interaction.layers.retain(|layer| painted.contains(&layer.id));
        for id in &painted {
            if !self.interaction.layers.iter().any(|layer| layer.id == *id) {
                // A layer replacing one that just closed hands focus back where the old one would.
                let previous_focus = restore.take().or(self.interaction.focused);
                self.interaction.layers.push(LayerRecord { id: *id, previous_focus, opened: now });
                self.dirty = true;
            }
        }
        if let Some(previous) = restore {
            let back_into_open_layer = self.frame.top_layer().is_some_and(|layer| {
                self.frame.focus_request == Some(FocusRequest::Within(layer)) && self.frame.is_within(previous, layer)
            });
            if self.frame.focus_request.is_none() || back_into_open_layer {
                self.frame.focus_request = Some(FocusRequest::Exact(previous));
            }
            self.dirty = true;
        }
    }

    /// Remembers which widget opened each non-modal layer painted this frame: the one that used
    /// the pointer press just before the layer first appeared. A press on that widget later only
    /// closes the layer, so a dropdown button pressed again does not close and reopen at once.
    pub(super) fn settle_openers(&mut self) {
        let openers = self
            .frame
            .layers
            .iter()
            .filter(|layer| !layer.modal)
            .map(|layer| {
                let known = self.openers.iter().find(|(id, _)| *id == layer.id);
                (layer.id, known.map_or(self.press_target, |(_, opener)| *opener))
            })
            .collect();
        self.openers = openers;
    }

    /// Moves focus as a widget asked while painting.
    pub(super) fn apply_focus_request(&mut self) {
        let focusable = &self.frame.focusable;
        let target = match self.frame.focus_request {
            None => None,
            Some(FocusRequest::Exact(id)) => focusable.contains(&id).then_some(id),
            Some(FocusRequest::Within(owner)) => {
                let inside = self.interaction.focused.is_some_and(|focused| self.frame.is_within(focused, owner));
                if inside { None } else { focusable.iter().find(|id| self.frame.is_within(**id, owner)).copied() }
            }
        };
        if let Some(target) = target
            && self.interaction.focused != Some(target)
        {
            self.interaction.focused = Some(target);
            self.dirty = true;
        }
    }

    pub(super) fn move_focus(&mut self, step: isize) {
        let order: Vec<WidgetId> =
            self.frame.focusable.iter().filter(|id| self.frame.reachable(**id)).copied().collect();
        if order.is_empty() {
            return;
        }
        let len = isize::try_from(order.len()).unwrap_or(isize::MAX);
        let next = match self.interaction.focused.and_then(|id| order.iter().position(|o| *o == id)) {
            Some(index) => (isize::try_from(index).unwrap_or(0) + step).rem_euclid(len),
            None if step > 0 => 0,
            None => len - 1,
        };
        self.interaction.focused = usize::try_from(next).ok().and_then(|i| order.get(i)).copied();
    }

    /// The focusable widget named `name` in the last frame.
    pub(super) fn named_focusable(&self, name: &str) -> Option<WidgetId> {
        self.frame.focusable.iter().find(|id| self.frame.names.get(id).is_some_and(|n| n == name)).copied()
    }
}
