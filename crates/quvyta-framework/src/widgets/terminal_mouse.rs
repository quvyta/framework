//! Mouse reports for the program inside a [`Terminal`](super::Terminal): xterm's mouse modes
//! (9, 1000, 1002, 1003) and encodings (default, 1005 UTF-8, 1006 SGR).

use vt100::{MouseProtocolEncoding as Encoding, MouseProtocolMode as Mode, Screen};

use crate::event::{MouseButton, MouseEvent, MouseKind};
use crate::keymap::Modifiers;
use crate::widget::EventCx;

use super::terminal_session::TerminalSession;

/// What the pointer did, as the program hears it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Press(MouseButton),
    Release(MouseButton),
    Drag(MouseButton),
    Move,
    WheelUp,
    WheelDown,
}

/// The cell of the last report, so a captured pointer held outside the terminal (clamped onto
/// its edge) does not report the same cell again and again.
#[derive(Debug, Default)]
struct MouseMemory {
    last: Option<(u16, u16)>,
}

/// Whether the program wants moves with no button held (mode 1003), which the engine delivers
/// only to widgets that ask for them while painting.
pub(super) fn wants_moves(screen: &Screen) -> bool {
    screen.mouse_protocol_mode() == Mode::AnyMotion
}

/// Reports `mouse` to the program in the terminal. `None` when the program has not asked for the
/// mouse (or has ended), so the terminal keeps its own handling; otherwise whether the event was
/// used. A shift-held press is left to the engine so it starts a text selection, as in xterm.
pub(super) fn event<Msg>(session: &TerminalSession, cx: &mut EventCx<'_, Msg>, mouse: &MouseEvent) -> Option<bool> {
    if session.exit().is_some() {
        return None;
    }
    let (mode, encoding) = {
        let parser = session.parser();
        let screen = parser.screen();
        (screen.mouse_protocol_mode(), screen.mouse_protocol_encoding())
    };
    if mode == Mode::None {
        return None;
    }
    // Drags and the release belong to the program only after it took the press; otherwise they
    // are the tail of a selection or of a press that began elsewhere.
    let held = cx.interaction.pointer_capture == Some(cx.id());
    let action = match mouse.kind {
        // Shift is the way past the program to the terminal's own selection and scrollback.
        MouseKind::Down(_) | MouseKind::ScrollUp | MouseKind::ScrollDown if mouse.mods.shift => return None,
        MouseKind::Down(button) => Action::Press(button),
        MouseKind::Drag(button) if held => Action::Drag(button),
        MouseKind::Up(button) if held => Action::Release(button),
        MouseKind::Drag(_) | MouseKind::Up(_) => return None,
        MouseKind::Moved => Action::Move,
        MouseKind::ScrollUp => Action::WheelUp,
        MouseKind::ScrollDown => Action::WheelDown,
    };
    let area = cx.area();
    if area.width == 0 || area.height == 0 {
        return None;
    }
    // A captured pointer can be anywhere on screen; the program only knows its own cells.
    let clamp =
        |at: i32, start: i32, len: u16| u16::try_from((at - start).clamp(0, i32::from(len) - 1)).unwrap_or_default();
    let cell = (clamp(mouse.x, area.x, area.width), clamp(mouse.y, area.y, area.height));
    let memory = cx.memory::<MouseMemory>();
    let moved = memory.last != Some(cell);
    memory.last = Some(cell);
    if matches!(action, Action::Drag(_) | Action::Move) && !moved {
        return Some(true);
    }
    if let Some(bytes) = encode(mode, encoding, action, cell, mouse.mods) {
        let _ = session.write(&bytes);
    }
    if matches!(action, Action::Press(_)) {
        cx.capture_pointer();
    }
    // Whatever the mode leaves unreported is still the program's: a press in X10 mode must not
    // start a selection, nor its release click something else.
    Some(true)
}

