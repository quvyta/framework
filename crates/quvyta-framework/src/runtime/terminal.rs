//! Running an application in a real terminal.

use std::cell::Cell;
use std::io::{self, Stdout, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::clipboard::CopyToClipboard;
use crossterm::event::{
    self as ct, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
    disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement,
};
use crossterm::{cursor, execute};
use ratatui_core::terminal::Terminal;
use ratatui_crossterm::CrosstermBackend;

use super::app::App;
use super::detached::{self, DetachedOutcome};
use super::engine::{Engine, HandOver, TaskMode};
use super::handoff::{self, HandoffOutcome, HandoffScreen};
use super::signals::Signals;
use super::terminal_clipboard::TerminalClipboard;
use super::termination::Termination;
use crate::env::{AssetDirs, Env};
use crate::event::{Event, KeyEvent, KeyKind, MouseButton, MouseEvent, MouseKind};
use crate::keymap::{Key, KeyChord, Modifiers};
use crate::storage::{Preferences, Settings};

/// How long the loop sleeps when nothing is animating and no background work is running.
const IDLE_WAIT: Duration = Duration::from_millis(500);
/// How often finished background work is picked up.
const TASK_WAIT: Duration = Duration::from_millis(20);

/// Configures and runs an application in the terminal.
pub struct Runtime<A: App> {
    app: A,
    dirs: AssetDirs,
    theme: Option<String>,
    settings: Option<Settings>,
    preferences: Option<Preferences>,
}

impl<A: App> Runtime<A> {
    /// A runtime for `app` with built-in files only.
    pub fn new(app: A) -> Self {
        Self { app, dirs: AssetDirs::default(), theme: None, settings: None, preferences: None }
    }

    /// Loads theme files from `dir`.
    #[must_use]
    pub fn theme_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dirs.themes = Some(dir.into());
        self
    }

    /// Loads a theme file given as text, such as one compiled in with `include_str!`, so an
    /// installed program needs no files beside it. `file` names it in diagnostics and its stem
    /// is the theme id, the way a directory names its files. Text given this way wins over
    /// [`Runtime::theme_dir`].
    #[must_use]
    pub fn theme_source(mut self, file: impl Into<String>, text: impl Into<String>) -> Self {
        self.dirs.theme_sources.push((file.into(), text.into()));
        self
    }

    /// Loads icon set files from `dir`.
    #[must_use]
    pub fn icon_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dirs.icons = Some(dir.into());
        self
    }

    /// Loads an icon set given as text, such as one compiled in with `include_str!`, so an
    /// installed program needs no files beside it. `file` names it in diagnostics and its stem
    /// is the icon set id, the way a directory names its files. Text given this way wins over
    /// [`Runtime::icon_dir`].
    ///
    /// This is also how an application gives its own icons: keys the built-in set lacks, such as
    /// `category.internet`, are drawn by every widget that takes an icon key, in whatever set the
    /// theme chooses and in the glyph mode in use. A key the built-in set has, such as `check`,
    /// restyles the framework's icon only while a theme names this set; see
    /// [`IconSetRegistry`](crate::icons::IconSetRegistry).
    #[must_use]
    pub fn icon_source(mut self, file: impl Into<String>, text: impl Into<String>) -> Self {
        self.dirs.icon_sources.push((file.into(), text.into()));
        self
    }

    /// Loads locale files from `dir`.
    #[must_use]
    pub fn locale_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dirs.locales = Some(dir.into());
        self
    }

    /// Loads a locale file given as text, such as one compiled in with `include_str!`, so an
    /// installed program needs no files beside it. `file` names it in diagnostics. Text given
    /// this way wins over [`Runtime::locale_dir`].
    ///
    /// ```no_run
    /// # use qframe::prelude::*;
    /// # struct Hello;
    /// # impl App for Hello {
    /// #     type Msg = ();
    /// #     fn update(&mut self, _: ()) -> Command<()> { Command::none() }
    /// #     fn view(&self, ui: &mut View<'_, ()>) { ui.add(Text::new(t!("app.greeting"))); }
    /// # }
    /// # fn main() -> std::io::Result<()> {
    /// let english = "[meta]\nname = \"English\"\ncode = \"en\"\n[app]\ngreeting = \"Hello\"\n";
    /// Runtime::new(Hello).locale_source("en.toml", english).run()
    /// # }
    /// ```
    #[must_use]
    pub fn locale_source(mut self, file: impl Into<String>, text: impl Into<String>) -> Self {
        self.dirs.locale_sources.push((file.into(), text.into()));
        self
    }

    /// Layers keymap `file` over the built-in keymap.
    #[must_use]
    pub fn keymap_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.dirs.keymap = Some(file.into());
        self
    }

    /// Layers a keymap given as text over the built-in keymap, such as one compiled in with
    /// `include_str!`, so an installed program needs no files beside it. `file` names it in
    /// diagnostics. Text given this way wins over [`Runtime::keymap_file`].
    ///
    /// ```no_run
    /// # use qframe::prelude::*;
    /// # struct Hello;
    /// # impl App for Hello {
    /// #     type Msg = ();
    /// #     fn update(&mut self, _: ()) -> Command<()> { Command::none() }
    /// #     fn view(&self, ui: &mut View<'_, ()>) { ui.add(Text::new("hello")); }
    /// # }
    /// # fn main() -> std::io::Result<()> {
    /// let keys = "[app]\nsave = \"ctrl+s\"\n";
    /// Runtime::new(Hello).keymap_source("keymap.toml", keys).run()
    /// # }
    /// ```
    #[must_use]
    pub fn keymap_source(mut self, file: impl Into<String>, text: impl Into<String>) -> Self {
        self.dirs.keymap_source = Some((file.into(), text.into()));
        self
    }

    /// Starts with theme `id` instead of the default.
    #[must_use]
    pub fn theme(mut self, id: impl Into<String>) -> Self {
        self.theme = Some(id.into());
        self
    }

    /// Starts with the theme, language, icon mode and reduced motion saved in `settings`, so
    /// the first frame already looks the way the user left it. Saved values win over
    /// [`Runtime::theme`].
    #[must_use]
    pub fn settings(mut self, settings: &Settings) -> Self {
        self.settings = Some(settings.clone());
        self
    }

    /// Starts with the language, theme and icons of the ecosystem's shared
    /// [`Preferences`], as [`Ecosystem::preferences`](crate::storage::Ecosystem::preferences) resolved
    /// them for this application. They win over [`Runtime::theme`] and over the same keys in
    /// [`Runtime::settings`], which keeps the rest: reduced motion, pillar and slide.
    ///
    /// ```no_run
    /// # use qframe::prelude::*;
    /// # use qframe::i18n::I18n;
    /// # use qframe::storage::{Ecosystem, Settings};
    /// # struct Hello;
    /// # impl App for Hello {
    /// #     type Msg = ();
    /// #     fn update(&mut self, _: ()) -> Command<()> { Command::none() }
    /// #     fn view(&self, ui: &mut View<'_, ()>) { ui.add(Text::new("hello")); }
    /// # }
    /// # fn main() -> std::io::Result<()> {
    /// let ecosystem = Ecosystem::QUVYTA;
    /// let settings = Settings::load_member(&ecosystem, "hello");
    /// let prefs = ecosystem.preferences("hello", &I18n::builtin());
    /// Runtime::new(Hello).settings(&settings).preferences(&prefs).run()
    /// # }
    /// ```
    #[must_use]
    pub fn preferences(mut self, preferences: &Preferences) -> Self {
        self.preferences = Some(preferences.clone());
        self
    }

    /// Takes over the terminal and runs until the application quits. The terminal is restored
    /// on return and on panic.
    ///
    /// # Signals
    ///
    /// On Unix the run catches `SIGTERM`, `SIGINT` and `SIGHUP` and ends gracefully instead of
    /// dying on the spot: the application hears the cause through
    /// [`App::terminating`](super::App::terminating), may save, and quits; see
    /// [`Termination`] for what each signal does and the grace it leaves. The loop is woken the
    /// moment a signal arrives, even while it waits for a key or a deadline.
    ///
    /// Every way out ends in bounded time: after the grace the run quits without the
    /// application, a second `SIGTERM` or `SIGINT` quits at once, and when the loop itself is
    /// stuck the process is ended a second later all the same, by the signal, after the terminal
    /// is restored. The terminal is left in application mode in no case while it exists; after a
    /// hangup nothing more is written to it.
    ///
    /// During a [`Handoff`](super::Handoff), and a [`DetachedHandoff`](super::DetachedHandoff)
    /// until the program's first line, the program owns the terminal's foreground. A
    /// signal the application catches meanwhile is passed on to the program, which ends the way
    /// it would have as a job of the shell; the application then takes the terminal back and
    /// hears the signal itself. A hangup reaches the program from the system anyway.
    ///
    /// Once `run` returns the signals have their usual effect again.
    ///
    /// # Errors
    ///
    /// Returns I/O errors from loading asset directories or from the terminal.
    pub fn run(self) -> io::Result<()> {
        let mut env = Env::load(&self.dirs)?;
        if let Some(theme) = &self.theme {
            env.set_theme(theme);
        }
        if let Some(settings) = &self.settings {
            env.apply_settings(settings);
        }
        if let Some(preferences) = &self.preferences {
            env.apply_preferences(preferences);
        }
        // Before the terminal is taken, so the modes restored if the process has to be ended by
        // force are the ones the user had.
        let signals = Signals::catch()?;
        let guard = TerminalGuard::enter()?;
        install_panic_hook();
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let result = event_loop(&mut terminal, Engine::new(self.app, env, TaskMode::Threads), &guard, &signals);
        if guard.abandoned.get() {
            // Dropping it would show the cursor on a terminal that is gone.
            std::mem::forget(terminal);
        } else {
            drop(terminal);
        }
        drop(guard);
        drop(signals);
        result
    }
}

