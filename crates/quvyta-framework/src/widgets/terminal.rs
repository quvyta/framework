//! An embedded terminal: draws a [`TerminalSession`] in theme colours and types into it.

use crate::color::Rgb;
use crate::event::{Event, KeyEvent, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::{Key, KeyChord, Keymap, Modifiers, Scope};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::rows::WHEEL_ROWS;
use super::terminal_mouse;
use super::terminal_session::TerminalSession;

/// Lines one wheel step scrolls back.
const WHEEL_LINES: usize = WHEEL_ROWS as usize;

/// A terminal showing a [`TerminalSession`]: the program's screen drawn cell by cell, with the
/// sixteen classic colours taken from the theme so shells and tools match the application.
///
/// While focused every key goes to the program, including Tab and Esc, except `shift+tab`,
/// which moves focus on, and `ctrl+q`, which still quits; by default nothing else passes.
/// [`Terminal::pass_through`] lets the keys of chosen keymap actions through as well: they skip
/// the program and take the usual route, to ancestors such as a
/// [`SidePanel`](super::SidePanel), to key listeners and finally to
/// [`App::action`](crate::runtime::App::action). A key that types a character (a character or
/// the space key with at most `shift`) always goes to the program, even when it is bound to a
/// passed action, because the terminal is where the user types: with `help` on `?` and `f1`,
/// `?` types into the shell and `f1` opens the help. Pasted text is sent as a bracketed
/// paste when the program asks for it. The wheel scrolls back through earlier output (on the
/// alternate screen of full-screen programs it sends arrow keys instead); typing returns to the
/// live screen. The widget asks the session for its size; a running
/// [`TerminalWatch`](super::TerminalWatch) applies it.
///
/// Once the program has ended nothing the widget cannot deliver is swallowed: a key, a paste and
/// a wheel step on the alternate screen are handed back instead. A paste then reaches
/// [`App::clipboard`](crate::runtime::App::clipboard) as
/// [`ClipboardEvent::Pasted`](crate::runtime::ClipboardEvent::Pasted), so an application can say
/// that the text went nowhere rather than let it disappear. Mouse reports are the exception: a
/// program that has ended has nothing to learn about the pointer, and a press handed back would
/// act on whatever is behind the terminal.
///
/// [`Terminal::read_only`] is the same terminal with the writing taken away: it shows a session
/// faint, takes no focus and never sends anything to the program, while colours, wide characters,
/// the cursor, selecting and scrolling back stay as they are.
///
/// The output is a text selection region: a mouse drag selects inside it (turn it off with
/// [`NodeMut::selectable`](crate::widget::NodeMut::selectable)); clean copies leave out the
/// scrolled-back and exited notes.
///
/// A program that turns on mouse reporting (xterm modes 9, 1000, 1002 and 1003, in the default,
/// UTF-8 or SGR encoding) gets the mouse instead: presses, releases, drags, moves and the wheel
/// are sent to it as its mode asks, with cells counted from the terminal's corner. A press it
/// takes keeps the pointer until the release, so a drag past the edge still reports the nearest
/// cell. `shift` with a press or the wheel goes past the program: it selects text and scrolls
/// back as when the mouse is off.
///
/// Style keys: `terminal` (`bg`, `fg`, the default colours), `terminal-cursor` (`bg`, `fg`)
/// with `focus`, `terminal-note` (`bg`, `fg`) for the scrolled-back and exited notes. Framework
/// strings: `quvyta.terminal.scrolled`, `quvyta.terminal.exited`.
pub struct Terminal {
    session: TerminalSession,
    pass_through: Vec<(Scope, String)>,
    read_only: bool,
}

#[derive(Debug, Default)]
struct TerminalMemory {
    scrollback: usize,
}

impl Terminal {
    /// A terminal drawing `session`; cloning a session is cheap.
    #[must_use]
    pub fn new(session: &TerminalSession) -> Self {
        Self { session: session.clone(), pass_through: Vec::new(), read_only: false }
    }

    /// A terminal that only shows: it draws the session's screen and never writes to it, takes no
    /// focus by Tab or by a click, and is drawn faint.
    ///
    /// Everything else the widget does stays: the program's own colours, wide characters, the
    /// cursor it left behind, selecting and copying its output, and scrolling back — which is the
    /// reason to keep a program's last screen on the display at all. The size is not asked for
    /// either, so a window shrinking around a program that has ended cannot reflow the screen
    /// that was left to be read.
    ///
    /// This is the shape for a window whose program has ended and stays open so its last words
    /// can be read. Keys, pastes and wheel steps take the usual route past the terminal, to the
    /// buttons around it, so what a person types after the end reaches something that can answer.
    #[must_use]
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Lets the keys of a keymap action through to the application instead of the program,
    /// e.g. `.pass_through(Scope::Global, "toggle-panel")`; call again for each action. The
    /// keymap in force when the key arrives decides which keys those are, so rebinding follows.
    /// Keys that type a character still go to the program, see [`Terminal`].
    ///
    /// To tell a key passed out of the terminal from the same key pressed elsewhere, answer the
    /// action on the terminal's node with [`NodeMut::on_action`](crate::widget::NodeMut::on_action):
    /// a focus toggle passes `"focus-toggle"`, answers it with a message that focuses the rest
    /// of the application, and maps it in [`App::action`](crate::runtime::App::action) to one
    /// that returns focus with [`Command::focus`](crate::runtime::Command::focus).
    #[must_use]
    pub fn pass_through(mut self, scope: Scope, action: impl Into<String>) -> Self {
        self.pass_through.push((scope, action.into()));
        self
    }

    /// Whether `chord` belongs to the application rather than the program.
    fn passes(&self, keymap: &Keymap, chord: KeyChord) -> bool {
        !types_text(chord)
            && self.pass_through.iter().any(|(scope, action)| keymap.chords_for(*scope, action).contains(&chord))
    }
}

/// How far a view-only screen is mixed into its background: enough to read as inactive at a
/// glance, little enough to keep the program's own colours apart. The same share a cell the
/// program itself marks faint is drawn with.
const FAINT: f32 = 0.45;

/// Whether `chord` types a character, which always belongs to the program.
fn types_text(chord: KeyChord) -> bool {
    matches!(chord.key, Key::Char(_) | Key::Space) && !chord.mods.ctrl && !chord.mods.alt
}

/// The theme colour of one of the sixteen classic terminal colours.
fn classic(cx: &PaintCx<'_>, index: u8) -> Rgb {
    let base = |i: u8| match i {
        0 => cx.color("raised"),
        1 => cx.color("danger"),
        2 => cx.color("success"),
        3 => cx.color("warning"),
        4 => cx.color("info"),
        5 => cx.color("danger").mix(cx.color("info"), 0.5),
        6 => cx.color("info").mix(cx.color("success"), 0.5),
        _ => cx.color("dim"),
    };
    match index {
        8 => cx.color("muted"),
        15 => cx.color("text"),
        9..=14 => base(index - 8).mix(cx.color("text"), 0.25),
        _ => base(index),
    }
}

/// The xterm colour of palette entries 16 to 255.
fn xterm(index: u8) -> Rgb {
    if index >= 232 {
        let level = 8 + (index - 232) * 10;
        return Rgb::new(level, level, level);
    }
    let cube = index - 16;
    let level = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
    Rgb::new(level(cube / 36), level(cube / 6 % 6), level(cube % 6))
}

fn resolve(cx: &PaintCx<'_>, color: vt100::Color, default: Rgb) -> Rgb {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Idx(index) if index < 16 => classic(cx, index),
        vt100::Color::Idx(index) => xterm(index),
        vt100::Color::Rgb(r, g, b) => Rgb::new(r, g, b),
    }
}

