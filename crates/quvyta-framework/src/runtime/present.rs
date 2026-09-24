//! Putting a frame on the terminal, and nothing at all when the frame is the one already shown.
//!
//! Every message the application handles asks for a frame, and most of them change nothing on
//! screen: a clock that ticks each second but shows minutes, a reading that stayed the same.
//! Over SSH each frame written is bytes on the wire even when no cell differs (the synchronized
//! update markers, the cursor, the colour reset), so a frame equal to the one on screen is not
//! written.
//!
//! The pointer's shape travels with the frame but outside the cells: a widget asks for a resize
//! arrow over an edge while painting, and the frame writes OSC 22 only when the shape under the
//! pointer changed, only to a terminal known to understand it. A frame that changed no cell and
//! no shape still writes nothing.

use std::io::{self, Write};

use crossterm::execute;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::terminal::Terminal;
use ratatui_crossterm::CrosstermBackend;

use crate::widget::PointerShape;

/// The variable that turns pointer shapes on or off whatever the terminal is: `on` or `off`.
const POINTER_SHAPES_VAR: &str = "QUVYTA_POINTER_SHAPES";

/// The terminal an application draws on, and the frame it shows.
pub(crate) struct Screen<W: Write> {
    terminal: Terminal<CrosstermBackend<W>>,
    /// The frame on the terminal now; `None` when the terminal's contents are not known, as after
    /// a program was handed the screen, so the next frame is written whole.
    shown: Option<Buffer>,
    /// Whether the terminal is told the pointer's shape; see [`pointer_shapes_supported`].
    pointer_shapes: bool,
    /// The pointer's shape on the terminal now. A terminal starts with its usual pointer.
    shape: PointerShape,
}

impl<W: Write> Screen<W> {
    pub(crate) fn new(terminal: Terminal<CrosstermBackend<W>>) -> Self {
        Self { terminal, shown: None, pointer_shapes: false, shape: PointerShape::Default }
    }

    /// Tells the terminal the pointer's shape, for a terminal that understands OSC 22.
    pub(crate) fn pointer_shapes(mut self, on: bool) -> Self {
        self.pointer_shapes = on;
        self
    }

    /// Paints a frame with `render`, which answers the pointer shape the frame asks for, and
    /// writes what changed inside one synchronized update: the cells, and the shape when it is
    /// not the one the terminal shows. Returns whether anything was written: a frame equal to
    /// the one shown, with the same shape, writes nothing.
    pub(crate) fn present(&mut self, render: impl FnOnce(&mut Buffer) -> PointerShape) -> io::Result<bool> {
        self.terminal.autoresize()?;
        let mut frame = self.terminal.get_frame();
        let shape = render(frame.buffer_mut());
        let reshape = Some(shape).filter(|shape| self.pointer_shapes && *shape != self.shape);
        let painted = self.terminal.current_buffer_mut();
        if self.shown.as_ref() == Some(&*painted) {
            // The buffer painted this time is reused by the next frame, which paints every cell
            // again, so it is left as it is and no cell reaches the terminal.
            let Some(shape) = reshape else {
                return Ok(false);
            };
            self.write_shape(shape)?;
            self.terminal.backend_mut().flush()?;
            return Ok(true);
        }
        let painted = painted.clone();
        execute!(self.terminal.backend_mut(), BeginSynchronizedUpdate)?;
        self.terminal.apply_buffer_with_cursor(None)?;
        if let Some(shape) = reshape {
            self.write_shape(shape)?;
        }
        execute!(self.terminal.backend_mut(), EndSynchronizedUpdate)?;
        self.shown = Some(painted);
        Ok(true)
    }

    /// Gives the pointer its usual shape back, for before the terminal is handed to a program
    /// or left: the shell after it should not keep a resize arrow.
    pub(crate) fn reset_pointer_shape(&mut self) -> io::Result<()> {
        if self.pointer_shapes && self.shape != PointerShape::Default {
            self.write_shape(PointerShape::Default)?;
            self.terminal.backend_mut().flush()?;
        }
        Ok(())
    }

    /// Writes OSC 22 for `shape`, unflushed, and remembers it as the one shown.
    fn write_shape(&mut self, shape: PointerShape) -> io::Result<()> {
        write!(self.terminal.backend_mut(), "\x1b]22;{}\x1b\\", shape.name())?;
        self.shape = shape;
        Ok(())
    }

    /// The size the terminal reports now.
    pub(crate) fn size(&self) -> io::Result<Rect> {
        Ok(self.terminal.size()?.into())
    }

