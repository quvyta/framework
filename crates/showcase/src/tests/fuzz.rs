//! Event fuzz sweep: every showcase page takes a few hundred random key presses, clicks, drags,
//! wheels, pastes, clock jumps and resizes (down to 0 × 0 and up to very large screens) in every
//! theme and glyph mode, and must neither panic nor draw outside the screen.
//!
//! The sweep is deterministic. A small seeded generator picks every action, so a failure names a
//! seed, a page, a theme and a glyph mode, and running the same four again replays it exactly.
//!
//! - `cargo test -p quvyta-framework-showcase fuzz` runs the quick sweep of the gate: each page once, the themes
//!   and glyph modes rotating over the pages.
//! - `cargo test --release -p quvyta-framework-showcase fuzz_long -- --ignored --nocapture` runs every page in
//!   every theme and glyph mode for several seeds (4 seeds × 600 actions by default). Tune it with
//!   `QUVYTA_FUZZ_SEEDS` (seed count), `QUVYTA_FUZZ_STEPS` (actions per run),
//!   `QUVYTA_FUZZ_SEED` (first seed), and `QUVYTA_FUZZ_PAGE`, `QUVYTA_FUZZ_THEME` and
//!   `QUVYTA_FUZZ_MODE` (one page id, theme id or `Unicode`/`Ascii`/`Nerd`); a failure prints
//!   the exact line that replays it. Arithmetic overflow only panics with overflow checks, so add
//!   `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true`.
//! - `QUVYTA_FUZZ_TRACE=1` prints every run as it starts and ends, `2` every action too, to find
//!   a run that aborts the process (a stack overflow cannot be caught).

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use qframe::event::{MouseButton, MouseKind};
use qframe::icons::GlyphMode;
use qframe::runtime::Harness;

use super::env;
use crate::app::{Msg, Showcase};
use crate::pages::{PAGES, PageMsg, log_view};

const THEMES: [&str; 4] = ["monochrome", "nordic", "amber", "iris"];
const MODES: [GlyphMode; 3] = [GlyphMode::Unicode, GlyphMode::Ascii, GlyphMode::Nerd];

/// Seed of the gate's sweep; any fixed number works, this one keeps it reproducible.
const GATE_SEED: u64 = 0x5155_5659_5441;

/// Actions per page in the gate's sweep. Pages run in parallel; on twelve cores the debug sweep
/// takes about twenty seconds.
const GATE_STEPS: usize = 200;

/// Starts a real shell; random typing must never reach it, so on this page the sweep only moves,
/// scrolls, waits and resizes.
const SHELL_PAGE: &str = "terminal";

/// Streams with a `Command::perform` that schedules the next one, each waiting a real interval.
/// Random presses seldom land on its switch, so the sweep also turns streaming on and off itself,
/// on for about a third of the time.
const STREAM_PAGE: &str = "log-view";

/// SplitMix64: a tiny, well-mixed generator, enough to pick actions reproducibly.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in `0..n`; `n` is never zero here.
    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).unwrap_or(0)
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// True with a probability of `percent` / 100.
    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

/// One input the sweep sends.
#[derive(Clone)]
enum Action {
    Key(String),
    Mouse(MouseKind, i32, i32),
    Drag(MouseButton, (i32, i32), (i32, i32)),
    Paste(String),
    Advance(u64),
    Resize(u16, u16),
    Section(usize),
    ReducedMotion(bool),
    Stream(bool),
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(chord) => write!(f, "press({chord:?})"),
            Self::Mouse(kind, x, y) => write!(f, "mouse({kind:?}, {x}, {y})"),
            Self::Drag(button, from, to) => write!(f, "drag({button:?}, {from:?}, {to:?})"),
            Self::Paste(text) => write!(f, "paste({text:?})"),
            Self::Advance(ms) => write!(f, "advance({ms} ms)"),
            Self::Resize(w, h) => write!(f, "resize({w}, {h})"),
            Self::Section(section) => write!(f, "section({section})"),
            Self::ReducedMotion(on) => write!(f, "reduced_motion({on})"),
            Self::Stream(on) => write!(f, "stream({on})"),
        }
    }
}

