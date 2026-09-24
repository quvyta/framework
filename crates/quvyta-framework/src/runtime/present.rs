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
//! pointer changed, only to a terminal known to understand it. Pictures a kitty terminal draws
//! itself travel the same way: after the cells, only what changed about them (see
//! [`KittyPictures`](super::kitty::KittyPictures)). Sixel pictures are painted into the cells, so
//! they come after the cells too, whenever a cell under them was written (see
//! [`SixelPictures`](super::sixel::SixelPictures)). A frame that changed no cell, no shape and no
//! picture still writes nothing.

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

/// What a painted frame asks of the terminal beyond its cells.
#[derive(Debug, Clone, Default)]
pub(crate) struct Painted {
    /// The pointer's shape.
    pub(crate) shape: PointerShape,
    /// The pictures the terminal draws itself, where it shows them.
    #[cfg(feature = "image")]
    pub(crate) pictures: Vec<crate::widgets::image::PicturePlacement>,
    /// The identities of every picture painted for the terminal to draw, placed or not.
    #[cfg(feature = "image")]
    pub(crate) painted: Vec<u64>,
    /// Whether the terminal draws those pictures with sixel rather than the kitty protocol.
    #[cfg(feature = "image")]
    pub(crate) sixel: bool,
}

impl From<PointerShape> for Painted {
    fn from(shape: PointerShape) -> Self {
        Self {
            shape,
            #[cfg(feature = "image")]
            pictures: Vec::new(),
            #[cfg(feature = "image")]
            painted: Vec::new(),
            #[cfg(feature = "image")]
            sixel: false,
        }
    }
}

/// Asks the terminal the size of a cell in pixels; `None` when it does not say.
#[cfg(feature = "image")]
type MeasureCell = fn() -> Option<(u16, u16)>;

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
    /// The size the last frame was drawn at; the terminal was cleared when it changes.
    area: Option<Rect>,
    /// The pictures a kitty terminal draws itself, and what it holds of them.
    #[cfg(feature = "image")]
    pictures: super::kitty::KittyPictures,
    /// The pictures painted with sixel, and where.
    #[cfg(feature = "image")]
    sixels: super::sixel::SixelPictures,
    /// Asks the terminal the size of a cell in pixels, at the first frame and at every new size.
    #[cfg(feature = "image")]
    measure_cell: Option<MeasureCell>,
}

impl<W: Write> Screen<W> {
    pub(crate) fn new(terminal: Terminal<CrosstermBackend<W>>) -> Self {
        Self {
            terminal,
            shown: None,
            pointer_shapes: false,
            shape: PointerShape::Default,
            area: None,
            #[cfg(feature = "image")]
            pictures: super::kitty::KittyPictures::default(),
            #[cfg(feature = "image")]
            sixels: super::sixel::SixelPictures::default(),
            #[cfg(feature = "image")]
            measure_cell: None,
        }
    }

    /// Asks `measure` the size of a cell in pixels at the first frame and at every new size, for
    /// shrinking sixel pictures to the pixels their cells cover. Without it, or when it answers
    /// `None`, a cell is taken to be 10 × 20 pixels.
    #[cfg(feature = "image")]
    pub(crate) fn measure_cell(mut self, measure: MeasureCell) -> Self {
        self.measure_cell = Some(measure);
        self
    }

    /// Tells the terminal the pointer's shape, for a terminal that understands OSC 22.
    pub(crate) fn pointer_shapes(mut self, on: bool) -> Self {
        self.pointer_shapes = on;
        self
    }