    /// Forgets what is on screen and takes the size `area`, so the next frame draws every cell:
    /// for after a program had the terminal and wrote over it.
    pub(crate) fn redraw_all(&mut self, area: Rect) -> io::Result<()> {
        self.terminal.resize(area)?;
        self.shown = None;
        Ok(())
    }

    /// The terminal itself, for what does not go through frames.
    pub(crate) fn into_terminal(self) -> Terminal<CrosstermBackend<W>> {
        self.terminal
    }
}

/// Whether the terminal the application runs in changes the pointer's shape on OSC 22, told by
/// `env`, which looks a variable up (`|name| std::env::var(name).ok()` in the runtime, a fixed
/// map in tests).
///
/// `QUVYTA_POINTER_SHAPES=on` or `off` decides outright. Otherwise only terminals known to
/// understand it answer yes: kitty (`TERM=xterm-kitty`), foot (`TERM=foot` or `foot-extra`) and
/// WezTerm (`TERM_PROGRAM=WezTerm`). Inside tmux or screen the answer is no, since the
/// multiplexer does not pass the sequence on. An unknown terminal is sent nothing: a sequence it
/// does not know could show up as text.
pub(crate) fn pointer_shapes_supported(env: impl Fn(&str) -> Option<String>) -> bool {
    let lower = |name: &str| env(name).map(|value| value.trim().to_lowercase());
    match lower(POINTER_SHAPES_VAR).as_deref() {
        Some("on") => return true,
        Some("off") => return false,
        _ => {}
    }
    if env("TMUX").is_some() || env("STY").is_some() {
        return false;
    }
    let term = lower("TERM").unwrap_or_default();
    let program = lower("TERM_PROGRAM").unwrap_or_default();
    term == "xterm-kitty" || term == "foot" || term == "foot-extra" || program == "wezterm" || program == "kitty"
}

#[cfg(test)]
mod tests {
    use ratatui_core::terminal::{TerminalOptions, Viewport};

    use super::*;

