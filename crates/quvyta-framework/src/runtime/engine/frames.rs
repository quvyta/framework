//! How often frames are drawn: the frame limit, and the input that is never held back.
//!
//! The engine owns no terminal, so it only answers the loop's question, [`Engine::frame_due`]:
//! is a frame wanted now, and does the limit allow it. A frame is wanted when the view changed
//! ([`Engine::dirty`]) or a moment an animation or an idle watch asked for has come. A frame
//! answering input is drawn whatever the limit says; the rest are merged, which is what keeps a
//! program pouring out lines from spending a slow connection on frames nobody can read apart.

use std::time::Duration;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;

use super::Engine;
use crate::runtime::App;

/// What the engine keeps about the pace of frames.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Pacing {
    /// When the latest frame was drawn, or `None` before the first one.
    last: Option<Duration>,
    /// Input arrived since that frame: the next one is drawn without waiting.
    urgent: bool,
}

impl<A: App> Engine<A> {
    /// Notes that the next frame answers input, so the limit does not hold it back.
    pub(super) fn frame_is_urgent(&mut self) {
        self.pacing.urgent = true;
    }

    /// Notes the frame drawn at `now`; the limit counts from it.
    pub(super) fn frame_drawn(&mut self, now: Duration) {
        self.pacing.last = Some(now);
        self.pacing.urgent = false;
    }

    /// Builds and paints the view off screen when an event changed it and another event follows
    /// before the next frame, so every event of a burst meets what the one before it did.
    ///
    /// Events arrive in bursts: a fast typist, a terminal multiplexer or a slow connection sends
    /// several keys in one read, and a terminal without bracketed paste delivers a paste as keys.
    /// A controlled text input computes its new value from the value its node was built with, so
    /// without this every key of a burst starts from the same old value and only the last one
    /// survives: `demo` typed at once becomes `do`, then `o`. The frame is not drawn and does not
    /// count against the frame limit, so the burst's answer still reaches the screen at once.
    pub(super) fn catch_up(&mut self, now: Duration) {
        let Some(size) = self.screen.filter(|_| self.dirty && self.tree.is_some()) else {
            return;
        };
        let pacing = self.pacing;
        let mut unseen = Buffer::empty(Rect::new(0, 0, size.width, size.height));
        self.render(&mut unseen, now);
        self.pacing = pacing;
        // The screen still shows the frame before the burst.
        self.dirty = true;
    }

    /// Whether a frame is wanted at `now`: the view changed, or a moment an animation or an idle
    /// watch asked for has come.
    fn frame_wanted(&self, now: Duration) -> bool {
        self.dirty || self.deadline().is_some_and(|deadline| deadline <= now)
    }

    /// Whether the loop should draw a frame at `now`: one is wanted and the
    /// [`FrameLimit`](crate::runtime::FrameLimit) allows it. A frame that answers input, and the
    /// first frame of a run, are always allowed.
    pub(crate) fn frame_due(&self, now: Duration) -> bool {
        if !self.frame_wanted(now) {
            return false;
        }
        if self.pacing.urgent {
            return true;
        }
        match (self.frame_gap(), self.pacing.last) {
            (Some(gap), Some(last)) => now.saturating_sub(last) >= gap,
            _ => true,
        }
    }

    /// When a frame the limit is holding back may be drawn, if one is waiting. The loop waits no
    /// longer than this, so a held frame costs the rest of the gap and never a whole idle wait.
    /// `None` while no frame waits, so a quiet application still wakes only for its own work.
    pub(crate) fn frame_deadline(&self, now: Duration) -> Option<Duration> {
        if self.pacing.urgent || !self.frame_wanted(now) {
            return None;
        }
        let at = self.pacing.last?.checked_add(self.frame_gap()?)?;
        (at > now).then_some(at)
    }

