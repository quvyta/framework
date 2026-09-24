//! The frame limit and the pointer: a drag, the pointer passing over and the wheel are merged
//! like the application's own work, a press, a release and a key never wait.
//!
//! Every test drives the engine the way the terminal loop does, on a simulated clock: events are
//! handed over as they arrive, and a frame is written to a terminal whenever the engine says one
//! is due. The terminal is a real [`Screen`] writing into memory, so the bytes a drag costs are
//! the bytes the runtime would send down a connection.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::Duration;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::terminal::{Terminal, TerminalOptions, Viewport};
use ratatui_crossterm::CrosstermBackend;

use super::super::{Engine, TaskMode};
use crate::env::Env;
use crate::event::{Event, KeyEvent, MouseButton, MouseEvent, MouseKind};
use crate::keymap::Modifiers;
use crate::runtime::present::Screen;
use crate::runtime::{App, Command, FrameLimit};
use crate::widget::View;
use crate::widgets::{Slider, Text};

const MILLISECOND: Duration = Duration::from_millis(1);

/// The screen the tests draw on: wide enough for a rail with a cell for every value.
const WIDTH: u16 = 80;
const HEIGHT: u16 = 4;

/// The row the slider sits on, under the line that names its value.
const SLIDER_ROW: i32 = 1;

/// Where a drag starts, on the slider's rail; the rail reaches fifty cells further and more.
const LEFT: i32 = 20;

/// A slider the user drags, and a program writing lines beside it.
struct Board {
    limit: FrameLimit,
    value: f64,
    /// How many values the slider reported: one for every motion that moved it.
    changes: usize,
    lines: usize,
}

#[derive(Debug, Clone)]
enum Msg {
    Slide(f64),
    Line,
}

impl App for Board {
    type Msg = Msg;
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Slide(value) => {
                self.value = value;
                self.changes += 1;
            }
            Msg::Line => self.lines += 1,
        }
        Command::none()
    }
    fn frame_limit(&self) -> FrameLimit {
        self.limit
    }
    fn view(&self, ui: &mut View<'_, Msg>) {
        ui.add(Text::new(format!("value {} lines {}", self.value, self.lines)));
        ui.add(Slider::new(self.value).on_change(Msg::Slide));
    }
}

/// What the screen wrote, kept outside it so a test can count it.
#[derive(Clone, Default)]
struct Wire(Rc<RefCell<Vec<u8>>>);

impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// An engine, the terminal it draws on and what the loop drew.
struct Run {
    engine: Engine<Board>,
    screen: Screen<Wire>,
    wire: Wire,
    /// When each frame after the first was drawn.
    frames: Vec<Duration>,
    /// The cells of the latest frame drawn.
    shown: Buffer,
}

impl Run {
    /// A run whose first frame was drawn at the clock's zero.
    fn new(limit: FrameLimit) -> Self {
        let wire = Wire::default();
        let options = TerminalOptions { viewport: Viewport::Fixed(Rect::new(0, 0, WIDTH, HEIGHT)) };
        let terminal = Terminal::with_options(CrosstermBackend::new(wire.clone()), options).expect("a terminal");
        let board = Board { limit, value: 0.0, changes: 0, lines: 0 };
        let mut run = Self {
            engine: Engine::new(board, Env::builtin(), TaskMode::Inline),
            screen: Screen::new(terminal),
            wire,
            frames: Vec::new(),
            shown: Buffer::empty(Rect::new(0, 0, WIDTH, HEIGHT)),
        };
        assert!(run.draw(Duration::ZERO), "the first frame");
        run.frames.clear();
        run.wire.0.borrow_mut().clear();
        run
    }

    /// Draws a frame at `now` when the engine says one is due, the way the terminal loop does.
    /// Returns whether one was drawn.
    fn draw(&mut self, now: Duration) -> bool {
        if !self.engine.frame_due(now) {
            return false;
        }
        let engine = &mut self.engine;
        let shown = &mut self.shown;
        self.screen
            .present(|buffer| {
                engine.render(buffer, now);
                shown.clone_from(buffer);
                engine.painted()
            })
            .expect("a frame");
        self.frames.push(now);
        true
    }

    /// Hands `event` over at `now` and draws if a frame is due. Returns whether one was drawn.
    fn send(&mut self, event: Event, now: Duration) -> bool {
        self.engine.handle(event, now);
        self.draw(now)
    }

    fn mouse(&mut self, kind: MouseKind, x: i32, now: Duration) -> bool {
        self.send(Event::Mouse(MouseEvent { kind, x, y: SLIDER_ROW, mods: Modifiers::default() }), now)
    }

    /// Frames drawn after `from`, up to and including `to`.
    fn frames_between(&self, from: Duration, to: Duration) -> usize {
        self.frames.iter().filter(|at| **at > from && **at <= to).count()
    }

