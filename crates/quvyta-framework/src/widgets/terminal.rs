//! An embedded terminal: draws a [`TerminalSession`] in theme colours and types into it.

use crate::color::Rgb;
use crate::event::{Event, KeyEvent, MouseKind};
use crate::geometry::{Rect, Size};
use crate::keymap::{Key, Modifiers};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};

use super::rows::WHEEL_ROWS;
use super::terminal_session::TerminalSession;

/// Lines one wheel step scrolls back.
const WHEEL_LINES: usize = WHEEL_ROWS as usize;

/// A terminal showing a [`TerminalSession`]: the program's screen drawn cell by cell, with the
/// sixteen classic colours taken from the theme so shells and tools match the application.
///
/// While focused every key goes to the program, including Tab and Esc, except `shift+tab`,
/// which moves focus on, and `ctrl+q`, which still quits. Pasted text is sent as a bracketed
/// paste when the program asks for it. The wheel scrolls back through earlier output (on the
/// alternate screen of full-screen programs it sends arrow keys instead); typing returns to the
/// live screen. The widget asks the session for its size; a running
/// [`TerminalWatch`](super::TerminalWatch) applies it.
///
/// The output is a text selection region: a mouse drag selects inside it (turn it off with
/// [`NodeMut::selectable`](crate::widget::NodeMut::selectable)); clean copies leave out the
/// scrolled-back and exited notes.
///
/// Style keys: `terminal` (`bg`, `fg`, the default colours), `terminal-cursor` (`bg`, `fg`)
/// with `focus`, `terminal-note` (`bg`, `fg`) for the scrolled-back and exited notes. Framework
/// strings: `quvyta.terminal.scrolled`, `quvyta.terminal.exited`.
pub struct Terminal {
    session: TerminalSession,
}

#[derive(Debug, Default)]
struct TerminalMemory {
    scrollback: usize,
}

impl Terminal {
    /// A terminal drawing `session`; cloning a session is cheap.
    #[must_use]
    pub fn new(session: &TerminalSession) -> Self {
        Self { session: session.clone() }
    }
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
        Key::Space if mods.ctrl => vec![0],
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
        // Only a request: the watch resizes the pseudo-terminal off the drawing thread.
        self.session.request_size(area.height, area.width);
        let focused = cx.is_focused();
        let base = cx.style("terminal", None, &[]).text();
        let default_bg = base.bg.unwrap_or_else(|| cx.color("surface"));
        let default_fg = base.fg.unwrap_or_else(|| cx.color("text"));
        cx.clear(area, default_bg);
        let cursor_states = if focused { vec![crate::theme::State::Focus] } else { Vec::new() };
        let cursor_style = cx.style("terminal-cursor", None, &cursor_states).text();
        let wanted = cx.memory::<TerminalMemory>().scrollback;

        let session = self.session.clone();
        let mut parser = session.parser();
        parser.screen_mut().set_scrollback(wanted);
        let screen = parser.screen();
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
                    fg = fg.mix(bg, 0.45);
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
        match event {
            Event::Key(key) => {
                let shift = Modifiers { shift: true, ..Modifiers::default() };
                let ctrl = Modifiers { ctrl: true, ..Modifiers::default() };
                if (key.chord.key == Key::Tab && key.chord.mods == shift)
                    || (key.chord.key == Key::Char('q') && key.chord.mods == ctrl)
                {
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
                let _ = self.session.write(&bytes);
                true
            }
            Event::Paste(text) => {
                let bracketed = self.session.parser().screen().bracketed_paste();
                let mut bytes = Vec::new();
                if bracketed {
                    bytes.extend_from_slice(b"\x1b[200~");
                }
                bytes.extend_from_slice(text.as_bytes());
                if bracketed {
                    bytes.extend_from_slice(b"\x1b[201~");
                }
                cx.memory::<TerminalMemory>().scrollback = 0;
                let _ = self.session.write(&bytes);
                true
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
                    let _ = self.session.write(&key_bytes(&key, application_cursor).repeat(WHEEL_LINES));
                    return true;
                }
                let memory = cx.memory::<TerminalMemory>();
                memory.scrollback =
                    if up { memory.scrollback + WHEEL_LINES } else { memory.scrollback.saturating_sub(WHEEL_LINES) };
                true
            }
            _ => false,
        }
    }

    fn focusable(&self) -> bool {
        true
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
        assert_eq!(key_bytes(&press("enter"), false), b"\r");
        assert_eq!(key_bytes(&press("backspace"), false), [0x7f]);
        assert_eq!(key_bytes(&press("up"), false), b"\x1b[A");
        assert_eq!(key_bytes(&press("up"), true), b"\x1bOA");
        assert_eq!(key_bytes(&press("ctrl+right"), false), b"\x1b[1;5C");
        assert_eq!(key_bytes(&press("pgdn"), false), b"\x1b[6~");
        assert_eq!(key_bytes(&press("f1"), false), b"\x1bOP");
        assert_eq!(key_bytes(&press("f12"), false), b"\x1b[24~");
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