    /// The shortest time between two frames the application's limit allows on this connection.
    fn frame_gap(&self) -> Option<Duration> {
        self.app.frame_limit().gap(self.env.remote())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui_core::buffer::Buffer;
    use ratatui_core::layout::Rect;

    use super::super::{Engine, TaskMode};
    use crate::env::Env;
    use crate::event::{Event, KeyEvent};
    use crate::runtime::{App, Command, FrameLimit};
    use crate::widget::View;
    use crate::widgets::Text;

    const MILLISECOND: Duration = Duration::from_millis(1);

    /// An application whose embedded program keeps writing: every message changes the screen.
    struct Program {
        limit: FrameLimit,
        lines: usize,
    }

    impl App for Program {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            self.lines += 1;
            Command::none()
        }
        fn frame_limit(&self) -> FrameLimit {
            self.limit
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Text::new(format!("{} lines", self.lines)));
        }
    }

    /// An engine already past its first frame, at the clock's zero.
    fn running(limit: FrameLimit) -> (Engine<Program>, Buffer) {
        let mut engine = Engine::new(Program { limit, lines: 0 }, Env::builtin(), TaskMode::Inline);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 1));
        engine.render(&mut buffer, Duration::ZERO);
        (engine, buffer)
    }

    /// One simulated second of a program writing a line every millisecond, drawn the way the
    /// terminal loop draws: a frame whenever the engine says one is due. Returns how many frames
    /// were drawn. The clock is simulated, so nothing sleeps.
    fn frames_in_a_second(limit: FrameLimit) -> usize {
        let (mut engine, mut buffer) = running(limit);
        let mut frames = 0;
        for step in 1..=1000 {
            let now = step * MILLISECOND;
            engine.update(());
            if engine.frame_due(now) {
                engine.render(&mut buffer, now);
                frames += 1;
            }
        }
        frames
    }

    #[test]
    fn a_program_writing_without_pause_is_drawn_at_the_limit_and_no_more() {
        assert_eq!(frames_in_a_second(FrameLimit::per_second(20)), 20, "20 frames a second");
        assert_eq!(frames_in_a_second(FrameLimit::per_second(10)), 10);
        // The limit is a ceiling: 60 frames a second leave 16⅔ ms between frames, and a loop that
        // wakes on whole milliseconds draws just after each gap, so a second holds 58 of them.
        assert!(frames_in_a_second(FrameLimit::default()) <= 60, "the default, on a local connection");
        assert_eq!(frames_in_a_second(FrameLimit::default()), 58);
    }

    #[test]
    fn without_a_limit_every_wanted_frame_is_drawn() {
        assert_eq!(frames_in_a_second(FrameLimit::none()), 1000);
    }

    #[test]
    fn the_frame_after_a_key_is_drawn_at_once_while_the_limit_holds_the_others_back() {
        let (mut engine, mut buffer) = running(FrameLimit::per_second(20));
        engine.update(());
        assert!(!engine.frame_due(MILLISECOND), "a line the program wrote waits for the gap");
        engine.handle(Event::Key(KeyEvent::press("x")), MILLISECOND);
        assert!(engine.frame_due(MILLISECOND), "the character the user typed is echoed at once");
        engine.render(&mut buffer, MILLISECOND);
        engine.update(());
        assert!(!engine.frame_due(2 * MILLISECOND), "the limit counts from the frame just drawn");
        assert!(engine.frame_due(Duration::from_millis(51)), "a gap after it");
    }

    #[test]
    fn a_frame_built_between_two_events_is_not_a_drawn_frame() {
        // The view is rebuilt between the events of a burst so each meets what the one before it
        // did. That frame never reaches the screen, so it neither takes the place of the frame
        // the screen still waits for nor moves the moment the limit counts from.
        let (mut engine, _) = running(FrameLimit::per_second(20));
        engine.update(());
        engine.handle(Event::PointerOutside, 10 * MILLISECOND);
        assert!(engine.frame_due(Duration::from_millis(50)), "the gap still counts from the frame drawn at zero");
        assert!(!engine.frame_due(Duration::from_millis(49)), "and the limit still holds");
    }

    #[test]
    fn the_loop_wakes_when_a_held_frame_may_be_drawn_and_not_while_nothing_waits() {
        let (mut engine, mut buffer) = running(FrameLimit::per_second(20));
        assert_eq!(engine.frame_deadline(MILLISECOND), None, "nothing is waiting to be drawn");
        engine.update(());
        assert_eq!(engine.frame_deadline(MILLISECOND), Some(Duration::from_millis(50)), "the rest of the gap");
        engine.handle(Event::Key(KeyEvent::press("x")), MILLISECOND);
        assert_eq!(engine.frame_deadline(MILLISECOND), None, "input is not waited for");
        engine.render(&mut buffer, MILLISECOND);
        assert_eq!(engine.frame_deadline(MILLISECOND), None, "the frame was drawn");
    }

    #[test]
    fn the_first_frame_of_a_run_is_never_held_back() {
        let mut engine =
            Engine::new(Program { limit: FrameLimit::per_second(1), lines: 0 }, Env::builtin(), TaskMode::Inline);
        assert!(engine.frame_due(Duration::ZERO), "a second before the first frame would be an empty screen");
        engine.render(&mut Buffer::empty(Rect::new(0, 0, 20, 1)), Duration::ZERO);
        engine.update(());
        assert!(!engine.frame_due(Duration::from_millis(999)));
        assert!(engine.frame_due(Duration::from_secs(1)));
    }
}