    /// What a screen wrote, kept outside it so a test can read it.
    #[derive(Clone, Default)]
    struct Wire(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl Write for Wire {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn screen() -> (Screen<Wire>, Wire) {
        let wire = Wire::default();
        let options = TerminalOptions { viewport: Viewport::Fixed(Rect::new(0, 0, 12, 2)) };
        let terminal = Terminal::with_options(CrosstermBackend::new(wire.clone()), options).expect("a terminal");
        (Screen::new(terminal), wire)
    }

    fn paint(text: &'static str) -> impl FnOnce(&mut Buffer) -> PointerShape {
        pointing(text, PointerShape::Default)
    }

    /// Paints `text` and asks for `shape`.
    fn pointing(text: &'static str, shape: PointerShape) -> impl FnOnce(&mut Buffer) -> PointerShape {
        move |buffer: &mut Buffer| {
            buffer.reset();
            buffer.set_string(0, 0, text, ratatui_core::style::Style::default());
            shape
        }
    }

    /// Bytes written by one frame.
    fn bytes(screen: &mut Screen<Wire>, wire: &Wire, render: impl FnOnce(&mut Buffer) -> PointerShape) -> usize {
        written(screen, wire, render).len()
    }

    /// What one frame wrote, as text.
    fn written(screen: &mut Screen<Wire>, wire: &Wire, render: impl FnOnce(&mut Buffer) -> PointerShape) -> String {
        wire.0.borrow_mut().clear();
        screen.present(render).expect("a frame");
        String::from_utf8_lossy(&wire.0.borrow()).into_owned()
    }

    const OSC_22: &str = "\x1b]22;";

    #[test]
    fn the_pointer_shape_is_written_with_the_frame_only_when_it_changes() {
        let (screen, wire) = screen();
        let mut screen = screen.pointer_shapes(true);
        let first = written(&mut screen, &wire, paint("notes"));
        assert!(!first.contains(OSC_22), "the terminal starts with its usual pointer: {first:?}");
        let arrow = written(&mut screen, &wire, pointing("notes", PointerShape::EwResize));
        assert_eq!(arrow, "\x1b]22;ew-resize\x1b\\", "no cell changed, so the shape is all that is written");
        assert_eq!(
            bytes(&mut screen, &wire, pointing("notes", PointerShape::EwResize)),
            0,
            "the same shape again is not a byte"
        );
        let cells = written(&mut screen, &wire, pointing("notes!", PointerShape::EwResize));
        assert!(!cells.is_empty() && !cells.contains(OSC_22), "a changed cell with the same shape: {cells:?}");
        let both = written(&mut screen, &wire, pointing("notes?", PointerShape::NwseResize));
        let (begin, end) = (both.find("\x1b[?2026h"), both.find("\x1b[?2026l"));
        let at = both.find("\x1b]22;nwse-resize\x1b\\");
        assert!(begin < at && at < end, "the shape goes inside the frame's synchronized update: {both:?}");
        assert_eq!(
            written(&mut screen, &wire, paint("notes?")),
            "\x1b]22;default\x1b\\",
            "and back to the usual pointer"
        );
        assert_eq!(bytes(&mut screen, &wire, paint("notes?")), 0);
    }

    #[test]
    fn a_terminal_that_does_not_know_pointer_shapes_is_sent_none() {
        let (mut screen, wire) = screen();
        bytes(&mut screen, &wire, paint("notes"));
        assert_eq!(bytes(&mut screen, &wire, pointing("notes", PointerShape::EwResize)), 0);
        let cells = written(&mut screen, &wire, pointing("notes!", PointerShape::NsResize));
        assert!(!cells.is_empty() && !cells.contains(OSC_22), "{cells:?}");
        screen.reset_pointer_shape().expect("nothing to write");
        assert_eq!(wire.0.borrow().len(), cells.len(), "nor is anything written to reset it");
    }

    #[test]
    fn resetting_the_pointer_writes_the_usual_shape_once() {
        let (screen, wire) = screen();
        let mut screen = screen.pointer_shapes(true);
        bytes(&mut screen, &wire, pointing("notes", PointerShape::NeswResize));
        wire.0.borrow_mut().clear();
        screen.reset_pointer_shape().expect("a reset");
        assert_eq!(String::from_utf8_lossy(&wire.0.borrow()), "\x1b]22;default\x1b\\");
        let once = wire.0.borrow().len();
        screen.reset_pointer_shape().expect("a reset");
        assert_eq!(wire.0.borrow().len(), once, "already the usual pointer: nothing more");
    }

    #[test]
    fn pointer_shapes_are_sent_only_to_terminals_known_to_understand_them() {
        let with = |vars: &[(&str, &str)]| {
            let vars: Vec<(String, String)> = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
            pointer_shapes_supported(move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()))
        };
        assert!(with(&[("TERM", "xterm-kitty")]), "kitty");
        assert!(with(&[("TERM", "foot")]) && with(&[("TERM", "foot-extra")]), "foot");
        assert!(with(&[("TERM", "xterm-256color"), ("TERM_PROGRAM", "WezTerm")]), "WezTerm");
        assert!(!with(&[("TERM", "xterm-256color")]), "an unknown terminal gets nothing");
        assert!(!with(&[]), "nor does one that says nothing");
        assert!(
            !with(&[("TERM", "tmux-256color"), ("TMUX", "/tmp/tmux-1000/default,1,0")]),
            "tmux does not pass it on"
        );
        assert!(!with(&[("TERM", "xterm-kitty"), ("TMUX", "/tmp/tmux-1000/default,1,0")]));
        assert!(with(&[("TERM", "xterm-256color"), (POINTER_SHAPES_VAR, "on")]), "the variable turns it on");
        assert!(!with(&[("TERM", "xterm-kitty"), (POINTER_SHAPES_VAR, "off")]), "and off");
        assert!(
            with(&[("TERM", "xterm-kitty"), (POINTER_SHAPES_VAR, "auto")]),
            "anything else leaves it to the terminal"
        );
    }

    #[test]
    fn a_frame_equal_to_the_one_shown_writes_nothing() {
        let (mut screen, wire) = screen();
        assert!(bytes(&mut screen, &wire, paint("12:04")) > 0, "the first frame is written");
        assert_eq!(bytes(&mut screen, &wire, paint("12:04")), 0, "the same frame again is not a byte");
        assert!(bytes(&mut screen, &wire, paint("12:05")) > 0, "a changed cell is written");
        assert_eq!(bytes(&mut screen, &wire, paint("12:05")), 0);
    }

    #[test]
    fn after_the_screen_was_lent_the_same_frame_is_written_whole() {
        let (mut screen, wire) = screen();
        bytes(&mut screen, &wire, paint("notes"));
        screen.redraw_all(Rect::new(0, 0, 12, 2)).expect("a resize");
        assert!(bytes(&mut screen, &wire, paint("notes")) > 0, "nothing on screen can be trusted after a handoff");
    }
}