const NAMED_KEYS: [&str; 16] = [
    "enter",
    "esc",
    "tab",
    "space",
    "backspace",
    "delete",
    "insert",
    "home",
    "end",
    "pgup",
    "pgdn",
    "up",
    "down",
    "left",
    "right",
    "f2",
];

const CHAR_KEYS: [&str; 20] =
    ["a", "c", "d", "e", "j", "k", "n", "q", "v", "w", "x", "z", "/", "?", "1", "2", "3", "4", "+", "-"];

const PASTES: [&str; 6] = ["", "nginx:1.27", "é👍🏽中文\tİı", "line one\nline two\r\n", "  ", "9999999999999999999999"];

const SIZES: [(u16, u16); 14] = [
    (0, 0),
    (1, 1),
    (0, 12),
    (40, 0),
    (1, 30),
    (200, 1),
    (2, 3),
    (7, 4),
    (19, 9),
    (40, 12),
    (80, 24),
    (140, 44),
    (220, 70),
    (400, 120),
];

fn chord(rng: &mut Rng) -> String {
    let key = if rng.chance(55) { *rng.pick(&NAMED_KEYS) } else { *rng.pick(&CHAR_KEYS) };
    let mut chord = String::new();
    if rng.chance(20) {
        chord.push_str("ctrl+");
    }
    if rng.chance(10) {
        chord.push_str("alt+");
    }
    if rng.chance(15) {
        chord.push_str("shift+");
    }
    chord.push_str(key);
    chord
}

/// A cell on screen most of the time, and now and then just outside it or far away.
fn position(rng: &mut Rng, size: (u16, u16)) -> (i32, i32) {
    let (w, h) = (i32::from(size.0), i32::from(size.1));
    if rng.chance(8) {
        let far = [-1, -40, w, w + 3, h, i32::from(u16::MAX), i32::MIN / 2];
        return (*rng.pick(&far), *rng.pick(&far));
    }
    let x = i32::try_from(rng.below(usize::from(size.0.max(1)))).unwrap_or(0);
    let y = i32::try_from(rng.below(usize::from(size.1.max(1)))).unwrap_or(0);
    (x, y)
}

fn button(rng: &mut Rng) -> MouseButton {
    *rng.pick(&[MouseButton::Left, MouseButton::Left, MouseButton::Left, MouseButton::Right, MouseButton::Middle])
}

fn action(rng: &mut Rng, size: (u16, u16), page: &str) -> Action {
    if page == STREAM_PAGE && rng.chance(4) {
        return Action::Stream(rng.chance(35));
    }
    let shell_page = page == SHELL_PAGE;
    let (x, y) = position(rng, size);
    if shell_page {
        return match rng.below(5) {
            0 => Action::Mouse(MouseKind::Moved, x, y),
            1 => Action::Mouse(MouseKind::ScrollDown, x, y),
            2 => Action::Mouse(MouseKind::ScrollUp, x, y),
            3 => Action::Advance(rng.next() % 700),
            _ => Action::Resize(rng.pick(&SIZES).0, rng.pick(&SIZES).1),
        };
    }
    match rng.below(100) {
        0..30 => Action::Key(chord(rng)),
        30..44 => Action::Mouse(MouseKind::Down(MouseButton::Left), x, y),
        44..52 => Action::Mouse(MouseKind::Up(button(rng)), x, y),
        52..58 => Action::Mouse(MouseKind::Down(button(rng)), x, y),
        58..64 => Action::Drag(button(rng), (x, y), position(rng, size)),
        64..68 => Action::Mouse(MouseKind::Drag(MouseButton::Left), x, y),
        68..74 => Action::Mouse(MouseKind::Moved, x, y),
        74..82 => Action::Mouse(*rng.pick(&[MouseKind::ScrollUp, MouseKind::ScrollDown]), x, y),
        82..86 => Action::Paste((*rng.pick(&PASTES)).to_owned()),
        86..91 => Action::Advance(*rng.pick(&[0, 16, 120, 400, 1500, 60_000])),
        91..96 => {
            let (w, h) = *rng.pick(&SIZES);
            Action::Resize(w, h)
        }
        96..99 => Action::Section(rng.below(5)),
        _ => Action::ReducedMotion(rng.chance(50)),
    }
}