fn event_loop<A: App>(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut engine: Engine<A>,
    guard: &TerminalGuard,
    signals: &Signals,
) -> io::Result<()> {
    let start = Instant::now();
    let mut clipboard = TerminalClipboard::default();
    // Set once the terminal hung up: from then on nothing is drawn, read or handed over, and the
    // run only finishes the application's work until it quits.
    let mut gone = false;
    loop {
        let now = start.elapsed();
        let heard = signals.take();
        if heard.resized {
            // The next frame measures the terminal again, even when crossterm's own resize
            // event has not been read yet.
            engine.dirty = true;
        }
        for cause in heard.causes {
            if cause == Termination::Hangup && !gone && signals.terminal_gone() {
                gone = true;
                guard.abandon();
            }
            engine.terminate(cause, now);
        }
        engine.poll_tasks();
        engine.run_queued_work();
        if gone {
            refuse_handoffs(&mut engine);
        } else {
            run_handoffs(terminal, &mut engine, guard, signals);
            // Output to a terminal that just hung up fails before its signal is heard.
            if let Err(error) = draw(terminal, &mut engine, &mut clipboard, start) {
                hang_up_or(error, signals, guard, &mut gone)?;
            }
        }
        engine.end_when_due(start.elapsed());
        if engine.quit {
            return Ok(());
        }
        let now = start.elapsed();
        let mut wait = match (gone, engine.deadline()) {
            // Frames are not drawn any more, so their deadlines never move.
            (true, _) | (false, None) => IDLE_WAIT,
            (false, Some(deadline)) => deadline.saturating_sub(now),
        };
        if let Some(deadline) = engine.ending_deadline() {
            wait = wait.min(deadline.saturating_sub(now));
        }
        if engine.pending_tasks > 0 || (!gone && engine.clipboard_reader.is_reading()) {
            wait = wait.min(TASK_WAIT);
        }
        if let Some(deadline) = clipboard.deadline().filter(|_| !gone) {
            wait = wait.min(deadline.saturating_sub(now));
        }
        // A frame the frame limit holds back: wake when the gap is over, not with the next
        // idle wait, so the limit paces frames without adding latency of its own.
        let held = if gone { None } else { engine.frame_deadline(now) };
        if let Some(at) = held {
            wait = wait.min(at.saturating_sub(now));
        }
        if (engine.dirty && held.is_none() && !gone) || engine.has_queued_work() {
            wait = Duration::ZERO;
        }
        if gone {
            signals.wait(wait, false)?;
            continue;
        }
        match read_input(&mut engine, &mut clipboard, signals, start, wait) {
            Ok(true) => hang_up(signals, guard, &mut gone),
            Ok(false) => {}
            Err(error) => hang_up_or(error, signals, guard, &mut gone)?,
        }
    }
}

