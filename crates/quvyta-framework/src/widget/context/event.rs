//! The event handling context.

use std::time::Duration;

use super::frame::Interaction;
use crate::env::Env;
use crate::event::Event;
use crate::geometry::Rect;
use crate::keymap::Scope;
use crate::runtime::CopyKind;
use crate::widget::memory::Memory;
use crate::widget::{Node, WidgetId};

/// Requests widgets make of the runtime while handling an event.
#[derive(Debug, Default)]
pub(crate) struct Effects {
    pub(crate) focus: Option<WidgetId>,
    pub(crate) key_capture: Option<Option<WidgetId>>,
    pub(crate) pointer_capture: bool,
    pub(crate) flash: Option<WidgetId>,
    pub(crate) copy: Vec<String>,
    pub(crate) run_action: Option<(Scope, String)>,
    pub(crate) pointer_repeat: Option<Duration>,
    /// End this widget's pointer repeat, see [`EventCx::stop_pointer_repeat`].
    pub(crate) stop_pointer_repeat: bool,
    pub(crate) answer: Option<bool>,
    pub(crate) focus_step: Option<isize>,
    /// Read the clipboard to learn whether pasting is possible, see [`EventCx::probe_clipboard`].
    pub(crate) probe_clipboard: bool,
    /// Copy the mouse selection, see [`EventCx::copy_selection`].
    pub(crate) copy_selection: Option<CopyKind>,
}

/// Event handling context.
pub struct EventCx<'a, Msg> {
    pub(crate) id: WidgetId,
    pub(crate) rect: Rect,
    pub(crate) focus_rect: Option<Rect>,
    pub(crate) env: &'a Env,
    pub(crate) memory: &'a mut Memory,
    pub(crate) interaction: &'a Interaction,
    pub(crate) messages: &'a mut Vec<Msg>,
    pub(crate) effects: &'a mut Effects,
    pub(crate) now: Duration,
    pub(crate) persistent: bool,
    /// Whether this is a press shown to a widget before the widgets inside it, see
    /// [`PaintCx::preview_presses`](super::PaintCx::preview_presses).
    pub(crate) preview: bool,
}