fn apply(harness: &mut Harness<Showcase>, action: &Action) {
    match action {
        Action::Key(chord) => {
            harness.press(chord);
        }
        Action::Mouse(kind, x, y) => {
            harness.mouse(*kind, *x, *y);
        }
        Action::Drag(button, from, to) => {
            harness.mouse(MouseKind::Down(*button), from.0, from.1);
            let mid = (from.0.saturating_add(to.0) / 2, from.1.saturating_add(to.1) / 2);
            harness.mouse(MouseKind::Drag(*button), mid.0, mid.1);
            harness.mouse(MouseKind::Drag(*button), to.0, to.1);
            harness.mouse(MouseKind::Up(*button), to.0, to.1);
        }
        Action::Paste(text) => {
            harness.paste(text);
        }
        Action::Advance(ms) => {
            harness.advance(Duration::from_millis(*ms));
        }
        Action::Resize(w, h) => {
            harness.resize(*w, *h);
        }
        Action::Section(section) => {
            harness.send(Msg::Section(*section));
        }
        Action::ReducedMotion(on) => {
            harness.set_reduced_motion(*on);
        }
        Action::Stream(on) => {
            harness.send(Msg::Page(PageMsg::LogView(log_view::Msg::Streaming(*on))));
        }
    }
}

/// Where a run happened, enough to replay it.
#[derive(Debug, Clone, Copy)]
pub struct Run {
    seed: u64,
    page: &'static str,
    theme: &'static str,
    mode: GlyphMode,
}

/// What a failing run did last and why it failed.
pub struct Failure {
    run: Run,
    step: usize,
    message: String,
    recent: Vec<Action>,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Run { seed, page, theme, mode } = self.run;
        writeln!(f, "seed {seed:#x}, page `{page}`, {theme}, {mode:?}, step {}: {}", self.step, self.message)?;
        writeln!(
            f,
            "  replay: QUVYTA_FUZZ_SEED={seed} QUVYTA_FUZZ_SEEDS=1 QUVYTA_FUZZ_PAGE={page} QUVYTA_FUZZ_THEME={theme} \
             QUVYTA_FUZZ_MODE={mode:?} cargo test -p quvyta-framework-showcase fuzz_long -- --ignored"
        )?;
        for action in &self.recent {
            writeln!(f, "    {action:?}")?;
        }
        Ok(())
    }
}

/// The screen's size must be the size it was given: a widget that draws out of bounds would
/// otherwise have grown or corrupted the buffer.
fn check_buffer(harness: &Harness<Showcase>, size: (u16, u16)) -> Result<(), String> {
    let area = harness.buffer().area;
    let cells = usize::from(size.0) * usize::from(size.1);
    if (area.x, area.y, area.width, area.height) != (0, 0, size.0, size.1) || harness.buffer().content.len() != cells {
        return Err(format!("buffer is {area:?} with {} cells, expected {size:?}", harness.buffer().content.len()));
    }
    Ok(())
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_owned()))
        .unwrap_or_else(|| "panic without a message".to_owned())
}

/// Seed of one run: the base seed mixed with the page, theme and glyph mode, so every run of a
/// sweep walks its own path.
fn run_seed(base: u64, page: usize, theme: usize, mode: usize) -> u64 {
    let mut rng = Rng(base ^ ((page as u64) << 32) ^ ((theme as u64) << 16) ^ mode as u64);
    rng.next()
}