/// Draws a frame when one is due, after the terminal clipboard and timed input had their turn.
/// What is due is the engine's answer: a frame the view or an animation wants, unless the frame
/// limit holds it back; a frame answering input is never held back.
fn draw<A: App>(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    engine: &mut Engine<A>,
    clipboard: &mut TerminalClipboard,
    start: Instant,
) -> io::Result<()> {
    let now = start.elapsed();
    clipboard.update(engine, now)?;
    engine.tick(now);
    if engine.frame_due(now) {
        execute!(io::stdout(), BeginSynchronizedUpdate)?;
        terminal.draw(|frame| engine.render(frame.buffer_mut(), start.elapsed()))?;
        execute!(io::stdout(), EndSynchronizedUpdate)?;
        for text in engine.clipboard.drain(..) {
            execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text))?;
        }
    }
    Ok(())
}

/// Waits up to `wait` for the keyboard or a signal and hands every waiting event to the engine.
/// Returns whether the terminal hung up instead.
fn read_input<A: App>(
    engine: &mut Engine<A>,
    clipboard: &mut TerminalClipboard,
    signals: &Signals,
    start: Instant,
    wait: Duration,
) -> io::Result<bool> {
    // Events crossterm already read ahead come first: the terminal has nothing more to say about
    // them, so waiting on it would not end.
    let Some(mut ready) = event_waiting(signals)? else {
        return Ok(true);
    };
    if !ready && !wait.is_zero() {
        let woken = signals.wait(wait, true)?;
        if woken.hung_up {
            return Ok(true);
        }
        if woken.keyboard {
            let Some(waiting) = event_waiting(signals)? else {
                return Ok(true);
            };
            ready = waiting;
        }
    }
    while ready {
        let event = ct::read()?;
        if let ct::Event::Resize(..) = event {
            engine.dirty = true;
        }
        let more = event_waiting(signals)?;
        ready = more == Some(true);
        for event in clipboard.filter(event, ready, engine, start.elapsed()) {
            if let Some(event) = translate(event) {
                engine.handle(event, start.elapsed());
            }
        }
        if more.is_none() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether crossterm has an event to read, or `None` when the terminal hung up. Crossterm is
/// asked only while the terminal is there: on a terminal that hung up every read finds nothing,
/// and its reader would keep reading forever.
fn event_waiting(signals: &Signals) -> io::Result<Option<bool>> {
    if signals.hung_up_now() {
        return Ok(None);
    }
    ct::poll(Duration::ZERO).map(Some)
}

/// Handles a failed exchange with the terminal: when the terminal is gone, it hung up and the
/// run goes on without it; otherwise the error ends the run.
fn hang_up_or(error: io::Error, signals: &Signals, guard: &TerminalGuard, gone: &mut bool) -> io::Result<()> {
    if !signals.terminal_gone() {
        return Err(error);
    }
    hang_up(signals, guard, gone);
    Ok(())
}

/// The terminal hung up: nothing is written to it again, and the application hears a hangup
/// whether or not its `SIGHUP` arrives.
fn hang_up(signals: &Signals, guard: &TerminalGuard, gone: &mut bool) {
    *gone = true;
    guard.abandon();
    signals.hung_up();
}

/// Answers the handoffs the engine queued after the terminal hung up: there is nothing to hand
/// over, so each one fails without running its program.
fn refuse_handoffs<A: App>(engine: &mut Engine<A>) {
    const GONE: &str = "the terminal is gone";
    while let Some(work) = engine.take_handoff() {
        let message = match work {
            HandOver::Wait(handoff) => handoff.finish(HandoffOutcome::Failed(GONE.to_owned())),
            HandOver::Detach(handoff) => handoff.finish(DetachedOutcome::Failed(GONE.to_owned()), engine.deliveries()),
        };
        engine.update(message);
    }
}

/// Runs the handoffs the engine queued, oldest first, each one blocking this thread: the screen
/// is given back, the program runs with the terminal to itself, and afterwards the application
/// takes the screen and draws all of it again. The engine owns no terminal, so this is the only
/// place a handoff can happen.
fn run_handoffs<A: App>(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    engine: &mut Engine<A>,
    guard: &TerminalGuard,
    signals: &Signals,
) {
    while let Some(work) = engine.take_handoff() {
        let prompt = engine.env.i18n().translate("quvyta.handoff.pause", &[]);
        let deliveries = engine.deliveries();
        let message = {
            let mut release = |notice: Option<&str>| -> io::Result<()> {
                guard.suspend()?;
                let mut out = io::stdout();
                execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
                if let Some(text) = notice {
                    writeln!(out, "{text}")?;
                }
                out.flush()
            };
            let mut take = || -> io::Result<()> {
                // Even when a step of taking the terminal back failed, the rest of it happened
                // and the next frame must be drawn whole, so the failure is reported afterwards.
                let resumed = guard.resume();
                // The program wrote over the screen we left, so nothing of it can be reused, and
                // it may have been resized meanwhile. Resizing to the size the terminal has now
                // clears it and empties the buffer the next frame is compared against, so every
                // cell is drawn again. `Terminal::clear` would do the same but first ask the
                // terminal where its cursor is, a round trip some terminals never answer.
                let area = terminal.size()?.into();
                terminal.resize(area)?;
                resumed
            };
            let mut wait_for_key = || wait_for_key_press(&prompt, signals);
            let mut screen = HandoffScreen { release: &mut release, take: &mut take, wait_for_key: &mut wait_for_key };
            // Signals caught while the program owns the terminal are passed on to it.
            signals.handoff(true);
            let message = match work {
                HandOver::Wait(handoff) => handoff::run(handoff, &mut screen),
                HandOver::Detach(handoff) => detached::run(handoff, &mut screen, &deliveries),
            };
            signals.handoff(false);
            message
        };
        engine.dirty = true;
        engine.update(message);
    }
}

/// Prints `prompt` on the screen the program leaves behind and waits for one key press.
fn wait_for_key_press(prompt: &str, signals: &Signals) -> io::Result<()> {
    let mut out = io::stdout();
    write!(out, "\n{prompt}")?;
    out.flush()?;
    // The keys are still the terminal's to echo; raw mode makes one press enough.
    enable_raw_mode()?;
    let pressed = wait_for_key(signals);
    disable_raw_mode()?;
    writeln!(out)?;
    pressed
}

/// Waits for one key press, or for a signal that ends the run: nobody should have to press a
/// key for the application to hear it.
fn wait_for_key(signals: &Signals) -> io::Result<()> {
    loop {
        if signals.pending() {
            return Ok(());
        }
        match event_waiting(signals)? {
            // Nobody is left to press a key.
            None => return Ok(()),
            Some(true) => {
                if let ct::Event::Key(key) = ct::read()?
                    && key.kind == ct::KeyEventKind::Press
                {
                    return Ok(());
                }
            }
            Some(false) => {
                if signals.wait(IDLE_WAIT, true)?.hung_up {
                    return Ok(());
                }
            }
        }
    }
}

/// Puts the terminal into application mode and restores it when dropped. The pair of
/// [`TerminalGuard::suspend`] and [`TerminalGuard::resume`] gives the terminal back for a while,
/// for a [`Handoff`](super::Handoff), and takes it again with the same keyboard enhancement flags.
struct TerminalGuard {
    keyboard_enhanced: bool,
    /// Set when the terminal hung up: there is nothing left to restore, and nothing is written
    /// to a terminal that is gone.
    abandoned: Cell<bool>,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        // From here on the guard exists, so a failure below drops it and the terminal is
        // restored instead of being left in raw mode.
        let mut guard = Self { keyboard_enhanced: false, abandoned: Cell::new(false) };
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste, cursor::Hide)?;
        // Asked once: the terminal cannot change its answer while the application runs, and the
        // question costs a round trip to it.
        guard.keyboard_enhanced = supports_keyboard_enhancement().unwrap_or(false);
        guard.push_keyboard_flags()?;
        Ok(guard)
    }

    /// Gives the terminal back: raw mode off, the normal screen and the cursor again.
    fn suspend(&self) -> io::Result<()> {
        release(self.keyboard_enhanced)
    }

    /// Takes the terminal again after [`TerminalGuard::suspend`], flags and all. The caller
    /// redraws afterwards, because the screen it left is gone.
    fn resume(&self) -> io::Result<()> {
        take_back(&mut io::stdout(), self.keyboard_enhanced, enable_raw_mode)
    }

    /// Gives up the terminal after it hung up: dropping the guard then writes nothing.
    fn abandon(&self) {
        self.abandoned.set(true);
    }

    fn push_keyboard_flags(&self) -> io::Result<()> {
        push_keyboard_flags(&mut io::stdout(), self.keyboard_enhanced)
    }
}