/// The bytes a key sends to a program, following xterm.
fn key_bytes(key: &KeyEvent, application_cursor: bool) -> Vec<u8> {
    let mods = key.chord.mods;
    let modifier = 1 + u8::from(mods.shift) + 2 * u8::from(mods.alt) + 4 * u8::from(mods.ctrl);
    let cursor = |letter: char| {
        if modifier > 1 {
            format!("\x1b[1;{modifier}{letter}").into_bytes()
        } else if application_cursor {
            format!("\x1bO{letter}").into_bytes()
        } else {
            format!("\x1b[{letter}").into_bytes()
        }
    };
    let tilde = |code: u8| {
        if modifier > 1 {
            format!("\x1b[{code};{modifier}~").into_bytes()
        } else {
            format!("\x1b[{code}~").into_bytes()
        }
    };
    let alt = |mut bytes: Vec<u8>| {
        if mods.alt {
            bytes.insert(0, 0x1b);
        }
        bytes
    };
    match key.chord.key {
        Key::Char(c) if mods.ctrl => {
            let control = match c {
                'a'..='z' => Some(c as u8 - b'a' + 1),
                '@' | '2' => Some(0),
                '[' | '3' => Some(0x1b),
                '\\' | '4' => Some(0x1c),
                ']' | '5' => Some(0x1d),
                '^' | '6' => Some(0x1e),
                '_' | '7' | '-' => Some(0x1f),
                '?' | '8' => Some(0x7f),
                _ => None,
            };
            control.map_or_else(Vec::new, |byte| alt(vec![byte]))
        }
        Key::Char(c) => {
            let typed = key.text.unwrap_or(if mods.shift { c.to_ascii_uppercase() } else { c });
            alt(typed.to_string().into_bytes())
        }
        Key::Space if mods.ctrl => alt(vec![0]),
        Key::Space => alt(vec![b' ']),
        Key::Enter => alt(vec![b'\r']),
        Key::Tab if mods.shift => b"\x1b[Z".to_vec(),
        Key::Tab => vec![b'\t'],
        Key::Backspace => alt(vec![0x7f]),
        Key::Esc => vec![0x1b],
        Key::Up => cursor('A'),
        Key::Down => cursor('B'),
        Key::Right => cursor('C'),
        Key::Left => cursor('D'),
        Key::Home => cursor('H'),
        Key::End => cursor('F'),
        Key::Insert => tilde(2),
        Key::Delete => tilde(3),
        Key::PageUp => tilde(5),
        Key::PageDown => tilde(6),
        Key::F(n @ 1..=4) => format!("\x1bO{}", char::from(b'P' + n - 1)).into_bytes(),
        Key::F(n) => match n {
            5 => tilde(15),
            6 => tilde(17),
            7 => tilde(18),
            8 => tilde(19),
            9 => tilde(20),
            10 => tilde(21),
            11 => tilde(23),
            12 => tilde(24),
            _ => Vec::new(),
        },
        // Legacy terminals have no bytes for the menu key.
        Key::Menu => Vec::new(),
    }
}