    /// Paints a frame with `render`, which answers what the frame asks beyond its cells, and
    /// writes what changed inside one synchronized update: the cells, the cells a sixel picture no
    /// longer shown still covers and the last rows a sixel's bands leave short, the pointer's
    /// shape when it is not the one the terminal shows, then the pictures when their places
    /// changed or, for sixel, the pixels of the cells that lost theirs. Returns
    /// whether anything was written: a frame equal to the one shown, with the same shape and the
    /// same pictures, writes nothing.
    pub(crate) fn present(&mut self, render: impl FnOnce(&mut Buffer) -> Painted) -> io::Result<bool> {
        self.terminal.autoresize()?;
        let mut frame = self.terminal.get_frame();
        let area = frame.area();
        let painted = render(frame.buffer_mut());
        let before = self.area.replace(area);
        if before != Some(area) {
            #[cfg(feature = "image")]
            if let Some(measure) = self.measure_cell {
                self.sixels.set_cell(measure().unwrap_or(super::sixel::CELL));
            }
            // A new size clears the terminal, and pictures with it in some terminals.
            #[cfg(feature = "image")]
            if before.is_some() {
                self.pictures.resized();
            }
        }
        let reshape = Some(painted.shape).filter(|shape| self.pointer_shapes && *shape != self.shape);
        // The pictures go to one of the two ways of drawing them; the other is told there are none,
        // so switching between them leaves nothing of the other behind.
        #[cfg(feature = "image")]
        let (kitty, sixel) = {
            let (kitty, sixel): (&[_], &[_]) =
                if painted.sixel { (&[], &painted.pictures) } else { (&painted.pictures, &[]) };
            let (kitty_painted, sixel_painted): (&[u64], &[u64]) =
                if painted.sixel { (&[], &painted.painted) } else { (&painted.painted, &[]) };
            let kitty = self.pictures.commands(kitty, kitty_painted);
            let sixel =
                self.sixels.frame(sixel, sixel_painted, self.shown.as_ref(), self.terminal.current_buffer_mut());
            (kitty, sixel)
        };
        #[cfg(not(feature = "image"))]
        let (kitty, sixel): (Vec<u8>, Vec<u8>) = (Vec::new(), Vec::new());
        #[cfg(feature = "image")]
        let (repaint, short, sixel) = (sixel.repaint, sixel.short, sixel.bytes);
        #[cfg(not(feature = "image"))]
        let (repaint, short): (Vec<(u16, u16)>, Vec<(u16, u16, ratatui_core::buffer::Cell)>) = (Vec::new(), Vec::new());
        let buffer = self.terminal.current_buffer_mut();
        // The buffer painted this time is reused by the next frame, which paints every cell
        // again, so when it equals the one shown it is left as it is and no cell is written.
        let changed = (self.shown.as_ref() != Some(&*buffer)).then(|| buffer.clone());
        // Read now: writing the cells hands the next frame this buffer, emptied.
        let mut repaint: Vec<(u16, u16, ratatui_core::buffer::Cell)> =
            repaint.into_iter().map(|(x, y)| (x, y, buffer[(x, y)].clone())).collect();
        // Before the pictures, since writing a cell wipes the pixels in it.
        repaint.extend(short);
        let pictures = !kitty.is_empty() || !sixel.is_empty() || !repaint.is_empty();
        if changed.is_none() && !pictures {
            let Some(shape) = reshape else { return Ok(false) };
            self.write_shape(shape)?;
            self.terminal.backend_mut().flush()?;
            return Ok(true);
        }
        execute!(self.terminal.backend_mut(), BeginSynchronizedUpdate)?;
        if changed.is_some() {
            self.terminal.apply_buffer_with_cursor(None)?;
        }
        if !repaint.is_empty() {
            let backend = self.terminal.backend_mut();
            ratatui_core::backend::Backend::draw(backend, repaint.iter().map(|(x, y, cell)| (*x, *y, cell)))?;
        }
        if let Some(shape) = reshape {
            self.write_shape(shape)?;
        }
        self.terminal.backend_mut().write_all(&kitty)?;
        self.terminal.backend_mut().write_all(&sixel)?;
        execute!(self.terminal.backend_mut(), EndSynchronizedUpdate)?;
        if let Some(buffer) = changed {
            self.shown = Some(buffer);
        }
        Ok(true)
    }

    /// Frees every picture the terminal holds, for before the terminal is handed to a program or
    /// left: their pixels would otherwise stay in its memory.
    #[cfg(feature = "image")]
    pub(crate) fn release_pictures(&mut self) -> io::Result<()> {
        self.sixels.release();
        let bytes = self.pictures.release();
        if !bytes.is_empty() {
            self.terminal.backend_mut().write_all(&bytes)?;
            self.terminal.backend_mut().flush()?;
        }
        Ok(())
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

    /// Forgets what is on screen and takes the size `area`, so the next frame draws every cell
    /// and sends and places every picture again: for after a program had the terminal and wrote
    /// over it.
    pub(crate) fn redraw_all(&mut self, area: Rect) -> io::Result<()> {
        self.terminal.resize(area)?;
        self.shown = None;
        #[cfg(feature = "image")]
        self.pictures.forget();
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

    fn paint(text: &'static str) -> impl FnOnce(&mut Buffer) -> Painted {
        pointing(text, PointerShape::Default)
    }

    /// Paints `text` and asks for `shape`.
    fn pointing(text: &'static str, shape: PointerShape) -> impl FnOnce(&mut Buffer) -> Painted {
        move |buffer: &mut Buffer| {
            buffer.reset();
            buffer.set_string(0, 0, text, ratatui_core::style::Style::default());
            shape.into()
        }
    }

    /// Bytes written by one frame.
    fn bytes(screen: &mut Screen<Wire>, wire: &Wire, render: impl FnOnce(&mut Buffer) -> Painted) -> usize {
        written(screen, wire, render).len()
    }

    /// What one frame wrote, as text.
    fn written(screen: &mut Screen<Wire>, wire: &Wire, render: impl FnOnce(&mut Buffer) -> Painted) -> String {
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