/// Takes the terminal again: raw mode through `raw_on`, then the screen, the mouse and the
/// keyboard flags written to `out`. Every step is tried even when one before it failed, so a
/// failure leaves the terminal as close to application mode as it can be; the first error is
/// the one reported.
fn take_back(out: &mut impl Write, keyboard_enhanced: bool, raw_on: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    let raw = raw_on();
    let screen = execute!(out, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste, cursor::Hide);
    let flags = push_keyboard_flags(out, keyboard_enhanced);
    raw.and(screen).and(flags)
}

fn push_keyboard_flags(out: &mut impl Write, keyboard_enhanced: bool) -> io::Result<()> {
    if keyboard_enhanced {
        execute!(
            out,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        )?;
    }
    Ok(())
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.abandoned.get() {
            restore(self.keyboard_enhanced);
        }
    }
}

/// Leaves application mode, reporting what failed.
fn release(keyboard_enhanced: bool) -> io::Result<()> {
    give_back(&mut io::stdout(), keyboard_enhanced, disable_raw_mode)
}

/// Leaves application mode: the keyboard flags, the mouse and the screen written to `out`, and
/// raw mode through `raw_off`. Every step is tried even when one before it failed: raw mode is a
/// setting of the terminal device, not output, and output that cannot be written must not leave
/// the user's shell in raw mode. The first error is the one reported.
pub(super) fn give_back(
    out: &mut impl Write,
    keyboard_enhanced: bool,
    raw_off: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let flags = if keyboard_enhanced { execute!(out, PopKeyboardEnhancementFlags) } else { Ok(()) };
    let screen = execute!(out, DisableBracketedPaste, DisableMouseCapture, LeaveAlternateScreen, cursor::Show);
    let raw = raw_off();
    let flushed = out.flush();
    flags.and(screen).and(raw).and(flushed)
}

