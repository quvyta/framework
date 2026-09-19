//! Routing the pointer: presses, drags, scrolling, layers a press closes and held-still repeats.

use std::time::Duration;

use super::Engine;
use crate::event::{Event, MouseButton, MouseEvent, MouseKind};
use crate::keymap::Modifiers;
use crate::runtime::App;
use crate::runtime::selection::{Press, Selection};
use crate::widget::WidgetId;
use crate::widgets::ToastPress;

/// Synthetic drag events for a widget holding the pointer still, see
/// [`EventCx::repeat_pointer`](crate::widget::EventCx::repeat_pointer).
#[derive(Debug, Clone, Copy)]
pub(super) struct PointerRepeat {
    pub(super) owner: WidgetId,
    pub(super) button: MouseButton,
    pub(super) interval: Duration,
    pub(super) next: Duration,
}

impl<A: App> Engine<A> {
    /// The time at which an animation, a held pointer or idleness needs the next frame, if any.
    pub(crate) fn deadline(&self) -> Option<Duration> {
        let repeat = self.pointer_repeat.map(|repeat| repeat.next);
        [self.frame.next_frame, repeat, self.idle.deadline()].into_iter().flatten().min()
    }

    /// Delivers timed input that is due at `now`: repeated drags for a pointer held still.
    pub(crate) fn tick(&mut self, now: Duration) {
        let Some(repeat) = self.pointer_repeat else {
            return;
        };
        if now < repeat.next {
            return;
        }
        if self.interaction.pointer_capture != Some(repeat.owner) {
            self.pointer_repeat = None;
            return;
        }
        self.pointer_repeat = Some(PointerRepeat { next: now + repeat.interval, ..repeat });
        let (x, y) = self.pointer.unwrap_or_default();
        let event = Event::Mouse(MouseEvent { kind: MouseKind::Drag(repeat.button), x, y, mods: Modifiers::default() });
        self.dispatch(&[repeat.owner], &event, now);
        self.dirty = true;
    }

    pub(super) fn handle_mouse(&mut self, mouse: MouseEvent, now: Duration) {
        self.pointer = Some((mouse.x, mouse.y));
        self.interaction.pointer = self.pointer;
        let hit = self.frame.hit(mouse.x, mouse.y);
        self.interaction.hovered_chain = hit.map(|id| self.frame.ancestry(id)).unwrap_or_default();
        if hit != self.interaction.hovered || hit.is_some() {
            self.interaction.hovered = hit;
            self.dirty = true;
        }
        let event = Event::Mouse(mouse);
        match mouse.kind {
            // Only widgets that asked hear plain moves: most widgets read any mouse event under
            // them as theirs, and a move is no reason to act.
            MouseKind::Moved => {
                if let Some(target) = hit {
                    let wanting: Vec<WidgetId> = self
                        .frame
                        .routed_ancestry(target)
                        .into_iter()
                        .filter(|id| self.frame.pointer_moves.contains(id))
                        .collect();
                    self.dispatch(&wanting, &event, now);
                }
            }
            MouseKind::Down(_) if let Some(press) = self.toasts.press(mouse.x, mouse.y) => self.press_toast(press, now),
            MouseKind::Down(button) => self.press(mouse, button, hit, now),
            MouseKind::Drag(MouseButton::Left) | MouseKind::Up(MouseButton::Left)
                if self.interaction.pointer_capture.is_none()
                    && self.selection.as_ref().is_some_and(|selection| selection.dragging) =>
            {
                if let Some(selection) = &mut self.selection {
                    if mouse.kind == MouseKind::Up(MouseButton::Left) {
                        if !selection.release() {
                            self.selection = None;
                        }
                    } else {
                        selection.drag_to(mouse.x, mouse.y);
                    }
                }
                self.dirty = true;
            }
            MouseKind::Drag(_) | MouseKind::Up(_) if self.swallow_release => {
                if matches!(mouse.kind, MouseKind::Up(_)) {
                    self.swallow_release = false;
                }
            }
            MouseKind::Drag(_) | MouseKind::Up(_) => {
                if let Some(owner) = self.interaction.pointer_capture {
                    self.dispatch(&[owner], &event, now);
                } else if let Some(target) = hit {
                    let ancestry = self.frame.routed_ancestry(target);
                    self.dispatch(&ancestry, &event, now);
                }
                if matches!(mouse.kind, MouseKind::Up(_)) {
                    self.interaction.pointer_capture = None;
                    self.pointer_repeat = None;
                }
                self.dirty = true;
            }
            MouseKind::ScrollUp | MouseKind::ScrollDown => {
                if let Some(target) = hit {
                    let ancestry = self.frame.routed_ancestry(target);
                    self.dispatch(&ancestry, &event, now);
                }
                self.dirty = true;
            }
        }
    }