    /// Bytes written to the terminal so far.
    fn bytes(&self) -> usize {
        self.wire.0.borrow().len()
    }

    /// The first line of the latest frame drawn: the slider's value and the program's lines.
    fn first_line(&self) -> String {
        (0..WIDTH).map(|x| self.shown[(x, 0)].symbol()).collect::<String>().trim_end().to_owned()
    }
}

const fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

const DRAG: MouseKind = MouseKind::Drag(MouseButton::Left);
const DOWN: MouseKind = MouseKind::Down(MouseButton::Left);
const UP: MouseKind = MouseKind::Up(MouseButton::Left);

/// A run at five frames a second, with the slider pressed at one second: the press is drawn.
fn pressed_at_one_second() -> Run {
    let mut run = Run::new(FrameLimit::per_second(5));
    assert!(run.mouse(DOWN, LEFT, ms(1000)), "a press is drawn at once");
    run
}

/// Drags one cell to the right every two milliseconds from `from` for `steps` motions, from
/// `LEFT + 1` on. Returns the moment of the last motion.
fn drag_right(run: &mut Run, from: Duration, steps: i32) -> Duration {
    let mut now = from;
    for step in 1..=steps {
        now = from + 2 * MILLISECOND * u32::try_from(step - 1).expect("a small step");
        run.mouse(DRAG, LEFT + step, now);
    }
    now
}

#[test]
fn a_drag_is_drawn_at_most_once_a_gap_and_every_motion_reaches_the_application() {
    let mut run = pressed_at_one_second();
    let changes = run.engine.app.changes;
    let pressed = run.first_line();
    // Forty motions inside one gap of 200 ms, the first right after the press was drawn.
    let last = drag_right(&mut run, ms(1002), 40);
    assert_eq!(last, ms(1080));
    assert_eq!(run.frames_between(ms(1000), last), 0, "the motions wait for the gap the press started");
    assert_eq!(run.engine.app.changes - changes, 40, "the application heard every motion");
    assert_eq!(run.first_line(), pressed, "the screen still shows the press");
    assert!(!run.draw(ms(1199)), "not before the gap is over");
    assert!(run.draw(ms(1200)), "the latest state when it is");
    let value = run.engine.app.value;
    assert!(run.first_line().starts_with(&format!("value {value} ")), "{}", run.first_line());

    // A long drag: a motion every two milliseconds for two seconds, back and forth over the rail.
    let started = ms(1200);
    for step in 1..=1000u32 {
        let now = started + 2 * MILLISECOND * step;
        let x = LEFT + i32::try_from(step % 50).expect("a cell");
        run.mouse(DRAG, x, now);
        run.draw(now);
    }
    assert_eq!(run.frames_between(started, ms(3200)), 10, "five frames a second for two seconds");
}

#[test]
fn the_release_is_drawn_at_once_with_the_final_position() {
    let mut run = pressed_at_one_second();
    let last = drag_right(&mut run, ms(1002), 30);
    assert_eq!(run.frames_between(ms(1000), last), 0);
    assert!(run.mouse(UP, LEFT + 30, last + MILLISECOND), "the release does not wait for the gap");
    let value = run.engine.app.value;
    assert!(value > 0.0, "the drag moved the slider");
    assert!(run.first_line().starts_with(&format!("value {value} ")), "the final position: {}", run.first_line());
    assert_eq!(run.engine.frame_deadline(last + 2 * MILLISECOND), None, "nothing is left waiting");
}

#[test]
fn a_key_during_a_drag_is_drawn_at_once_with_the_latest_drag() {
    let mut run = pressed_at_one_second();
    let last = drag_right(&mut run, ms(1002), 25);
    assert_eq!(run.frames_between(ms(1000), last), 0);
    let value = run.engine.app.value;
    assert!(run.send(Event::Key(KeyEvent::press("x")), last + MILLISECOND), "a key never waits");
    assert!(run.first_line().starts_with(&format!("value {value} ")), "with the drag in it: {}", run.first_line());
}

#[test]
fn motion_that_stops_without_a_release_is_drawn_by_the_gap() {
    let mut run = pressed_at_one_second();
    let last = drag_right(&mut run, ms(1002), 20);
    // The loop waits no longer than this, so the held frame cannot be forgotten.
    assert_eq!(run.engine.frame_deadline(last), Some(ms(1200)), "the loop wakes when the gap is over");
    assert_eq!(run.engine.frame_deadline(ms(1150)), Some(ms(1200)));
    assert!(!run.draw(ms(1199)));
    assert!(run.draw(ms(1200)), "the last position, still held down, reaches the screen");
    let value = run.engine.app.value;
    assert!(run.first_line().starts_with(&format!("value {value} ")), "{}", run.first_line());
    assert_eq!(run.engine.frame_deadline(ms(1201)), None, "and then nothing waits");
}