/// Leaves application mode as far as it can, for a drop or a panic: nothing is left to report to.
fn restore(keyboard_enhanced: bool) {
    let _ = release(keyboard_enhanced);
}

fn install_panic_hook() {
    on_panic_in_this_thread(|| restore(true));
}

/// Runs `on_panic` before the panic hook that was installed, for panics on the calling thread
/// only. Hooks run for every panic, even one a background task catches and reports as its
/// outcome; restoring the terminal then would leave the running application on the normal
/// screen without raw mode.
fn on_panic_in_this_thread(on_panic: impl Fn() + Send + Sync + 'static) {
    let owner = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().id() == owner {
            on_panic();
        }
        previous(info);
    }));
}

/// Converts a crossterm event; events the framework does not use become `None`.
fn translate(event: ct::Event) -> Option<Event> {
    match event {
        ct::Event::Key(key) => translate_key(key).map(Event::Key),
        ct::Event::Mouse(mouse) => translate_mouse(mouse).map(Event::Mouse),
        ct::Event::Paste(text) => Some(Event::Paste(text)),
        ct::Event::FocusGained | ct::Event::FocusLost | ct::Event::Resize(..) => None,
    }
}

fn modifiers(mods: ct::KeyModifiers) -> Modifiers {
    Modifiers {
        ctrl: mods.contains(ct::KeyModifiers::CONTROL),
        alt: mods.contains(ct::KeyModifiers::ALT),
        shift: mods.contains(ct::KeyModifiers::SHIFT),
    }
}