/// Moves the widget's own view one wheel step back through earlier output, or forward again.
/// Nothing is written to the program: the screen is drawn from the lines the session already
/// keeps, which is why it also works for a program that has ended.
fn scroll_back<Msg>(cx: &mut EventCx<'_, Msg>, up: bool) {
    let memory = cx.memory::<TerminalMemory>();
    memory.scrollback =
        if up { memory.scrollback + WHEEL_LINES } else { memory.scrollback.saturating_sub(WHEEL_LINES) };
}

impl<Msg: 'static> Widget<Msg> for Terminal {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        if area.is_empty() {
            return;
        }
        cx.register_hit(area);
        cx.selectable(area);
        if !self.read_only {
            // Only a request: the watch resizes the pseudo-terminal off the drawing thread. A
            // view-only terminal does not ask: resizing is a write, and a screen left to be read
            // would be reflowed by the window it sits in changing size.
            self.session.request_size(area.height, area.width);
        }
        let focused = cx.is_focused();
        let base = cx.style("terminal", None, &[]).text();
        let default_bg = base.bg.unwrap_or_else(|| cx.color("surface"));
        let default_fg = base.fg.unwrap_or_else(|| cx.color("text"));
        cx.clear(area, default_bg);
        let cursor_states = if focused { vec![crate::theme::State::Focus] } else { Vec::new() };
        let mut cursor_style = cx.style("terminal-cursor", None, &cursor_states).text();
        if self.read_only {
            // The cursor the program left fades with the screen it sits on; it is part of the
            // picture, not a place text will appear.
            cursor_style.fg = cursor_style.fg.map(|color| color.mix(default_bg, FAINT));
            cursor_style.bg = cursor_style.bg.map(|color| color.mix(default_bg, FAINT));
        }
        let wanted = cx.memory::<TerminalMemory>().scrollback;

        let session = self.session.clone();
        let mut parser = session.parser();
        parser.screen_mut().set_scrollback(wanted);
        let screen = parser.screen();
        // Moves are only ever asked for to report them to the program, which a view-only terminal
        // never does.
        if !self.read_only && terminal_mouse::wants_moves(screen) {
            cx.track_pointer_moves();
        }
        let scrolled = screen.scrollback();
        let (rows, cols) = screen.size();
        for row in 0..rows.min(area.height) {
            let y = area.y + i32::from(row);
            for col in 0..cols.min(area.width) {
                let Some(cell) = screen.cell(row, col) else { continue };
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut fg = resolve(cx, cell.fgcolor(), default_fg);
                let mut bg = resolve(cx, cell.bgcolor(), default_bg);
                if cell.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if cell.dim() {
                    fg = fg.mix(bg, FAINT);
                }
                if self.read_only {
                    // Both colours are mixed into the background, so a screen with its own
                    // background tones fades as one picture instead of losing its text alone.
                    fg = fg.mix(default_bg, FAINT);
                    bg = bg.mix(default_bg, FAINT);
                }
                let style = CellStyle {
                    fg: Some(fg),
                    bg: Some(bg),
                    bold: cell.bold(),
                    italic: cell.italic(),
                    underline: cell.underline(),
                    dim: false,
                };
                let contents = cell.contents();
                let symbol = if contents.is_empty() { " " } else { contents };
                let width = if cell.is_wide() { 2 } else { 1 };
                cx.text(area.x + i32::from(col), y, symbol, style, width);
            }
        }
        if scrolled == 0 && !screen.hide_cursor() {
            let (row, col) = screen.cursor_position();
            if row < area.height && col < area.width {
                let symbol = screen.cell(row, col).map(|c| c.contents().to_owned()).unwrap_or_default();
                let symbol = if symbol.is_empty() { " ".to_owned() } else { symbol };
                cx.text(area.x + i32::from(col), area.y + i32::from(row), &symbol, cursor_style, 1);
            }
        }
        drop(parser);
        cx.memory::<TerminalMemory>().scrollback = scrolled;

        let note = if let Some(code) = self.session.exit() {
            let code = code.map_or_else(|| "?".to_owned(), |c| c.to_string());
            Some(crate::i18n::translate_active("quvyta.terminal.exited", &[("code", code.into())]))
        } else if scrolled > 0 {
            Some(crate::i18n::translate_active("quvyta.terminal.scrolled", &[("n", scrolled.into())]))
        } else {
            None
        };
        if let Some(note) = note {
            let width = text::width(&note).saturating_add(2).min(area.width);
            let rect = Rect::new(area.right() - i32::from(width), area.bottom() - 1, width, 1);
            let style = cx.style("terminal-note", None, &[]).text();
            if let Some(bg) = style.bg {
                cx.clear(rect, bg);
            }
            // The note is the widget's, not the program's output.
            cx.decoration(rect);
            cx.text(rect.x + 1, rect.y, &note, CellStyle { bg: None, ..style }, width.saturating_sub(2));
        }
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        if self.read_only {
            // Nothing here reaches the session: no key, no paste, no mouse report, no arrow keys
            // for the wheel. What is left is the widget's own scrolling back, which is what a
            // screen kept on display is for, and it changes nothing but how much of it is shown.
            // Everything else is handed back and takes the usual route to the application.
            return match event {
                Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) => {
                    scroll_back(cx, mouse.kind == MouseKind::ScrollUp);
                    true
                }
                _ => false,
            };
        }
        match event {
            Event::Key(key) => {
                let shift = Modifiers { shift: true, ..Modifiers::default() };
                let ctrl = Modifiers { ctrl: true, ..Modifiers::default() };
                if (key.chord.key == Key::Tab && key.chord.mods == shift)
                    || (key.chord.key == Key::Char('q') && key.chord.mods == ctrl)
                {
                    return false;
                }
                if self.passes(cx.env().keymap(), key.chord) {
                    return false;
                }
                if self.session.exit().is_some() {
                    return false;
                }
                let application_cursor = self.session.parser().screen().application_cursor();
                let bytes = key_bytes(key, application_cursor);
                if bytes.is_empty() {
                    return false;
                }
                cx.memory::<TerminalMemory>().scrollback = 0;
                // The branch above already hands a key back when the program has ended, which is
                // the case that loses keys. What is left here is the race of a program ending
                // between that check and this write, one key wide; handing that one back would
                // reach whatever is behind the terminal and do something else with it, which is
                // worse than a key nobody heard.
                let _ = self.session.write(&bytes);
                true
            }
            Event::Paste(text) => {
                // The session decides how a paste is sent, so an application pasting into a
                // program and a person pasting into this widget take exactly the same path.
                //
                // The error is carried up: a paste the program never received is handed back, as
                // a key is when the program has ended, and then reaches
                // [`App::clipboard`](crate::runtime::App::clipboard) as
                // `ClipboardEvent::Pasted`, where the application can say what happened. Text a
                // person meant to send is the one thing that must not disappear without a word.
                if self.session.paste_typed(text).is_err() {
                    return false;
                }
                cx.memory::<TerminalMemory>().scrollback = 0;
                true
            }
            Event::Mouse(mouse) if let Some(used) = terminal_mouse::event(&self.session, cx, mouse) => {
                if used {
                    cx.memory::<TerminalMemory>().scrollback = 0;
                }
                used
            }
            Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) => {
                let up = mouse.kind == MouseKind::ScrollUp;
                let (alternate, application_cursor) = {
                    let parser = self.session.parser();
                    (parser.screen().alternate_screen(), parser.screen().application_cursor())
                };
                if alternate {
                    let key =
                        KeyEvent::from_chord(crate::keymap::KeyChord::plain(if up { Key::Up } else { Key::Down }));
                    // The error is carried up, as for a key. On the alternate screen the wheel is
                    // arrow keys and there is no scrollback to fall back on, so a step a program
                    // that has ended never hears must not be reported as used: handed back it
                    // reaches whatever holds the terminal, which can scroll instead.
                    if self.session.write(&key_bytes(&key, application_cursor).repeat(WHEEL_LINES)).is_err() {
                        return false;
                    }
                    return true;
                }
                scroll_back(cx, up);
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        // A view-only terminal is left out of the focus order, which also keeps a click from
        // focusing it: focus lands on the nearest focusable ancestor instead, so a window whose
        // program has ended keeps its keys on the buttons that can still do something.
        !self.read_only
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::TerminalEvent;

    fn press(chord: &str) -> KeyEvent {
        KeyEvent::press(chord)
    }

    #[test]
    fn keys_encode_like_xterm() {
        assert_eq!(key_bytes(&press("a"), false), b"a");
        assert_eq!(key_bytes(&press("shift+a"), false), b"A");
        assert_eq!(key_bytes(&press("ctrl+c"), false), [3]);
        assert_eq!(key_bytes(&press("alt+b"), false), b"\x1bb");
        assert_eq!(key_bytes(&press("ctrl+space"), false), [0]);
        assert_eq!(key_bytes(&press("ctrl+alt+space"), false), b"\x1b\0");
        assert_eq!(key_bytes(&press("enter"), false), b"\r");
        assert_eq!(key_bytes(&press("backspace"), false), [0x7f]);
        assert_eq!(key_bytes(&press("up"), false), b"\x1b[A");
        assert_eq!(key_bytes(&press("up"), true), b"\x1bOA");
        assert_eq!(key_bytes(&press("ctrl+right"), false), b"\x1b[1;5C");
        assert_eq!(key_bytes(&press("pgdn"), false), b"\x1b[6~");
        assert_eq!(key_bytes(&press("f1"), false), b"\x1bOP");
        assert_eq!(key_bytes(&press("f12"), false), b"\x1b[24~");
    }

    fn chord(text: &str) -> KeyChord {
        text.parse().expect("valid chord")
    }

    #[test]
    fn only_passed_actions_pass_and_never_text_keys() {
        let mut keymap = Keymap::builtin();
        keymap.bind(Scope::Global, "help", &[chord("?"), chord("f1"), chord("shift+h"), chord("space")]);
        let session = TerminalSession::spawn("/bin/true".as_ref(), &[] as &[&str], Path::new("/")).expect("pty");
        let plain = Terminal::new(&session);
        assert!(!plain.passes(&keymap, chord("f1")), "by default nothing passes");
        assert!(!plain.passes(&keymap, chord("alt+b")));
        let terminal = plain.pass_through(Scope::Global, "help").pass_through(Scope::Global, "toggle-panel");
        assert!(terminal.passes(&keymap, chord("f1")));
        assert!(terminal.passes(&keymap, chord("alt+b")));
        for typed in ["?", "shift+h", "space"] {
            assert!(!terminal.passes(&keymap, chord(typed)), "{typed} types into the program");
        }
        assert!(!terminal.passes(&keymap, chord("ctrl+p")), "palette was not passed");
        keymap.bind(Scope::Global, "help", &[chord("alt+h")]);
        assert!(terminal.passes(&keymap, chord("alt+h")), "the keymap in force decides, not the one at build time");
        assert!(!terminal.passes(&keymap, chord("f1")));
    }

    #[derive(Clone, Debug, PartialEq)]
    enum PassMsg {
        Help,
        Panel(bool),
    }

    struct Passing {
        session: TerminalSession,
        open: bool,
        heard: Vec<PassMsg>,
    }

    impl App for Passing {
        type Msg = PassMsg;
        fn update(&mut self, msg: PassMsg) -> Command<PassMsg> {
            if let PassMsg::Panel(open) = msg {
                self.open = open;
            }
            self.heard.push(msg);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, PassMsg>) {
            let session = self.session.clone();
            crate::widgets::SidePanel::new(8)
                .open(self.open)
                .on_toggle(PassMsg::Panel)
                .panel(|ui| {
                    ui.add(crate::widgets::Text::new("files"));
                })
                .body(move |ui| {
                    ui.add(
                        Terminal::new(&session)
                            .pass_through(Scope::Global, "help")
                            .pass_through(Scope::Global, "toggle-panel"),
                    )
                    .fill()
                    .id("terminal");
                })
                .show(ui);
        }
        fn action(&self, name: &str) -> Option<PassMsg> {
            (name == "help").then_some(PassMsg::Help)
        }
    }

    /// Waits until the program's screen contains `text`.
    fn wait_for(session: &TerminalSession, text: &str) {
        let watch = session.watch();
        let started = Instant::now();
        while !session.parser().screen().contents().contains(text) {
            let _ = watch.next();
            assert!(started.elapsed() < Duration::from_secs(10), "{}", session.parser().screen().contents());
        }
    }

    #[test]
    fn passed_actions_reach_the_application_and_the_rest_reaches_the_program() {
        // The program shows, in hex, the first four bytes it receives.
        let script = "stty raw -echo; printf 'ready '; head -c 4 | od -An -tx1";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        wait_for(&session, "ready");
        let mut env = crate::env::Env::builtin();
        env.keymap_mut().bind(Scope::Global, "help", &[chord("?"), chord("f1")]);
        let app = Passing { session: session.clone(), open: true, heard: Vec::new() };
        let mut h = Harness::with_env(app, env, 40, 4);
        for _ in 0..4 {
            if h.is_focused("terminal") {
                break;
            }
            h.press("tab");
        }
        assert!(h.is_focused("terminal"));
        h.press("f1").press("alt+b");
        assert_eq!(h.app().heard, [PassMsg::Help, PassMsg::Panel(false)], "f1 and alt+b reached the application");
        assert!(h.is_focused("terminal"));
        h.press("?").type_text("abc");
        wait_for(&session, "3f 61 62 63");
        assert_eq!(h.app().heard.len(), 2, "? typed into the program instead of opening the help");
        session.kill();
    }

    #[test]
    fn a_person_pasting_is_bracketed_and_counts_as_their_own_input() {
        // Shows what it is given as it arrives, escapes visible, after asking for the marks.
        let script = "stty -echo -icanon min 1 time 0; printf '\\033[?2004hready'; cat -v";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        wait_for(&session, "ready");
        let app = Passing { session: session.clone(), open: false, heard: Vec::new() };
        let mut h = Harness::new(app, 40, 4);
        for _ in 0..4 {
            if h.is_focused("terminal") {
                break;
            }
            h.press("tab");
        }
        assert!(h.is_focused("terminal"));
        let before = session.last_input();
        h.paste("one\ntwo");
        wait_for(&session, "^[[201~");
        let shown = session.parser().screen().contents();
        assert!(shown.contains("^[[200~one"), "{shown:?}");
        assert!(session.last_input() > before, "the person is at the keyboard here, unlike an application's paste");
        session.kill();
    }

    #[derive(Clone, Debug, PartialEq)]
    enum ToggleMsg {
        /// The key came from inside the terminal.
        Leave,
        /// The key came from anywhere else.
        Enter,
    }

    struct Toggling {
        session: TerminalSession,
        heard: Vec<ToggleMsg>,
    }

    impl App for Toggling {
        type Msg = ToggleMsg;
        fn update(&mut self, msg: ToggleMsg) -> Command<ToggleMsg> {
            let target = if msg == ToggleMsg::Leave { "tabs" } else { "terminal" };
            self.heard.push(msg);
            Command::focus(target)
        }
        fn view(&self, ui: &mut View<'_, ToggleMsg>) {
            ui.add(crate::widgets::Button::new("tabs").on_press(ToggleMsg::Enter)).id("tabs");
            ui.add(Terminal::new(&self.session).pass_through(Scope::App, "focus-toggle"))
                .fill()
                .id("terminal")
                .on_action(Scope::App, "focus-toggle", ToggleMsg::Leave);
        }
        fn action(&self, name: &str) -> Option<ToggleMsg> {
            (name == "focus-toggle").then_some(ToggleMsg::Enter)
        }
    }

    #[test]
    fn one_key_leaves_the_terminal_and_the_same_key_goes_back() {
        // The program shows, in hex, the first byte it receives.
        let script = "stty raw -echo; printf 'ready '; head -c 1 | od -An -tx1";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        wait_for(&session, "ready");
        let mut env = crate::env::Env::builtin();
        env.keymap_mut().bind(Scope::App, "focus-toggle", &[chord("ctrl+alt+space")]);
        let mut h = Harness::with_env(Toggling { session: session.clone(), heard: Vec::new() }, env, 40, 5);
        h.press("tab").press("tab");
        assert!(h.is_focused("terminal"));
        h.press("ctrl+alt+space");
        assert!(h.is_focused("tabs"), "the key left the terminal");
        h.press("ctrl+alt+space");
        assert!(h.is_focused("terminal"), "the same key went back in");
        assert_eq!(h.app().heard, [ToggleMsg::Leave, ToggleMsg::Enter]);
        h.type_text("a");
        wait_for(&session, "61");
        assert!(!session.parser().screen().contents().contains("00"), "the chord never reached the program");
        session.kill();
    }

    #[derive(Clone, Debug, PartialEq)]
    enum PasteMsg {
        /// Pasted text no widget took, offered to the application.
        Loose(String),
    }

    struct Pasting {
        session: TerminalSession,
        heard: Vec<PasteMsg>,
    }

    impl App for Pasting {
        type Msg = PasteMsg;
        fn update(&mut self, msg: PasteMsg) -> Command<PasteMsg> {
            self.heard.push(msg);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, PasteMsg>) {
            ui.add(Terminal::new(&self.session)).fill().id("terminal");
        }
        fn clipboard(&self, event: &crate::runtime::ClipboardEvent) -> Option<PasteMsg> {
            match event {
                crate::runtime::ClipboardEvent::Pasted(text) => Some(PasteMsg::Loose(text.clone())),
                crate::runtime::ClipboardEvent::Copied(_) => None,
            }
        }
    }

    #[test]
    fn a_paste_into_a_program_that_has_ended_is_handed_back_instead_of_disappearing() {
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", "printf done"], Path::new("/")).expect("pty");
        assert_eq!(run_to_exit(&session), Some(0));
        let mut h = Harness::new(Pasting { session, heard: Vec::new() }, 40, 4);
        h.press("tab");
        assert!(h.is_focused("terminal"));
        h.paste("a message");
        assert_eq!(
            h.app().heard,
            [PasteMsg::Loose("a message".to_owned())],
            "the application heard the text instead of losing it"
        );
    }

    #[test]
    fn a_paste_into_a_running_program_stays_in_the_terminal() {
        let script = "stty -echo -icanon min 1 time 0; printf ready; cat -v";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        wait_for(&session, "ready");
        let mut h = Harness::new(Pasting { session: session.clone(), heard: Vec::new() }, 40, 4);
        h.press("tab");
        h.paste("typed");
        wait_for(&session, "typed");
        assert!(h.app().heard.is_empty(), "the widget took the paste, so the application was not asked");
        session.kill();
    }

    struct Wheeling {
        session: TerminalSession,
    }

    impl App for Wheeling {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add_with(crate::widgets::ScrollView::new(), |ui| {
                ui.add(Terminal::new(&self.session))
                    .width(crate::widget::Length::Fill(1))
                    .height(crate::widget::Length::Cells(2))
                    .id("terminal");
                for i in 0..20 {
                    ui.add(crate::widgets::Text::new(format!("line {i}")));
                }
            })
            .fill();
        }
    }

    #[test]
    fn the_wheel_on_a_dead_alternate_screen_is_handed_back_and_a_live_one_keeps_it() {
        // A full-screen program that ends while its alternate screen is still on, which is what a
        // killed editor leaves behind.
        let dead =
            TerminalSession::spawn("/bin/sh".as_ref(), &["-c", "printf '\\033[?1049hfull screen'"], Path::new("/"))
                .expect("pty");
        assert_eq!(run_to_exit(&dead), Some(0));
        let mut h = Harness::new(Wheeling { session: dead }, 20, 6);
        assert!(h.screen().starts_with("full screen"), "{}", h.screen());
        h.mouse(MouseKind::ScrollDown, 2, 0);
        assert!(!h.screen().starts_with("full screen"), "the scroll view took the wheel: {}", h.screen());

        // The same wheel over a program that is still there stays in the terminal.
        let script = "stty raw -echo; printf '\\033[?1049hready '; cat -v";
        let live = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        wait_for(&live, "ready");
        let mut h = Harness::new(Wheeling { session: live.clone() }, 20, 6);
        h.mouse(MouseKind::ScrollDown, 2, 0);
        wait_for(&live, "^[[B");
        h.render();
        assert!(h.screen().starts_with("ready"), "the terminal kept the wheel: {}", h.screen());
        live.kill();
    }

    struct Viewing {
        session: TerminalSession,
        heard: Vec<PasteMsg>,
    }

    impl App for Viewing {
        type Msg = PasteMsg;
        fn update(&mut self, msg: PasteMsg) -> Command<PasteMsg> {
            self.heard.push(msg);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, PasteMsg>) {
            ui.add(crate::widgets::Button::new("close").on_press(PasteMsg::Loose("pressed".to_owned()))).id("close");
            ui.add(Terminal::new(&self.session).read_only()).fill().id("screen");
        }
        fn clipboard(&self, event: &crate::runtime::ClipboardEvent) -> Option<PasteMsg> {
            match event {
                crate::runtime::ClipboardEvent::Pasted(text) => Some(PasteMsg::Loose(text.clone())),
                crate::runtime::ClipboardEvent::Copied(_) => None,
            }
        }
    }

    #[test]
    fn a_view_only_terminal_sends_the_program_nothing() {
        // Echoes what it reads, escapes visible, and asks for every mouse motion, so a report the
        // widget might send would show up in its own output.
        let script = "stty raw -echo; printf '\\033[?1003h\\033[?1006hready '; cat -v";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        wait_for(&session, "ready");
        let mut h = Harness::new(Viewing { session: session.clone(), heard: Vec::new() }, 40, 6);
        h.press("tab").type_text("typed");
        h.paste("pasted text");
        h.click(5, 3).hover(6, 3).mouse(MouseKind::ScrollUp, 5, 3).mouse(MouseKind::ScrollDown, 5, 3);
        // Ask the program itself: what arrives after everything above is the only thing written.
        session.write(b"end").expect("write");
        wait_for(&session, "end");
        let shown = session.parser().screen().contents();
        assert!(!shown.contains("typed"), "a key reached the program: {shown:?}");
        assert!(!shown.contains("pasted"), "a paste reached the program: {shown:?}");
        assert!(!shown.contains("^["), "an escape sequence reached the program: {shown:?}");
        assert_eq!(
            h.app().heard,
            [PasteMsg::Loose("pasted text".to_owned())],
            "the paste was handed back to the application"
        );
        session.kill();
    }

    #[test]
    fn a_view_only_terminal_is_no_tab_stop_and_a_click_does_not_focus_it() {
        let session =
            TerminalSession::spawn("/bin/sh".as_ref(), &["-c", "printf screen"], Path::new("/")).expect("pty");
        assert_eq!(run_to_exit(&session), Some(0));
        let mut h = Harness::new(Viewing { session, heard: Vec::new() }, 40, 6);
        h.press("tab");
        assert!(h.is_focused("close"));
        h.press("tab");
        assert!(h.is_focused("close"), "Tab has nowhere else to go");
        h.click(5, 3);
        assert!(h.is_focused("close"), "the click did not move focus into the screen");
    }

    #[test]
    fn a_view_only_screen_keeps_its_colours_and_cursor_and_shows_them_faint() {
        // Red, a wide character, and the cursor left where the program stopped.
        let script = "printf '\\033[31mred\\033[0m 世界'";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        assert_eq!(run_to_exit(&session), Some(0));
        let live = Harness::new(Demo { session: session.clone() }, 40, 4);
        let mut faint = Harness::new(Viewing { session, heard: Vec::new() }, 40, 4);
        faint.render();
        let surface = live.env().theme().color("surface").expect("the theme has a surface colour");
        let screen = faint.screen();
        assert!(screen.contains("red 世界"), "{screen}");
        assert!(screen.contains("exited with 0"), "the note still says what happened: {screen}");
        // The view-only screen is one row lower: the button above it takes the first row.
        let dimmed = |color: Option<Rgb>| color.map(|color| color.mix(surface, FAINT));
        assert_eq!(live.fg(0, 0), live.env().theme().color("danger"));
        assert_eq!(faint.fg(0, 1), dimmed(live.fg(0, 0)), "classic red is drawn mixed into the background");
        // The cursor sits after "red " and the two wide characters.
        assert_ne!(live.bg(8, 0), live.bg(20, 0), "the cursor the program left is drawn");
        assert_eq!(faint.bg(8, 1), dimmed(live.bg(8, 0)), "and it fades with the screen");
    }

    #[test]
    fn a_view_only_terminal_scrolls_back_through_what_the_program_wrote() {
        let script = "i=1; while [ $i -le 40 ]; do echo line$i; i=$((i+1)); done";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        assert_eq!(run_to_exit(&session), Some(0));
        let mut h = Harness::new(Viewing { session, heard: Vec::new() }, 20, 6);
        let before = h.screen();
        assert!(before.contains("line18"), "the last screen of a 24-row program: {before}");
        h.mouse(MouseKind::ScrollUp, 5, 3);
        let back = h.screen();
        assert!(back.contains("line15"), "the wheel went back through the earlier output: {back}");
        assert!(!back.contains("line21"), "three lines of the live screen made way for them: {back}");
        h.mouse(MouseKind::ScrollDown, 5, 3);
        assert_eq!(h.screen(), before, "and forward again to where it was");
    }

    #[test]
    fn palette_maps_classic_colours_to_the_theme_and_the_rest_to_xterm() {
        assert_eq!(xterm(16), Rgb::new(0, 0, 0));
        assert_eq!(xterm(196), Rgb::new(255, 0, 0));
        assert_eq!(xterm(244), Rgb::new(128, 128, 128));
    }

    struct Demo {
        session: TerminalSession,
    }

    impl App for Demo {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Terminal::new(&self.session)).fill().id("terminal");
        }
    }

    /// Waits until the program ends, the way an application's background command would.
    fn run_to_exit(session: &TerminalSession) -> Option<u32> {
        let watch = session.watch();
        let started = Instant::now();
        loop {
            if let TerminalEvent::Exited(code) = watch.next() {
                return code;
            }
            assert!(started.elapsed() < Duration::from_secs(10), "the program did not finish");
        }
    }

    #[test]
    fn runs_a_program_in_a_pty_and_draws_its_colours() {
        let script = "printf '\\033[31mred\\033[0m and \\033[1mbold\\033[0m'; exit 3";
        let session = TerminalSession::spawn("/bin/sh".as_ref(), &["-c", script], Path::new("/")).expect("pty");
        assert_eq!(run_to_exit(&session), Some(3));
        let mut h = Harness::new(Demo { session }, 40, 4);
        let screen = h.screen();
        assert!(screen.starts_with("red and bold"), "{screen}");
        assert!(screen.contains("exited with 3"), "{screen}");
        let theme = h.env().theme();
        assert_eq!(h.fg(0, 0), theme.color("danger"), "classic red is the theme's danger colour");
        assert!(h.is_bold(8, 0));
        assert_eq!(h.bg(20, 0), theme.color("surface"));
        h.press("tab");
        assert!(!h.press("x").screen().is_empty());
    }

    #[test]
    fn typing_reaches_the_program_and_resizes_apply() {
        let session = TerminalSession::spawn("/bin/cat".as_ref(), &[] as &[&str], Path::new("/")).expect("pty");
        let watch = session.watch();
        let mut h = Harness::new(Demo { session: session.clone() }, 30, 5);
        h.press("tab").type_text("hello").press("enter");
        let started = Instant::now();
        // Wait through the watch at least once: the echo can arrive before the first call, and the
        // watch is what applies the size the widget asked for.
        loop {
            let _ = watch.next();
            if session.parser().screen().contents().contains("hello\nhello") {
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(10), "{}", session.parser().screen().contents());
        }
        h.render();
        assert!(h.screen().starts_with("hello\nhello"), "{}", h.screen());
        assert_eq!(session.parser().screen().size(), (5, 30), "the watch applied the widget's size");
        session.kill();
        assert!(matches!(run_to_exit(&session), Some(_) | None));
    }
}
