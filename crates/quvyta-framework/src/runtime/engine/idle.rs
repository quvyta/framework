//! Input idleness: when the user last did something in this terminal, and waking the
//! application for the silences its view watches.

use std::time::Duration;

use super::Engine;
use crate::event::Event;
use crate::runtime::App;
use crate::widget::{IdleScope, IdleWatch};

/// How far apart the frames of a view that reads idleness are: the value it shows is never more
/// than this out of date.
const READ_STEP: Duration = Duration::from_secs(1);

/// What the engine keeps about idleness between frames.
pub(super) struct Idle<Msg> {
    /// When the last input arrived, or zero, the start of the engine's clock.
    last_input: Duration,
    /// The watches the latest frame declared.
    watches: Vec<IdleWatch<Msg>>,
    /// The `after` of every watch told that the current silence began, so it is told only once.
    told: Vec<Duration>,
    /// When the view that reads idleness needs its next frame.
    redraw_at: Option<Duration>,
    /// A handoff was taken out of the queue; its end counts as input.
    handed_off: bool,
}

impl<Msg> Default for Idle<Msg> {
    fn default() -> Self {
        Self { last_input: Duration::ZERO, watches: Vec::new(), told: Vec::new(), redraw_at: None, handed_off: false }
    }
}

impl<Msg> Idle<Msg> {
    /// The scope the view of a frame at `now` reads and declares watches in.
    pub(super) fn scope(&self, now: Duration) -> IdleScope<Msg> {
        IdleScope::new(now.saturating_sub(self.last_input))
    }

    /// Keeps what the view of the frame at `now` asked for.
    pub(super) fn settle(&mut self, scope: IdleScope<Msg>, now: Duration) {
        self.watches = scope.watches.into_inner();
        self.redraw_at = if scope.read.get() {
            // The rest of the current whole second of silence, however long the silence is, so
            // the next frame is never one already past, which would wake the loop without end.
            let silent = now.saturating_sub(self.last_input);
            let into_step = silent.as_nanos() % READ_STEP.as_nanos();
            let rest = READ_STEP.saturating_sub(Duration::from_nanos(u64::try_from(into_step).unwrap_or(0)));
            now.checked_add(rest)
        } else {
            None
        };
    }

    /// When the engine must wake next for idleness: the next watch to reach its silence, or the
    /// next frame of a view that reads it.
    pub(super) fn deadline(&self) -> Option<Duration> {
        let watch = self
            .watches
            .iter()
            .filter(|watch| !self.told.contains(&watch.after))
            // A silence that would end past the clock's last moment is never reached.
            .filter_map(|watch| self.last_input.checked_add(watch.after))
            .min();
        match (watch, self.redraw_at) {
            (Some(watch), Some(redraw)) => Some(watch.min(redraw)),
            (watch, redraw) => watch.or(redraw),
        }
    }

    /// Notes that a handoff is about to run.
    pub(super) fn handing_off(&mut self) {
        self.handed_off = true;
    }
}

/// Whether `event` is the user doing something in this terminal. Every key event, every mouse
/// event (the pointer moving over the window included) and a paste are; a terminal resize never
/// reaches the engine as an event, and [`Event::PointerOutside`] is one the runtime makes itself.
fn is_input(event: &Event) -> bool {
    match event {
        Event::Key(_) | Event::Mouse(_) | Event::Paste(_) => true,
        Event::PointerOutside => false,
    }
}

impl<A: App> Engine<A> {
    /// Starts a new silence if `event`, arriving at `now`, is input. Watches told that the last
    /// silence began are told it ended, before the event reaches a widget.
    pub(super) fn note_input(&mut self, event: &Event, now: Duration) {
        if is_input(event) {
            self.input_at(now);
        }
    }