#[test]
fn the_first_motion_after_a_rest_is_drawn_at_once() {
    // The limit spaces frames; it does not delay one when the last frame is a gap behind.
    let mut run = pressed_at_one_second();
    assert!(run.mouse(DRAG, LEFT + 5, ms(1300)), "a gap after the press, a motion is drawn at once");
    assert!(!run.mouse(DRAG, LEFT + 6, ms(1301)), "the next one waits");
}

#[test]
fn the_pointer_passing_over_is_merged_like_a_drag() {
    let mut run = Run::new(FrameLimit::per_second(5));
    assert!(run.mouse(MouseKind::Moved, LEFT, ms(1000)), "the first move after a rest is drawn at once");
    for step in 1..=50u32 {
        let x = LEFT + i32::try_from(step).expect("a cell");
        run.mouse(MouseKind::Moved, x, ms(1000) + step * MILLISECOND);
    }
    assert_eq!(run.frames_between(ms(1000), ms(1050)), 0, "the tone under the pointer waits for the gap");
    assert_eq!(run.engine.frame_deadline(ms(1050)), Some(ms(1200)));
    assert!(run.draw(ms(1200)));
}

#[test]
fn a_wheel_notch_after_a_rest_is_drawn_at_once_and_a_spin_at_the_limit() {
    let mut run = Run::new(FrameLimit::per_second(5));
    assert!(run.mouse(MouseKind::ScrollUp, LEFT, ms(1000)), "one notch answers at once");
    assert!(run.engine.app.value > 0.0, "the wheel moved the slider");
    // Ninety-nine notches two milliseconds apart, the last at 1198 ms.
    for step in 1..=99u32 {
        run.mouse(MouseKind::ScrollUp, LEFT, ms(1000) + 2 * step * MILLISECOND);
    }
    assert_eq!(run.frames_between(ms(1000), ms(1199)), 0, "a spin is drawn at the limit's pace");
    assert!(run.draw(ms(1200)));
    let value = run.engine.app.value;
    assert!(run.first_line().starts_with(&format!("value {value} ")), "where the spin ended: {}", run.first_line());
}

#[test]
fn every_key_is_drawn_at_once_at_the_local_default() {
    // A program writing a line every millisecond, and a key every millisecond: each key's frame
    // is drawn the moment it arrives, as before the pointer was merged.
    let mut run = Run::new(FrameLimit::default());
    for step in 1..=100u32 {
        let now = ms(1000) + step * MILLISECOND;
        run.engine.update(Msg::Line);
        assert!(run.send(Event::Key(KeyEvent::press("x")), now), "the key at {now:?} waits for nothing");
    }
    assert_eq!(run.frames_between(ms(1000), ms(1100)), 100);
}

/// What a two-second drag writes to the terminal at `frames` a second: a press, a motion every
/// ten milliseconds back and forth over the rail, a release. The loop is simulated a millisecond
/// at a time, so a held frame is drawn when its gap is over, as the terminal loop wakes for it.
fn two_second_drag(frames: u32) -> (usize, usize) {
    let mut run = Run::new(FrameLimit::per_second(frames));
    run.mouse(DOWN, LEFT, ms(1000));
    for tick in 1..=2000u32 {
        let now = ms(1000) + tick * MILLISECOND;
        if tick % 10 == 0 {
            let cell = i32::try_from(tick / 10 % 100).expect("a cell");
            let x = if cell < 50 { LEFT + cell } else { LEFT + 100 - cell };
            run.mouse(DRAG, x, now);
        }
        run.draw(now);
    }
    run.mouse(UP, LEFT, ms(3001));
    (run.bytes(), run.frames.len())
}

#[test]
fn a_drag_costs_fewer_bytes_at_a_lower_limit() {
    let (slow_bytes, slow_frames) = two_second_drag(5);
    let (fast_bytes, fast_frames) = two_second_drag(60);
    println!("a two-second drag, a motion every 10 ms, {WIDTH} × {HEIGHT}:");
    println!("  5 frames a second:  {slow_frames:>4} frames {slow_bytes:>7} bytes");
    println!("  60 frames a second: {fast_frames:>4} frames {fast_bytes:>7} bytes");
    // At 5 a press, ten frames of drag and a release. At 60 the gap of 16⅔ ms is longer than the
    // ten between two motions, so the frames come at the limit's pace: about sixty a second.
    assert_eq!(slow_frames, 12, "the press, five a second for two seconds, the release");
    assert!((100..=121).contains(&fast_frames), "sixty a second for two seconds: {fast_frames}");
    assert!(slow_bytes * 5 < fast_bytes, "the limit is felt on the wire: {slow_bytes} against {fast_bytes}");
}