fn translate_key(key: ct::KeyEvent) -> Option<KeyEvent> {
    let mut mods = modifiers(key.modifiers);
    let code = match key.code {
        ct::KeyCode::Char(' ') => Key::Space,
        ct::KeyCode::Char(c) if c.is_uppercase() => {
            mods.shift = true;
            Key::Char(c.to_lowercase().next().unwrap_or(c))
        }
        ct::KeyCode::Char(c) => {
            if !c.is_alphabetic() {
                mods.shift = false;
            }
            Key::Char(c)
        }
        ct::KeyCode::Enter => Key::Enter,
        ct::KeyCode::Esc => Key::Esc,
        ct::KeyCode::Tab => Key::Tab,
        ct::KeyCode::BackTab => {
            mods.shift = true;
            Key::Tab
        }
        ct::KeyCode::Backspace => Key::Backspace,
        ct::KeyCode::Delete => Key::Delete,
        ct::KeyCode::Insert => Key::Insert,
        ct::KeyCode::Home => Key::Home,
        ct::KeyCode::End => Key::End,
        ct::KeyCode::PageUp => Key::PageUp,
        ct::KeyCode::PageDown => Key::PageDown,
        ct::KeyCode::Up => Key::Up,
        ct::KeyCode::Down => Key::Down,
        ct::KeyCode::Left => Key::Left,
        ct::KeyCode::Right => Key::Right,
        ct::KeyCode::F(n) => Key::F(n),
        ct::KeyCode::Menu => Key::Menu,
        _ => return None,
    };
    let kind = match key.kind {
        ct::KeyEventKind::Press => KeyKind::Press,
        ct::KeyEventKind::Repeat => KeyKind::Repeat,
        ct::KeyEventKind::Release => KeyKind::Release,
    };
    let text = match key.code {
        ct::KeyCode::Char(c) if !mods.ctrl && !mods.alt => Some(c),
        _ => None,
    };
    Some(KeyEvent { chord: KeyChord { key: code, mods }, kind, text })
}