    fn input_at(&mut self, now: Duration) {
        self.idle.last_input = now;
        if self.idle.told.is_empty() {
            return;
        }
        let told = std::mem::take(&mut self.idle.told);
        let messages: Vec<A::Msg> = self
            .idle
            .watches
            .iter()
            .filter(|watch| told.contains(&watch.after))
            .map(|watch| (watch.message)(false))
            .collect();
        for message in messages {
            self.update(message);
        }
    }

    /// Delivers what idleness has due at `now`: the end of a handoff counts as input, and every
    /// watch whose silence has been reached and was not told yet is told it began.
    pub(super) fn wake_idle(&mut self, now: Duration) {
        if std::mem::take(&mut self.idle.handed_off) {
            self.input_at(now);
        }
        let silent = now.saturating_sub(self.idle.last_input);
        let due: Vec<Duration> = self
            .idle
            .watches
            .iter()
            .map(|watch| watch.after)
            .filter(|after| *after <= silent && !self.idle.told.contains(after))
            .collect();
        if due.is_empty() {
            return;
        }
        let messages: Vec<A::Msg> = self
            .idle
            .watches
            .iter()
            .filter(|watch| due.contains(&watch.after))
            .map(|watch| (watch.message)(true))
            .collect();
        self.idle.told.extend(due);
        for message in messages {
            self.update(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;

    use super::super::{Engine, TaskMode};
    use crate::env::Env;
    use crate::event::{Event, KeyEvent, KeyKind, MouseButton, MouseEvent, MouseKind};
    use crate::keymap::Modifiers;
    use crate::runtime::{App, Command, Handoff, HandoffOutcome, Harness};
    use crate::widget::View;
    use crate::widgets::Text;

    const SECOND: Duration = Duration::from_secs(1);

    /// Shows how long the terminal has been idle, in milliseconds, and records what its watches
    /// were told.
    #[derive(Default)]
    struct Watcher {
        /// The `after` of each watch the view declares.
        watches: Vec<Duration>,
        /// Whether the view reads `idle_for`.
        reads: bool,
        told: Vec<(Duration, bool)>,
    }

    #[derive(Clone)]
    enum Msg {
        Told(Duration, bool),
        Watch(Duration),
        Handoff,
        Back,
    }

    impl App for Watcher {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Told(after, idle) => self.told.push((after, idle)),
                Msg::Watch(after) => self.watches.push(after),
                Msg::Handoff => return Command::handoff(Handoff::new("true", |_: HandoffOutcome| Msg::Back)),
                Msg::Back => {}
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            for after in &self.watches {
                let after = *after;
                ui.on_idle(after, move |idle| Msg::Told(after, idle));
            }
            if self.reads {
                let idle = ui.idle_for().as_millis();
                ui.add(Text::new(format!("idle {idle}ms")));
            }
        }
    }

    fn reading() -> Harness<Watcher> {
        Harness::new(Watcher { reads: true, ..Watcher::default() }, 30, 1)
    }

    fn watching(watches: &[u64]) -> Harness<Watcher> {
        let watches = watches.iter().map(|secs| Duration::from_secs(*secs)).collect();
        Harness::new(Watcher { watches, ..Watcher::default() }, 30, 1)
    }

    fn shown(harness: &Harness<Watcher>) -> String {
        harness.screen().trim().to_owned()
    }

    #[test]
    fn idleness_starts_at_zero_and_grows_with_the_clock() {
        let mut h = reading();
        assert_eq!(shown(&h), "idle 0ms", "before any input the silence counts from the start");
        h.advance(Duration::from_millis(2500));
        assert_eq!(shown(&h), "idle 2500ms");
        h.advance(Duration::from_secs(60));
        assert_eq!(shown(&h), "idle 62500ms");
    }

    #[test]
    fn every_kind_of_input_starts_the_silence_again() {
        let key = |kind| Event::Key(KeyEvent { kind, ..KeyEvent::press("x") });
        let mouse = |kind| Event::Mouse(MouseEvent { kind, x: 3, y: 0, mods: Modifiers::default() });
        let inputs = [
            key(KeyKind::Press),
            key(KeyKind::Repeat),
            key(KeyKind::Release),
            mouse(MouseKind::Down(MouseButton::Left)),
            mouse(MouseKind::Up(MouseButton::Right)),
            mouse(MouseKind::Drag(MouseButton::Left)),
            mouse(MouseKind::ScrollUp),
            mouse(MouseKind::ScrollDown),
            mouse(MouseKind::Moved),
            Event::Paste("text".to_owned()),
        ];
        for input in inputs {
            let mut h = reading();
            h.advance(Duration::from_secs(90));
            assert_eq!(shown(&h), "idle 90000ms");
            h.events(std::slice::from_ref(&input));
            assert_eq!(shown(&h), "idle 0ms", "{input:?} is input");
            h.advance(Duration::from_millis(300));
            assert_eq!(shown(&h), "idle 300ms", "the next silence counts from {input:?}");
        }
    }

    #[test]
    fn a_resize_a_message_and_a_runtime_event_are_not_input() {
        let mut h = reading();
        h.advance(Duration::from_secs(90));
        h.resize(40, 2);
        assert_eq!(shown(&h), "idle 90000ms", "a window manager resizes windows nobody sits at");
        h.send(Msg::Back);
        assert_eq!(shown(&h), "idle 90000ms", "a message is the application, not the user");
        h.events(&[Event::PointerOutside]);
        assert_eq!(shown(&h), "idle 90000ms", "the runtime makes this event itself");
    }

    #[test]
    fn a_watch_is_told_once_when_the_silence_begins_and_once_when_it_ends() {
        let mut h = watching(&[300]);
        h.advance(Duration::from_secs(299));
        assert_eq!(h.app().told, [], "one second short");
        h.advance(SECOND);
        assert_eq!(h.app().told, [(300 * SECOND, true)], "told at the very moment");
        h.advance(Duration::from_secs(600));
        assert_eq!(h.app().told.len(), 1, "a silence is announced once");
        h.hover(4, 0);
        assert_eq!(h.app().told, [(300 * SECOND, true), (300 * SECOND, false)], "the pointer came back");
        h.press("x").advance(SECOND);
        assert_eq!(h.app().told.len(), 2, "input during activity tells nothing");
        h.advance(Duration::from_secs(299));
        assert_eq!(h.app().told.last(), Some(&(300 * SECOND, true)), "the next silence counts from the last input");
    }

    #[test]
    fn events_that_are_not_input_do_not_end_the_silence() {
        let mut h = watching(&[60]);
        h.advance(Duration::from_secs(60));
        h.resize(50, 3).send(Msg::Back).events(&[Event::PointerOutside]);
        assert_eq!(h.app().told, [(60 * SECOND, true)]);
    }

    #[test]
    fn watches_of_different_lengths_are_independent() {
        let mut h = watching(&[60, 300]);
        h.advance(Duration::from_secs(60));
        assert_eq!(h.app().told, [(60 * SECOND, true)]);
        h.advance(Duration::from_secs(240));
        assert_eq!(h.app().told, [(60 * SECOND, true), (300 * SECOND, true)]);
        h.press("x");
        assert_eq!(h.app().told[2..], [(60 * SECOND, false), (300 * SECOND, false)]);
        h.advance(Duration::from_secs(61)).press("x");
        assert_eq!(h.app().told[4..], [(60 * SECOND, true), (60 * SECOND, false)], "only the one reached ends");
    }

    #[test]
    fn a_watch_declared_after_its_silence_was_reached_is_told_at_once() {
        let mut h = watching(&[]);
        h.advance(Duration::from_secs(120));
        h.send(Msg::Watch(60 * SECOND));
        assert_eq!(h.app().told, [(60 * SECOND, true)]);
    }

    fn engine(app: Watcher) -> (Engine<Watcher>, Buffer) {
        (Engine::new(app, Env::builtin(), TaskMode::Threads), Buffer::empty(Rect::new(0, 0, 30, 1)))
    }

    /// What the terminal loop waits for between frames.
    #[test]
    fn the_runtime_wakes_at_the_moment_a_watch_needs_and_not_before() {
        let (mut engine, mut buffer) = engine(Watcher { watches: vec![300 * SECOND], ..Watcher::default() });
        engine.render(&mut buffer, Duration::ZERO);
        assert_eq!(engine.deadline(), Some(300 * SECOND), "one wake-up, at the threshold");
        engine.handle(Event::Key(KeyEvent::press("x")), 7 * SECOND);
        engine.render(&mut buffer, 7 * SECOND);
        assert_eq!(engine.deadline(), Some(307 * SECOND), "input moves it");
        engine.render(&mut buffer, 307 * SECOND);
        assert_eq!(engine.app.told, [(300 * SECOND, true)]);
        assert_eq!(engine.deadline(), None, "nothing is left to wake for while the silence goes on");
    }

    #[test]
    fn a_watch_too_long_to_be_reached_never_wakes_the_loop() {
        let (mut engine, mut buffer) = engine(Watcher { watches: vec![Duration::MAX], ..Watcher::default() });
        engine.render(&mut buffer, Duration::ZERO);
        engine.handle(Event::Key(KeyEvent::press("x")), 7 * SECOND);
        engine.render(&mut buffer, 7 * SECOND);
        assert_eq!(engine.deadline(), None, "a silence that ends after the clock's last moment is never due");
        engine.render(&mut buffer, 3600 * SECOND);
        assert_eq!(engine.app.told, [], "and never told");
    }

    #[test]
    fn a_view_that_reads_idleness_is_drawn_again_each_second_and_only_then() {
        let (mut engine, mut buffer) = engine(Watcher::default());
        engine.render(&mut buffer, Duration::ZERO);
        assert_eq!(engine.deadline(), None, "a view that does not read it never wakes the loop");
        let (mut engine, mut buffer) = self::engine(Watcher { reads: true, ..Watcher::default() });
        engine.render(&mut buffer, Duration::ZERO);
        assert_eq!(engine.deadline(), Some(SECOND));
        engine.render(&mut buffer, Duration::from_millis(1400));
        assert_eq!(engine.deadline(), Some(2 * SECOND), "the next whole second of silence");
        engine.handle(Event::Key(KeyEvent::press("x")), Duration::from_millis(1500));
        engine.render(&mut buffer, Duration::from_millis(1500));
        assert_eq!(engine.deadline(), Some(Duration::from_millis(2500)), "seconds count from the input");
    }

    #[test]
    fn the_next_frame_of_a_view_that_reads_idleness_is_never_in_the_past() {
        let (mut engine, mut buffer) = engine(Watcher { reads: true, ..Watcher::default() });
        // Past the number of whole seconds a `u32` counts.
        let now = Duration::from_secs(u64::from(u32::MAX) + 10) + Duration::from_millis(300);
        engine.render(&mut buffer, now);
        assert_eq!(engine.deadline(), Some(now + Duration::from_millis(700)), "the next whole second of silence");
    }

    #[test]
    fn the_end_of_a_handoff_counts_as_input() {
        let (mut engine, mut buffer) = engine(Watcher { watches: vec![60 * SECOND], ..Watcher::default() });
        engine.render(&mut buffer, Duration::ZERO);
        engine.update(Msg::Handoff);
        assert!(engine.take_handoff().is_some(), "the handoff waits for the loop");
        // The program had the terminal for ten minutes; the user was working in it.
        engine.render(&mut buffer, 600 * SECOND);
        assert_eq!(engine.app.told, [], "no silence was announced");
        assert_eq!(engine.deadline(), Some(660 * SECOND));
    }
}