/// The bytes that report `action` at the 0-based cell `(col, row)`, or `None` when `mode` does
/// not report it or `encoding` cannot carry the values.
fn encode(mode: Mode, encoding: Encoding, action: Action, cell: (u16, u16), mods: Modifiers) -> Option<Vec<u8>> {
    let reported = match action {
        Action::Press(_) | Action::WheelUp | Action::WheelDown => mode != Mode::None,
        Action::Release(_) => matches!(mode, Mode::PressRelease | Mode::ButtonMotion | Mode::AnyMotion),
        Action::Drag(_) => matches!(mode, Mode::ButtonMotion | Mode::AnyMotion),
        Action::Move => mode == Mode::AnyMotion,
    };
    if !reported {
        return None;
    }
    let number = |button: MouseButton| match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    };
    let mut code: u32 = match action {
        Action::Press(button) | Action::Drag(button) => number(button),
        // Only SGR can say which button came up; the older encodings have one release code.
        Action::Release(button) if encoding == Encoding::Sgr => number(button),
        Action::Release(_) | Action::Move => 3,
        Action::WheelUp => 64,
        Action::WheelDown => 65,
    };
    if matches!(action, Action::Drag(_) | Action::Move) {
        code += 32;
    }
    // X10 mode predates modifier reporting.
    if mode != Mode::Press {
        code += 4 * u32::from(mods.shift) + 8 * u32::from(mods.alt) + 16 * u32::from(mods.ctrl);
    }
    let (x, y) = (u32::from(cell.0) + 1, u32::from(cell.1) + 1);
    match encoding {
        Encoding::Sgr => {
            let end = if matches!(action, Action::Release(_)) { 'm' } else { 'M' };
            Some(format!("\x1b[<{code};{x};{y}{end}").into_bytes())
        }
        Encoding::Default => {
            let mut bytes = b"\x1b[M".to_vec();
            for value in [code, x, y] {
                bytes.push(u8::try_from(value + 32).ok()?);
            }
            Some(bytes)
        }
        Encoding::Utf8 => {
            let mut bytes = b"\x1b[M".to_vec();
            for value in [code, x, y] {
                // xterm stops at two UTF-8 bytes, so 2047 is the largest value it sends.
                let value = value + 32;
                if value > 2047 {
                    return None;
                }
                let mut buffer = [0; 4];
                bytes.extend_from_slice(char::from_u32(value)?.encode_utf8(&mut buffer).as_bytes());
            }
            Some(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::event::Event;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::Terminal;

    const NONE: Modifiers = Modifiers { shift: false, alt: false, ctrl: false };

    fn sgr(mode: Mode, action: Action, cell: (u16, u16), mods: Modifiers) -> Option<String> {
        encode(mode, Encoding::Sgr, action, cell, mods).map(|bytes| String::from_utf8(bytes).expect("ascii"))
    }

    #[test]
    fn no_mode_reports_nothing() {
        for encoding in [Encoding::Default, Encoding::Utf8, Encoding::Sgr] {
            assert_eq!(encode(Mode::None, encoding, Action::Press(MouseButton::Left), (0, 0), NONE), None);
        }
    }

    #[test]
    fn sgr_reports_buttons_releases_and_modifiers() {
        let left = MouseButton::Left;
        let mode = Mode::PressRelease;
        assert_eq!(sgr(mode, Action::Press(left), (5, 3), NONE).as_deref(), Some("\x1b[<0;6;4M"));
        assert_eq!(sgr(mode, Action::Press(MouseButton::Middle), (0, 0), NONE).as_deref(), Some("\x1b[<1;1;1M"));
        assert_eq!(sgr(mode, Action::Press(MouseButton::Right), (0, 0), NONE).as_deref(), Some("\x1b[<2;1;1M"));
        assert_eq!(
            sgr(mode, Action::Release(MouseButton::Right), (0, 0), NONE).as_deref(),
            Some("\x1b[<2;1;1m"),
            "an SGR release keeps its button"
        );
        let shift = Modifiers { shift: true, ..NONE };
        let alt = Modifiers { alt: true, ..NONE };
        let ctrl = Modifiers { ctrl: true, ..NONE };
        let all = Modifiers { shift: true, alt: true, ctrl: true };
        assert_eq!(sgr(mode, Action::Press(left), (0, 0), shift).as_deref(), Some("\x1b[<4;1;1M"));
        assert_eq!(sgr(mode, Action::Press(left), (0, 0), alt).as_deref(), Some("\x1b[<8;1;1M"));
        assert_eq!(sgr(mode, Action::Press(left), (0, 0), ctrl).as_deref(), Some("\x1b[<16;1;1M"));
        assert_eq!(sgr(mode, Action::Press(left), (0, 0), all).as_deref(), Some("\x1b[<28;1;1M"));
        assert_eq!(sgr(mode, Action::Press(left), (300, 400), NONE).as_deref(), Some("\x1b[<0;301;401M"));
    }

    #[test]
    fn the_wheel_is_buttons_64_and_65() {
        assert_eq!(sgr(Mode::PressRelease, Action::WheelUp, (1, 1), NONE).as_deref(), Some("\x1b[<64;2;2M"));
        assert_eq!(sgr(Mode::PressRelease, Action::WheelDown, (1, 1), NONE).as_deref(), Some("\x1b[<65;2;2M"));
        assert_eq!(
            encode(Mode::PressRelease, Encoding::Default, Action::WheelUp, (0, 0), NONE),
            Some(b"\x1b[M`!!".to_vec())
        );
    }

    #[test]
    fn each_mode_reports_only_its_events() {
        let left = MouseButton::Left;
        let all = [
            Action::Press(left),
            Action::Release(left),
            Action::Drag(left),
            Action::Move,
            Action::WheelUp,
            Action::WheelDown,
        ];
        let reported = |mode: Mode| -> Vec<Action> {
            all.into_iter().filter(|action| sgr(mode, *action, (0, 0), NONE).is_some()).collect()
        };
        assert_eq!(reported(Mode::Press), [Action::Press(left), Action::WheelUp, Action::WheelDown]);
        assert_eq!(
            reported(Mode::PressRelease),
            [Action::Press(left), Action::Release(left), Action::WheelUp, Action::WheelDown]
        );
        assert_eq!(
            reported(Mode::ButtonMotion),
            [Action::Press(left), Action::Release(left), Action::Drag(left), Action::WheelUp, Action::WheelDown]
        );
        assert_eq!(reported(Mode::AnyMotion), all);
    }

    #[test]
    fn motion_adds_32_and_a_plain_move_is_button_3() {
        let mode = Mode::AnyMotion;
        assert_eq!(sgr(mode, Action::Drag(MouseButton::Left), (2, 0), NONE).as_deref(), Some("\x1b[<32;3;1M"));
        assert_eq!(sgr(mode, Action::Drag(MouseButton::Right), (2, 0), NONE).as_deref(), Some("\x1b[<34;3;1M"));
        assert_eq!(sgr(mode, Action::Move, (2, 0), NONE).as_deref(), Some("\x1b[<35;3;1M"));
        let ctrl = Modifiers { ctrl: true, ..NONE };
        assert_eq!(sgr(mode, Action::Move, (2, 0), ctrl).as_deref(), Some("\x1b[<51;3;1M"));
    }

    #[test]
    fn x10_reports_presses_without_modifiers() {
        let all = Modifiers { shift: true, alt: true, ctrl: true };
        assert_eq!(sgr(Mode::Press, Action::Press(MouseButton::Right), (0, 0), all).as_deref(), Some("\x1b[<2;1;1M"));
        assert_eq!(
            encode(Mode::Press, Encoding::Default, Action::Press(MouseButton::Left), (0, 0), all),
            Some(b"\x1b[M !!".to_vec())
        );
    }

    #[test]
    fn the_default_encoding_adds_32_and_releases_as_button_3() {
        let mode = Mode::ButtonMotion;
        let left = MouseButton::Left;
        assert_eq!(encode(mode, Encoding::Default, Action::Press(left), (5, 3), NONE), Some(b"\x1b[M &$".to_vec()));
        assert_eq!(
            encode(mode, Encoding::Default, Action::Release(MouseButton::Right), (5, 3), NONE),
            Some(b"\x1b[M#&$".to_vec())
        );
        let shift = Modifiers { shift: true, ..NONE };
        assert_eq!(
            encode(mode, Encoding::Default, Action::Release(left), (0, 0), shift),
            Some(vec![0x1b, b'[', b'M', 32 + 3 + 4, 33, 33]),
            "a legacy release keeps its modifiers"
        );
        assert_eq!(encode(mode, Encoding::Default, Action::Drag(left), (0, 0), NONE), Some(b"\x1b[M@!!".to_vec()));
    }

    #[test]
    fn the_default_encoding_drops_cells_past_223() {
        let press = Action::Press(MouseButton::Left);
        assert_eq!(
            encode(Mode::PressRelease, Encoding::Default, press, (222, 222), NONE),
            Some(vec![0x1b, b'[', b'M', 32, 255, 255])
        );
        assert_eq!(encode(Mode::PressRelease, Encoding::Default, press, (223, 0), NONE), None);
        assert_eq!(encode(Mode::PressRelease, Encoding::Default, press, (0, 223), NONE), None);
    }

    #[test]
    fn utf8_encodes_large_cells_as_characters() {
        let press = Action::Press(MouseButton::Left);
        assert_eq!(encode(Mode::PressRelease, Encoding::Utf8, press, (5, 3), NONE), Some(b"\x1b[M &$".to_vec()));
        let mut wide = b"\x1b[M ".to_vec();
        wide.extend_from_slice("\u{12c}".as_bytes());
        wide.push(b'!');
        assert_eq!(encode(Mode::PressRelease, Encoding::Utf8, press, (267, 0), NONE), Some(wide), "300 is two bytes");
        assert_eq!(
            encode(Mode::PressRelease, Encoding::Utf8, Action::Release(MouseButton::Middle), (0, 0), NONE),
            Some(b"\x1b[M#!!".to_vec())
        );
        assert_eq!(
            encode(Mode::PressRelease, Encoding::Utf8, press, (2015, 0), NONE),
            None,
            "past 2015 cannot be sent"
        );
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

    /// Starts `script` under `sh` in raw mode, echoing what it reads visibly.
    fn start(script: &str) -> TerminalSession {
        let script = format!("stty raw -echo; {script}; cat -v");
        TerminalSession::spawn("/bin/sh".as_ref(), &["-c", &script], Path::new("/")).expect("pty")
    }

    /// Drives the session's watch until `done` holds for its screen.
    fn wait(session: &TerminalSession, done: impl Fn(&Screen) -> bool) {
        let watch = session.watch();
        // The watch blocks until output comes; ending the program after a while turns a report
        // that never arrives into a failure instead of a test that never finishes.
        let timer = session.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(10));
            timer.kill();
        });
        let started = Instant::now();
        while !done(session.parser().screen()) {
            let _ = watch.next();
            assert!(started.elapsed() < Duration::from_secs(10), "{}", session.parser().screen().contents());
        }
    }

    /// A program with SGR mouse mode `mode` on, and a harness showing it.
    fn with_mouse(mode: &str, expected: Mode) -> (TerminalSession, Harness<Demo>) {
        let session = start(&format!("printf '\\033[?{mode}h\\033[?1006h'"));
        wait(&session, |screen| screen.mouse_protocol_mode() == expected);
        let h = Harness::new(Demo { session: session.clone() }, 40, 8);
        (session, h)
    }

    fn contains(text: &str) -> impl Fn(&Screen) -> bool + '_ {
        move |screen| screen.contents().contains(text)
    }

    fn shift_mouse(kind: MouseKind, x: i32, y: i32) -> Event {
        Event::Mouse(MouseEvent { kind, x, y, mods: Modifiers { shift: true, ..NONE } })
    }

    #[test]
    fn a_click_reaches_a_program_that_asked_for_the_mouse() {
        let (session, mut h) = with_mouse("1000", Mode::PressRelease);
        let plain = h.bg(5, 3);
        h.click(5, 3);
        wait(&session, contains("^[[<0;6;4M^[[<0;6;4m"));
        h.render();
        assert_eq!(h.bg(5, 3), plain, "the press was the program's, not a selection");
        session.kill();
    }

    #[test]
    fn a_drag_keeps_reporting_outside_the_terminal_area() {
        let (session, mut h) = with_mouse("1002", Mode::ButtonMotion);
        h.mouse(MouseKind::Down(MouseButton::Left), 1, 1)
            .mouse(MouseKind::Drag(MouseButton::Left), 3, 1)
            .mouse(MouseKind::Drag(MouseButton::Left), 90, -4)
            .mouse(MouseKind::Drag(MouseButton::Left), 95, -9)
            .mouse(MouseKind::Up(MouseButton::Left), 95, -9);
        wait(&session, contains("^[[<0;2;2M^[[<32;4;2M^[[<32;40;1M^[[<0;40;1m"));
        session.kill();
    }

    #[test]
    fn plain_moves_reach_the_program_in_any_motion_mode() {
        let (session, mut h) = with_mouse("1003", Mode::AnyMotion);
        h.hover(2, 2).hover(4, 2);
        wait(&session, contains("^[[<35;3;3M^[[<35;5;3M"));
        session.kill();
    }

    #[test]
    fn the_wheel_goes_to_the_program_as_buttons_64_and_65() {
        let (session, mut h) = with_mouse("1000", Mode::PressRelease);
        h.mouse(MouseKind::ScrollUp, 0, 0).mouse(MouseKind::ScrollDown, 1, 0);
        wait(&session, contains("^[[<64;1;1M^[[<65;2;1M"));
        session.kill();
    }

    #[test]
    fn shift_drag_selects_while_the_program_has_the_mouse() {
        let session = start("printf 'hello world'; printf '\\033[?1002h\\033[?1006h'");
        wait(&session, |screen| screen.mouse_protocol_mode() == Mode::ButtonMotion);
        let mut h = Harness::new(Demo { session: session.clone() }, 40, 8);
        let plain = h.bg(2, 0);
        h.events(&[
            shift_mouse(MouseKind::Down(MouseButton::Left), 0, 0),
            shift_mouse(MouseKind::Drag(MouseButton::Left), 4, 0),
            shift_mouse(MouseKind::Up(MouseButton::Left), 4, 0),
        ]);
        assert_ne!(h.bg(2, 0), plain, "shift keeps the terminal's selection");
        session.write(b"end").expect("write");
        wait(&session, contains("end"));
        assert!(!session.parser().screen().contents().contains("^["), "the program heard nothing of the drag");
        session.kill();
    }

    #[test]
    fn without_a_mouse_mode_a_drag_selects() {
        let session = start("printf 'hello world'");
        wait(&session, contains("hello world"));
        let mut h = Harness::new(Demo { session: session.clone() }, 40, 8);
        let plain = h.bg(2, 0);
        h.drag((0, 0), (4, 0));
        assert_ne!(h.bg(2, 0), plain, "the drag selects as before");
        session.kill();
    }
}