impl<Msg> EventCx<'_, Msg> {
    /// Whether the event is a press shown to this widget before the widgets inside it, because
    /// it asked with [`PaintCx::preview_presses`](super::PaintCx::preview_presses). Using it
    /// keeps it from them; leaving it lets it go on as usual, to this widget too.
    pub(crate) fn is_preview(&self) -> bool {
        self.preview
    }

    /// The id of the widget handling the event.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        self.id
    }

    /// The area the widget was painted in during the last frame.
    #[must_use]
    pub fn area(&self) -> Rect {
        self.rect
    }

    /// The area the focused widget was painted in during the last frame, if a widget has focus.
    /// Lets a container place something next to the focused child, e.g. a context menu opened
    /// from the keyboard.
    #[must_use]
    pub fn focused_area(&self) -> Option<Rect> {
        self.focus_rect
    }

    /// The environment.
    #[must_use]
    pub fn env(&self) -> &Env {
        self.env
    }

    /// Time since the runtime started.
    #[must_use]
    pub fn now(&self) -> Duration {
        self.now
    }

    /// Sends a message to the application.
    pub fn emit(&mut self, message: Msg) {
        self.messages.push(message);
    }

    /// This widget's state of type `T`.
    pub fn memory<T: Default + 'static>(&mut self) -> &mut T {
        self.memory.get::<T>(self.id, self.persistent)
    }

    /// Whether this widget has keyboard focus.
    #[must_use]
    pub fn is_focused(&self) -> bool {
        self.interaction.focused == Some(self.id)
    }

    /// Moves keyboard focus to this widget.
    pub fn request_focus(&mut self) {
        self.effects.focus = Some(self.id);
    }

    /// Moves keyboard focus to the next widget in focus order, as Tab does. Forms use it to go
    /// to the next field on Enter.
    pub fn focus_next(&mut self) {
        self.effects.focus_step = Some(1);
    }

    /// Offers `event` to a child `node` painted in `rect`, as if the child had received it: the
    /// child keeps its own memory, and its messages and requests go out with this widget's. For
    /// widgets that take focus as one control and let a child act, e.g. a settings row passing
    /// Space to its switch. Returns whether the child used the event.
    pub fn forward(&mut self, node: &Node<Msg>, rect: Rect, event: &Event) -> bool
    where
        Msg: 'static,
    {
        let mut child = EventCx {
            id: node.id,
            rect,
            focus_rect: self.focus_rect,
            env: self.env,
            memory: &mut *self.memory,
            interaction: self.interaction,
            messages: &mut *self.messages,
            effects: &mut *self.effects,
            now: self.now,
            persistent: self.persistent,
            preview: self.preview,
        };
        node.widget.event(&mut child, event)
    }

    /// While on, every key event goes to this widget first (an open dropdown), and a pointer
    /// press on any other widget, including one inside it, first sends it
    /// [`Event::PointerOutside`](crate::event::Event::PointerOutside) and then reaches that widget
    /// as usual. A press on this widget itself reaches only this widget.
    pub fn capture_keys(&mut self, on: bool) {
        self.effects.key_capture = Some(on.then_some(self.id));
    }

    /// Keeps pointer events flowing to this widget until the button is released.
    pub fn capture_pointer(&mut self) {
        self.effects.pointer_capture = true;
    }

    /// Flashes this widget to confirm an activation.
    pub fn flash(&mut self) {
        self.effects.flash = Some(self.id);
    }

    /// Copies `text` to the system clipboard.
    pub fn copy(&mut self, text: impl Into<String>) {
        self.effects.copy.push(text.into());
    }

    /// Runs keymap action `action` of `scope` as if its key had been pressed, after this event.
    /// Application actions reach [`App::action`](crate::runtime::App::action) even while a
    /// modal layer is open, since the user asked for them explicitly (e.g. from a command
    /// palette).
    pub fn run_action(&mut self, scope: Scope, action: impl Into<String>) {
        self.effects.run_action = Some((scope, action.into()));
    }

    /// While the pointer is captured, delivers a `Drag` event at the last pointer position to
    /// this widget every `interval` until the button is released, as if the pointer moved in
    /// place. Terminals send nothing while a button is held still; this lets a widget react
    /// to how long it is held (hold-to-confirm, auto-repeating steppers). Call it together
    /// with [`EventCx::capture_pointer`] on the button press.
    pub fn repeat_pointer(&mut self, interval: Duration) {
        self.effects.pointer_repeat = Some(interval.max(Duration::from_millis(1)));
    }

    /// Ends the repeat [`EventCx::repeat_pointer`] started for this widget before the button is
    /// released, so a widget that needs timed drags only for a while (scrolling while a dragged
    /// item rests against an edge) does not keep waking the loop afterwards. A repeat asked for in
    /// the same event wins. Nothing happens when this widget has no repeat running.
    pub fn stop_pointer_repeat(&mut self) {
        self.effects.stop_pointer_repeat = true;
    }

    /// Runs `handle` with a context whose messages are of type `M` instead of `Msg`, and returns
    /// its result with the messages it sent; everything else (memory, focus, captures, copies)
    /// is this widget's. Lets a widget drive an inner widget of its own, such as the edit menu of
    /// a text field, whose choices are the field's business rather than the application's.
    pub(crate) fn with_messages<M, R>(&mut self, handle: impl FnOnce(&mut EventCx<'_, M>) -> R) -> (R, Vec<M>) {
        let mut messages = Vec::new();
        let result = {
            let mut inner = EventCx {
                id: self.id,
                rect: self.rect,
                focus_rect: self.focus_rect,
                env: self.env,
                memory: &mut *self.memory,
                interaction: self.interaction,
                messages: &mut messages,
                effects: &mut *self.effects,
                now: self.now,
                persistent: self.persistent,
                preview: self.preview,
            };
            handle(&mut inner)
        };
        (result, messages)
    }

    /// Reads the clipboard in the background so `can_paste` on this context and on
    /// [`PaintCx`](crate::widget::PaintCx) soon tells whether it has text, e.g. when an edit menu
    /// opens.
    pub(crate) fn probe_clipboard(&mut self) {
        self.effects.probe_clipboard = true;
    }

    /// Whether pasting would insert text, as far as the runtime knows.
    pub(crate) fn can_paste(&self) -> bool {
        self.interaction.can_paste
    }

    /// Copies the runtime's mouse selection as `kind`.
    pub(crate) fn copy_selection(&mut self, kind: CopyKind) {
        self.effects.copy_selection = Some(kind);
    }

    /// Resolves the confirmation dialog the runtime shows for
    /// [`Command::confirm`](crate::runtime::Command::confirm).
    pub(crate) fn answer(&mut self, confirmed: bool) {
        self.effects.answer = Some(confirmed);
    }
}