fn translate_mouse(mouse: ct::MouseEvent) -> Option<MouseEvent> {
    let button = |b: ct::MouseButton| match b {
        ct::MouseButton::Left => MouseButton::Left,
        ct::MouseButton::Right => MouseButton::Right,
        ct::MouseButton::Middle => MouseButton::Middle,
    };
    let kind = match mouse.kind {
        ct::MouseEventKind::Down(b) => MouseKind::Down(button(b)),
        ct::MouseEventKind::Up(b) => MouseKind::Up(button(b)),
        ct::MouseEventKind::Drag(b) => MouseKind::Drag(button(b)),
        ct::MouseEventKind::Moved => MouseKind::Moved,
        ct::MouseEventKind::ScrollUp => MouseKind::ScrollUp,
        ct::MouseEventKind::ScrollDown => MouseKind::ScrollDown,
        ct::MouseEventKind::ScrollLeft | ct::MouseEventKind::ScrollRight => return None,
    };
    Some(MouseEvent { kind, x: i32::from(mouse.column), y: i32::from(mouse.row), mods: modifiers(mouse.modifiers) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panics_on_other_threads_leave_the_terminal_alone() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let restores = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&restores);
        on_panic_in_this_thread(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        // A task's panic is caught and becomes its outcome; the application keeps running.
        let _ = std::thread::spawn(|| panic!("a background task failed")).join();
        assert_eq!(restores.load(Ordering::SeqCst), 0, "the terminal stays in application mode");
        let _ = std::panic::catch_unwind(|| panic!("the runtime failed"));
        assert_eq!(restores.load(Ordering::SeqCst), 1, "a panic of the runtime thread restores it");
    }

    /// Output that cannot be written, as when the terminal went away.
    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("the terminal is gone"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("the terminal is gone"))
        }
    }

    #[test]
    fn raw_mode_is_left_even_when_the_screen_cannot_be_written() {
        let mut raw_left = false;
        let result = give_back(&mut Broken, true, || {
            raw_left = true;
            Ok(())
        });
        assert!(raw_left, "raw mode is a terminal setting, not output, and is always left");
        assert_eq!(result.expect_err("the failure is reported").to_string(), "the terminal is gone");
    }

    #[test]
    fn leaving_application_mode_writes_every_step_after_one_fails() {
        let mut out = Vec::new();
        let result = give_back(&mut out, true, || Err(io::Error::other("no raw mode")));
        assert_eq!(result.expect_err("the failure is reported").to_string(), "no raw mode");
        let text = String::from_utf8(out).expect("escape codes");
        assert!(text.contains("\x1b[?1049l"), "the alternate screen was left: {text:?}");
        assert!(text.contains("\x1b[?25h"), "the cursor is shown again: {text:?}");
    }

    #[test]
    fn taking_the_terminal_back_goes_on_when_raw_mode_fails() {
        // Without the alternate screen the application would draw over the shell's own lines.
        let mut out = Vec::new();
        let result = take_back(&mut out, false, || Err(io::Error::other("no raw mode")));
        assert_eq!(result.expect_err("the failure is reported").to_string(), "no raw mode");
        let text = String::from_utf8(out).expect("escape codes");
        assert!(text.contains("\x1b[?1049h"), "the alternate screen is entered again: {text:?}");
    }

    #[test]
    fn translates_uppercase_and_backtab() {
        let key = |code, mods| ct::KeyEvent::new(code, mods);
        let a = translate_key(key(ct::KeyCode::Char('A'), ct::KeyModifiers::SHIFT)).expect("key");
        assert_eq!(a.chord, "shift+a".parse().expect("chord"));
        assert_eq!(a.text, Some('A'));
        let question = translate_key(key(ct::KeyCode::Char('?'), ct::KeyModifiers::SHIFT)).expect("key");
        assert_eq!(question.chord, "?".parse().expect("chord"));
        let back = translate_key(key(ct::KeyCode::BackTab, ct::KeyModifiers::SHIFT)).expect("key");
        assert_eq!(back.chord, "shift+tab".parse().expect("chord"));
        let ctrl = translate_key(key(ct::KeyCode::Char('q'), ct::KeyModifiers::CONTROL)).expect("key");
        assert_eq!(ctrl.chord, "ctrl+q".parse().expect("chord"));
        assert_eq!(ctrl.text, None);
        let menu = translate_key(key(ct::KeyCode::Menu, ct::KeyModifiers::NONE)).expect("key");
        assert_eq!(menu.chord, "menu".parse().expect("chord"));
    }
}