    /// A press on a toast, which is above everything else.
    fn press_toast(&mut self, press: ToastPress<A::Msg>, now: Duration) {
        self.swallow_release = true;
        self.dirty = true;
        // A toast floats above everything, so a press on it is outside any open dropdown or
        // key-capturing widget: those close, and the press still acts on the toast.
        if let Some(owner) = self.interaction.key_capture {
            self.dispatch(&[owner], &Event::PointerOutside, now);
        }
        self.dismiss_outside(None, now);
        if let ToastPress::Action(message) | ToastPress::Pressed(message) = press {
            self.update(message);
        }
    }

    /// A button press that did not land on a toast: the selection, open layers, the widget
    /// pressed and, when nothing used the press, a new text selection.
    fn press(&mut self, mouse: MouseEvent, button: MouseButton, hit: Option<WidgetId>, now: Duration) {
        let event = Event::Mouse(mouse);
        self.pressed_button = Some(button);
        // A press on the selection's menu acts on the selection and a right press on the
        // selection opens that menu; any other press clears the selection.
        let menu = self.selection_menu;
        let on_menu = hit.zip(menu).is_some_and(|(target, menu)| self.frame.is_within(target, menu));
        let on_selection = button == MouseButton::Right
            && self.selection.as_ref().is_some_and(|selection| selection.contains(mouse.x, mouse.y));
        if !on_menu && !on_selection && self.selection.take().is_some() {
            self.dirty = true;
        }
        // A press outside an open dropdown or other non-modal layer closes it and still
        // acts on what it landed on; only a press on the widget that opened the layer stops.
        self.press_target = None;
        if let Some(owner) = self.interaction.key_capture
            && hit != Some(owner)
        {
            self.dispatch(&[owner], &Event::PointerOutside, now);
            self.dirty = true;
        }
        if self.dismiss_outside(hit, now) {
            self.swallow_release = true;
            return;
        }
        if on_selection && let Some(menu) = menu {
            self.dispatch(&[menu], &event, now);
            self.dirty = true;
            return;
        }
        let mut used = false;
        if let Some(target) = hit {
            let ancestry = self.frame.routed_ancestry(target);
            if let Some(focusable) = ancestry.iter().find(|id| self.frame.focusable.contains(id)) {
                self.interaction.focused = Some(*focusable);
            }
            // Widgets that asked see the press first, outermost first; one that uses it keeps it
            // from the widgets inside.
            let previews: Vec<WidgetId> =
                ancestry.iter().rev().filter(|id| self.frame.press_previews.contains(id)).copied().collect();
            for id in previews {
                self.press_target = self.dispatch_as(&[id], &event, now, true);
                if self.press_target.is_some() {
                    break;
                }
            }
            if self.press_target.is_none() {
                self.press_target = self.dispatch(&ancestry, &event, now);
            }
            used = self.press_target.is_some();
        }
        if !used && button == MouseButton::Left {
            let press = Press::next(self.last_press, (mouse.x, mouse.y), now);
            self.last_press = Some(press);
            self.selection = Selection::begin(&self.frame, press, hit);
            self.dirty = true;
        }
    }