/// Opens `run.page` at the test size and sends `steps` random actions, returning the first
/// panic or out-of-bounds screen.
pub fn fuzz(run: Run, steps: usize) -> Result<(), Failure> {
    // A stack overflow aborts the process and cannot be caught, so a trace of runs (and, at level
    // 2, of actions) on stderr is the way to find the run that caused it.
    let trace = env_number("QUVYTA_FUZZ_TRACE").unwrap_or(0);
    if trace > 0 {
        eprintln!("start {run:?}");
    }
    let mut rng = Rng(run.seed);
    let mut recent: Vec<Action> = Vec::new();
    let mut size = super::SIZE;
    let mut step = 0;
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut harness = Harness::with_env(Showcase::new(), env(), size.0, size.1);
        harness.set_locale("en").set_theme(run.theme).set_glyph_mode(run.mode);
        harness.send(Msg::Open(run.page.to_owned()));
        harness.advance(Duration::from_secs(1));
        while step < steps {
            // Pages link to each other (menu, palette, links); the sweep stays on its page.
            if harness.app().current() != run.page {
                harness.send(Msg::Open(run.page.to_owned()));
            }
            let action = action(&mut rng, size, run.page);
            if let Action::Resize(w, h) = action {
                size = (w, h);
            }
            if trace > 1 {
                eprintln!("  {step}: {action:?}");
            }
            recent.push(action.clone());
            if recent.len() > 24 {
                recent.remove(0);
            }
            apply(&mut harness, &action);
            check_buffer(&harness, size)?;
            step += 1;
        }
        Ok(())
    }));
    if trace > 0 {
        eprintln!("end {run:?}");
    }
    let message = match outcome {
        Ok(Ok(())) => return Ok(()),
        Ok(Err(message)) => message,
        Err(payload) => format!("panicked: {}", panic_message(payload.as_ref())),
    };
    Err(Failure { run, step, message, recent })
}

fn env_number(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.parse().ok()
}

/// Runs the sweep over `runs` on every core, collecting every failure rather than stopping at
/// the first.
fn sweep(runs: &[Run], steps: usize) {
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    let chunk = runs.len().div_ceil(threads).max(1);
    let failures: Vec<Failure> = std::thread::scope(|scope| {
        let workers: Vec<_> = runs
            .chunks(chunk)
            .map(|runs| scope.spawn(move || runs.iter().filter_map(|run| fuzz(*run, steps).err()).collect::<Vec<_>>()))
            .collect();
        workers.into_iter().flat_map(|worker| worker.join().unwrap_or_default()).collect()
    });
    let report: String = failures.iter().map(ToString::to_string).collect();
    assert!(failures.is_empty(), "{} fuzz run(s) failed:\n{report}", failures.len());
}

/// The gate's sweep: every page once, with the theme and glyph mode rotating over the pages.
#[test]
fn fuzz_every_page() {
    let runs: Vec<Run> = PAGES
        .iter()
        .enumerate()
        .map(|(index, page)| {
            let (theme, mode) = (index % THEMES.len(), index % MODES.len());
            Run {
                seed: run_seed(GATE_SEED, index, theme, mode),
                page: page.id,
                theme: THEMES[theme],
                mode: MODES[mode],
            }
        })
        .collect();
    sweep(&runs, GATE_STEPS);
}

/// The long sweep: every page in every theme and glyph mode, for several seeds.
#[test]
#[ignore = "long: run with --release -- --ignored"]
fn fuzz_long() {
    let seeds = env_number("QUVYTA_FUZZ_SEEDS").unwrap_or(4);
    let first = env_number("QUVYTA_FUZZ_SEED");
    let steps = env_number("QUVYTA_FUZZ_STEPS").map_or(600, |n| usize::try_from(n).unwrap_or(600));
    let only = |name: &str, value: &str| std::env::var(name).is_ok_and(|only| only != value);
    let mut runs = Vec::new();
    for (index, page) in PAGES.iter().enumerate() {
        if only("QUVYTA_FUZZ_PAGE", page.id) {
            continue;
        }
        for (theme_index, theme) in THEMES.iter().enumerate() {
            if only("QUVYTA_FUZZ_THEME", theme) {
                continue;
            }
            for (mode_index, mode) in MODES.iter().enumerate() {
                if only("QUVYTA_FUZZ_MODE", &format!("{mode:?}")) {
                    continue;
                }
                for round in 0..seeds {
                    let seed = match first {
                        Some(seed) => seed + round,
                        None => run_seed(GATE_SEED + round + 1, index, theme_index, mode_index),
                    };
                    runs.push(Run { seed, page: page.id, theme, mode: *mode });
                }
            }
        }
    }
    sweep(&runs, steps);
}
