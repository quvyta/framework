//! Running an application in a real terminal.

use std::io::{self, Stdout, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::clipboard::CopyToClipboard;
use crossterm::event::{
    self as ct, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, supports_keyboard_enhancement,
};
use crossterm::{cursor, execute};
use ratatui_core::terminal::Terminal;
use ratatui_crossterm::CrosstermBackend;

use super::app::App;
use super::engine::{Engine, TaskMode};
use super::terminal_clipboard::TerminalClipboard;
use crate::env::{AssetDirs, Env};
use crate::event::{Event, KeyEvent, KeyKind, MouseButton, MouseEvent, MouseKind};
use crate::keymap::{Key, KeyChord, Modifiers};
use crate::storage::Settings;

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
}

impl<A: App> Runtime<A> {
    /// A runtime for `app` with built-in files only.
    pub fn new(app: A) -> Self {
        Self { app, dirs: AssetDirs::default(), theme: None, settings: None }
    }

    /// Loads theme files from `dir`.
    #[must_use]
    pub fn theme_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dirs.themes = Some(dir.into());
        self
    }

    /// Loads icon set files from `dir`.
    #[must_use]
    pub fn icon_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dirs.icons = Some(dir.into());
        self
    }

    /// Loads locale files from `dir`.
    #[must_use]
    pub fn locale_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dirs.locales = Some(dir.into());
        self
    }

    /// Layers keymap `file` over the built-in keymap.
    #[must_use]
    pub fn keymap_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.dirs.keymap = Some(file.into());
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

    /// Takes over the terminal and runs until the application quits. The terminal is restored
    /// on return and on panic.
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
        let guard = TerminalGuard::enter()?;
        install_panic_hook();
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let result = event_loop(&mut terminal, Engine::new(self.app, env, TaskMode::Threads));
        drop(terminal);
        drop(guard);
        result
    }
}

fn event_loop<A: App>(terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut engine: Engine<A>) -> io::Result<()> {
    let start = Instant::now();
    let mut clipboard = TerminalClipboard::default();
    loop {
        let now = start.elapsed();
        engine.poll_tasks();
        engine.run_queued_work();
        clipboard.update(&mut engine, now)?;
        engine.tick(now);
        let animation_due = engine.deadline().is_some_and(|deadline| deadline <= now);
        if engine.dirty || animation_due {
            execute!(io::stdout(), BeginSynchronizedUpdate)?;
            terminal.draw(|frame| engine.render(frame.buffer_mut(), start.elapsed()))?;
            execute!(io::stdout(), EndSynchronizedUpdate)?;
            for text in engine.clipboard.drain(..) {
                execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text))?;
            }
        }
        if engine.quit {
            return Ok(());
        }
        let now = start.elapsed();
        let mut wait = engine.deadline().map_or(IDLE_WAIT, |deadline| deadline.saturating_sub(now));
        if engine.pending_tasks > 0 || engine.clipboard_reader.is_reading() {
            wait = wait.min(TASK_WAIT);
        }
        if let Some(deadline) = clipboard.deadline() {
            wait = wait.min(deadline.saturating_sub(now));
        }
        if engine.dirty || engine.has_queued_work() {
            wait = Duration::ZERO;
        }
        if ct::poll(wait)? {
            loop {
                let event = ct::read()?;
                if let ct::Event::Resize(..) = event {
                    engine.dirty = true;
                }
                let more = ct::poll(Duration::ZERO)?;
                for event in clipboard.filter(event, more, &mut engine, start.elapsed()) {
                    if let Some(event) = translate(event) {
                        engine.handle(event, start.elapsed());
                    }
                }
                if !more {
                    break;
                }
            }
        }
    }
}

/// Puts the terminal into application mode and restores it when dropped.
struct TerminalGuard {
    keyboard_enhanced: bool,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste, cursor::Hide)?;
        let keyboard_enhanced = supports_keyboard_enhancement().unwrap_or(false);
        if keyboard_enhanced {
            execute!(
                out,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            )?;
        }
        Ok(Self { keyboard_enhanced })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore(self.keyboard_enhanced);
    }
}

fn restore(keyboard_enhanced: bool) {
    let mut out = io::stdout();
    if keyboard_enhanced {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(out, DisableBracketedPaste, DisableMouseCapture, LeaveAlternateScreen, cursor::Show);
    let _ = disable_raw_mode();
    let _ = out.flush();
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
