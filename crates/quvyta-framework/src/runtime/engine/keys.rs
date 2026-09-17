//! Routing keys: focused widgets, key listeners, layers and keymap actions.

use std::time::Duration;

use super::Engine;
use super::clipboard::ClipboardRead;
use crate::event::{Event, KeyEvent, KeyKind};
use crate::keymap::{Key, KeyChord, Modifiers, Scope};
use crate::runtime::App;
use crate::runtime::selection::CopyKind;
use crate::widget::WidgetId;

/// Enter or Space presses closer together than this are typematic repeat, not new presses.
const HELD_KEY_WINDOW: Duration = Duration::from_millis(100);

/// Global keymap actions the runtime or its widgets own. Other global actions, such as `help`
/// and `palette`, are passed to [`App::action`].
const RUNTIME_ACTIONS: [&str; 7] = ["quit", "focus-next", "focus-prev", "debug", "copy", "paste", "toggle-panel"];

impl<A: App> Engine<A> {
    pub(super) fn keyboard_targets(&self) -> Vec<WidgetId> {
        match self.interaction.key_capture.or(self.interaction.focused) {
            Some(id) => self.frame.routed_ancestry(id),
            None => self.frame.top_layer().into_iter().collect(),
        }
    }

    /// Offers `key` to the widgets listening to it, see [`PaintCx::listen_key`]. Releases match
    /// the key alone, since modifiers are often let go first.
    pub(super) fn offer_to_listeners(&mut self, key: KeyEvent, now: Duration) -> bool {
        let listeners: Vec<WidgetId> = self
            .frame
            .listeners
            .iter()
            .filter(
                |(chord, _)| {
                    if key.kind == KeyKind::Release { chord.key == key.chord.key } else { *chord == key.chord }
                },
            )
            .map(|(_, id)| *id)
            .filter(|id| self.frame.reachable(*id))
            .collect();
        let mut used = false;
        for id in listeners {
            used |= self.dispatch(&[id], &Event::Key(key), now).is_some();
        }
        used
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent, now: Duration) {
        if key.kind == KeyKind::Release {
            self.offer_to_listeners(key, now);
            return;
        }
        // A layer the keyboard opens has no opening press.
        self.press_target = None;
        // While the selection's menu is open its keys belong to the menu, which may copy.
        let menu_open = self.selection_menu.is_some() && self.interaction.key_capture == self.selection_menu;
        if self.selection.is_some() && !menu_open {
            self.dirty = true;
            let copy = self.env.keymap().action_for(key.chord) == Some((Scope::Global, "copy"));
            if copy {
                if let Some(selection) = &mut self.selection {
                    selection.copy_pending = Some(CopyKind::Clean);
                }
                return;
            }
            self.selection = None;
        }
        let activation = matches!(key.chord.key, Key::Enter | Key::Space) && key.chord.mods == Modifiers::default();
        if activation {
            let repeated = key.kind == KeyKind::Repeat
                || self
                    .last_activation_key
                    .is_some_and(|(last, at)| last == key.chord.key && now.saturating_sub(at) < HELD_KEY_WINDOW);
            self.last_activation_key = Some((key.chord.key, now));
            if repeated {
                // A held key must not press buttons again, but listeners measuring the hold
                // still hear it.
                self.offer_to_listeners(key, now);
                return;
            }
        }
        self.dirty = true;
        let targets = self.keyboard_targets();
        if self.dispatch(&targets, &Event::Key(key), now).is_some() {
            return;
        }
        if self.interaction.key_capture.is_some() {
            return;
        }
        if key.chord == KeyChord::plain(Key::Esc)
            && let Some(layer) = self.frame.layers.last().map(|layer| layer.id)
            && self.dispatch(&[layer], &Event::Key(key), now).is_some()
        {
            return;
        }
        if self.offer_to_listeners(key, now) {
            return;
        }
        let Some((scope, action)) = self.env.keymap().action_for(key.chord).map(|(s, a)| (s, a.to_owned())) else {
            return;
        };
        self.run_action(scope, &action, self.frame.top_layer().is_none(), now);
    }

    /// Runs a keymap action. Global actions the runtime owns run here; other global actions and
    /// application actions reach [`App::action`] when `to_app` is true, which it is not while a
    /// modal layer pauses shortcuts.
    pub(super) fn run_action(&mut self, scope: Scope, action: &str, to_app: bool, now: Duration) {
        match (scope, action) {
            (Scope::Global, "quit") => self.quit = true,
            (Scope::Global, "focus-next") => self.move_focus(1),
            (Scope::Global, "focus-prev") => self.move_focus(-1),
            (Scope::Global, "debug") => self.debug = !self.debug,
            (Scope::Global, "paste") => self.read_clipboard(ClipboardRead::Paste, now),
            // Owned by the mouse selection and by side panels, which handle their keys first.
            (Scope::Global, name) if RUNTIME_ACTIONS.contains(&name) => {}
            (_, name) => {
                if to_app && let Some(message) = self.app.action(name) {
                    self.update(message);
                }
            }
        }
    }
}