    /// Sends [`Event::PointerOutside`] to the dismissable layers above the layer a press at `hit`
    /// landed in. A modal layer stops the walk: it covers the screen, so nothing beneath it is
    /// outside. Returns whether the press goes no further, which is only when it landed on the
    /// widget that opened one of the closed layers; otherwise the press still reaches its target.
    pub(super) fn dismiss_outside(&mut self, hit: Option<WidgetId>, now: Duration) -> bool {
        let outside: Vec<WidgetId> = self
            .frame
            .layers
            .iter()
            .rev()
            .take_while(|layer| !layer.modal && hit.is_none_or(|target| !self.frame.is_within(target, layer.id)))
            .map(|layer| layer.id)
            .collect();
        for layer in &outside {
            self.dispatch(&[*layer], &Event::PointerOutside, now);
        }
        if !outside.is_empty() {
            self.dirty = true;
        }
        let on_opener = |layer: &WidgetId| {
            self.openers
                .iter()
                .find(|(id, _)| id == layer)
                .and_then(|(_, opener)| *opener)
                .is_some_and(|opener| hit.is_some_and(|target| self.frame.is_within(target, opener)))
        };
        outside.iter().any(on_opener)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::event::{Event, MouseButton, MouseKind};
    use crate::geometry::{Rect, Size};
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{EventCx, MeasureCx, PaintCx, View, Widget};

    /// Repeats the held pointer every 10 ms and stops the repeat on its third drag.
    struct Holder;

    impl Widget<u32> for Holder {
        fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
            Size::new(4, 1).min(available)
        }
        fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
            cx.register_hit(area);
        }
        fn event(&self, cx: &mut EventCx<'_, u32>, event: &Event) -> bool {
            let Event::Mouse(mouse) = event else {
                return false;
            };
            match mouse.kind {
                MouseKind::Down(MouseButton::Left) => {
                    cx.capture_pointer();
                    cx.repeat_pointer(Duration::from_millis(10));
                }
                MouseKind::Drag(MouseButton::Left) => {
                    let drags = cx.memory::<u32>();
                    *drags += 1;
                    let drags = *drags;
                    if drags == 3 {
                        cx.stop_pointer_repeat();
                    }
                    cx.emit(drags);
                }
                _ => {}
            }
            true
        }
    }

    /// The number of drags the holder has seen.
    struct Drags(u32);

    impl App for Drags {
        type Msg = u32;
        fn update(&mut self, drags: u32) -> Command<u32> {
            self.0 = drags;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, u32>) {
            ui.add(Holder);
        }
    }

    #[test]
    fn a_widget_stops_its_pointer_repeat_before_the_release() {
        let mut h = Harness::new(Drags(0), 10, 1);
        h.mouse(MouseKind::Down(MouseButton::Left), 1, 0);
        h.advance(Duration::from_millis(9));
        assert_eq!(h.app().0, 0, "nothing before the interval");
        for expected in 1..=3 {
            h.advance(Duration::from_millis(if expected == 1 { 1 } else { 10 }));
            assert_eq!(h.app().0, expected, "one drag per interval while the repeat runs");
        }
        for _ in 0..5 {
            h.advance(Duration::from_millis(10));
        }
        assert_eq!(h.app().0, 3, "the stopped repeat delivers nothing more");
        h.mouse(MouseKind::Drag(MouseButton::Left), 2, 0);
        assert_eq!(h.app().0, 4, "real drags still arrive");
    }

    /// Counts every mouse event it hears, as `(its index, kind)`; asks for plain moves when `track`.
    struct Counter {
        index: usize,
        track: bool,
    }

    impl Widget<(usize, MouseKind)> for Counter {
        fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
            Size::new(4, 1).min(available)
        }
        fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
            cx.register_hit(area);
            if self.track {
                cx.track_pointer_moves();
            }
        }
        fn event(&self, cx: &mut EventCx<'_, (usize, MouseKind)>, event: &Event) -> bool {
            let Event::Mouse(mouse) = event else {
                return false;
            };
            cx.emit((self.index, mouse.kind));
            true
        }
    }

    /// What the counters heard.
    struct Heard(Vec<(usize, MouseKind)>);

    impl App for Heard {
        type Msg = (usize, MouseKind);
        fn update(&mut self, heard: (usize, MouseKind)) -> Command<(usize, MouseKind)> {
            self.0.push(heard);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, (usize, MouseKind)>) {
            ui.row(|ui| {
                ui.add(Counter { index: 0, track: false });
                ui.add(Counter { index: 1, track: true });
            });
        }
    }

    #[test]
    fn plain_moves_reach_only_widgets_that_track_them() {
        let mut h = Harness::new(Heard(Vec::new()), 8, 1);
        h.hover(1, 0).hover(2, 0);
        assert!(h.app().0.is_empty(), "a widget that did not ask hears no moves");
        h.hover(5, 0);
        assert_eq!(h.app().0, [(1, MouseKind::Moved)]);
        h.click(1, 0);
        assert_eq!(h.app().0.len(), 3, "presses still reach every widget");
    }
}
