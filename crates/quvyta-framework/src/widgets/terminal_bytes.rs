//! What the screen's parser is handed: a program's output with the few sequences the parser
//! does not know rewritten into ones it does.
//!
//! - HVP (`CSI row ; col f`) moves the cursor exactly as CUP (`CSI row ; col H`) does, and btop
//!   draws its whole screen with it. The parser ignores HVP, which piles every line onto one, so
//!   its final `f` becomes `H`. Only a sequence of digits and `;` is rewritten: one with a private
//!   marker (`CSI ? … f`) or an intermediate byte is some other command and is left alone.
//! - The DEC line drawing set (`ESC ( 0`, back with `ESC ( B`) draws boxes with plain letters:
//!   `lqqk` is `┌──┐`. The parser does not switch sets, so while the set is in use the letters are
//!   replaced by the characters they stand for. Both G0 and G1 are followed, and Shift Out and
//!   Shift In choose between them, since some programs draw through G1.
//!
//! Every decision is made on one byte with what came before it kept as a state, so a sequence
//! split across two reads is rewritten the same as a whole one and no byte is held back.

use super::terminal_notice::OscLimit;

const ESC: u8 = 0x1b;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1a;
const BEL: u8 = 0x07;
/// Shift Out: draw with G1.
const SO: u8 = 0x0e;
/// Shift In: draw with G0.
const SI: u8 = 0x0f;

/// The characters of the DEC line drawing set for the bytes `_` (0x5f) to `~` (0x7e), as xterm
/// draws them.
const LINE_DRAWING: [&str; 32] = [
    " ", "◆", "▒", "␉", "␌", "␍", "␊", "°", "±", "␤", "␋", "┘", "┐", "┌", "└", "┼", "⎺", "⎻", "─", "⎼", "⎽", "├", "┤",
    "┴", "┬", "│", "≤", "≥", "π", "≠", "£", "·",
];

/// Where the byte stream stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Place {
    #[default]
    Ground,
    /// Right after an ESC.
    Escape,
    /// After `ESC (` or `ESC )`: the next byte names the set of G0 (`false`) or G1 (`true`).
    Designate { g1: bool },
    /// After an ESC and an intermediate byte other than a set designation.
    EscapeIntermediate,
    /// Inside a CSI sequence; `plain` while it has held only digits and `;`.
    Csi { plain: bool },
    /// Inside an OSC, DCS, SOS, PM or APC string, which ends at BEL or at an ESC.
    String,
}

/// Rewrites a program's output for the parser, across reads; see the module documentation.
#[derive(Debug, Default)]
pub(super) struct Rewrite {
    place: Place,
    /// Whether G0 holds the line drawing set.
    g0_lines: bool,
    /// Whether G1 holds the line drawing set.
    g1_lines: bool,
    /// Whether Shift Out chose G1.
    shifted: bool,
}

impl Rewrite {
    /// Hands `bytes` to `sink` in runs, with the replacements put in.
    pub(super) fn feed(&mut self, bytes: &[u8], mut sink: impl FnMut(&[u8])) {
        let mut start = 0;
        for (index, &byte) in bytes.iter().enumerate() {
            if let Some(replacement) = self.step(byte) {
                if start < index {
                    sink(&bytes[start..index]);
                }
                sink(replacement);
                start = index + 1;
            }
        }
        if start < bytes.len() {
            sink(&bytes[start..]);
        }
    }

    /// Moves past one byte; what to hand over in its place, if it is replaced.
    fn step(&mut self, byte: u8) -> Option<&'static [u8]> {
        // These three cut any sequence short, wherever the stream stands.
        match byte {
            ESC => {
                self.place = Place::Escape;
                return None;
            }
            CAN | SUB => {
                self.place = Place::Ground;
                return None;
            }
            _ => {}
        }
        match self.place {
            Place::Ground => match byte {
                SO => self.shifted = true,
                SI => self.shifted = false,
                0x5f..=0x7e if self.lines() => {
                    return Some(LINE_DRAWING[usize::from(byte - 0x5f)].as_bytes());
                }
                _ => {}
            },
            Place::Escape => {
                self.place = match byte {
                    b'(' => Place::Designate { g1: false },
                    b')' => Place::Designate { g1: true },
                    0x20..=0x2f => Place::EscapeIntermediate,
                    b'[' => Place::Csi { plain: true },
                    b']' | b'P' | b'X' | b'^' | b'_' => Place::String,
                    // Other control bytes are carried out in the middle of a sequence.
                    0x00..=0x1f => Place::Escape,
                    _ => {
                        if byte == b'c' {
                            // A full reset: both sets go back to ASCII.
                            *self = Self::default();
                        }
                        Place::Ground
                    }
                };
            }
            Place::Designate { g1 } => match byte {
                0x00..=0x1f => {}
                0x20..=0x2f => self.place = Place::EscapeIntermediate,
                _ => {
                    let lines = byte == b'0';
                    if g1 {
                        self.g1_lines = lines;
                    } else {
                        self.g0_lines = lines;
                    }
                    self.place = Place::Ground;
                }
            },
            Place::EscapeIntermediate => {
                if !matches!(byte, 0x00..=0x2f) {
                    self.place = Place::Ground;
                }
            }
            Place::Csi { plain } => match byte {
                0x00..=0x1f => {}
                b'0'..=b'9' | b';' => {}
                // A private marker, a sub-parameter or an intermediate byte: not a plain HVP.
                0x20..=0x3f => self.place = Place::Csi { plain: false },
                0x40..=0x7e => {
                    self.place = Place::Ground;
                    if plain && byte == b'f' {
                        return Some(b"H");
                    }
                }
                _ => self.place = Place::Ground,
            },
            Place::String => {
                if byte == BEL {
                    self.place = Place::Ground;
                }
            }
        }
        None
    }

    /// Whether the set drawn with now is the line drawing set.
    fn lines(&self) -> bool {
        if self.shifted { self.g1_lines } else { self.g0_lines }
    }
}

/// The whole way from a read of the program's output to the parser: overlong OSC strings cut,
/// then the rewrites above.
#[derive(Debug, Default)]
pub(super) struct ByteFeed {
    limit: OscLimit,
    rewrite: Rewrite,
}

impl ByteFeed {
    /// Parses one read of the program's output.
    pub(super) fn feed<C: vt100::Callbacks>(&mut self, bytes: &[u8], parser: &mut vt100::Parser<C>) {
        let Self { limit, rewrite } = self;
        limit.feed(bytes, |run| rewrite.feed(run, |out| parser.process(out)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The screen after parsing `chunks` one after another, as separate reads.
    fn screen(rows: u16, cols: u16, chunks: &[&[u8]]) -> vt100::Screen {
        let mut parser = vt100::Parser::new(rows, cols, 0);
        let mut feed = ByteFeed::default();
        for chunk in chunks {
            feed.feed(chunk, &mut parser);
        }
        parser.screen().clone()
    }

    fn rows(screen: &vt100::Screen) -> Vec<String> {
        let (_, cols) = screen.size();
        screen.rows(0, cols).map(|row| row.trim_end().to_owned()).collect()
    }

    #[test]
    fn hvp_moves_the_cursor_like_cup() {
        let shown = screen(4, 20, &[b"\x1b[2;3fcpu\x1b[4;1fmem\x1b[1;10fup"]);
        assert_eq!(rows(&shown), ["         up", "  cpu", "", "mem"]);
        let bare = screen(3, 10, &[b"xx\x1b[fA\x1b[3;fB"]);
        assert_eq!(rows(&bare), ["Ax", "", "B"], "missing numbers mean 1, as for CUP");
    }

    #[test]
    fn an_hvp_split_across_reads_still_moves_the_cursor() {
        let stream: &[u8] = b"\x1b[2;3fhere";
        for split in 1..stream.len() {
            let shown = screen(3, 10, &[&stream[..split], &stream[split..]]);
            assert_eq!(rows(&shown), ["", "  here", ""], "split at {split}");
        }
    }

    #[test]
    fn other_sequences_ending_in_f_are_left_alone() {
        let mut out = Vec::new();
        let mut rewrite = Rewrite::default();
        for sequence in [&b"\x1b[?5f"[..], b"\x1b[>1;2f", b"\x1b[1 f", b"\x1b[1:2f", b"\x1b]0;f\x07", b"\x1bPf\x1b\\"] {
            out.clear();
            rewrite.feed(sequence, |run| out.extend_from_slice(run));
            assert_eq!(out, sequence, "{:?}", String::from_utf8_lossy(sequence));
        }
        out.clear();
        rewrite.feed(b"of fun\x1b[3;4f", |run| out.extend_from_slice(run));
        assert_eq!(out, b"of fun\x1b[3;4H", "an f in text is text");
    }

    #[test]
    fn the_line_drawing_set_draws_lines() {
        let shown = screen(3, 10, &[b"\x1b(0lqqk\r\nx  x\r\nmqqj\x1b(B lq"]);
        assert_eq!(rows(&shown), ["┌──┐", "│  │", "└──┘ lq"], "and back to letters after ESC ( B");
    }

    #[test]
    fn the_line_drawing_set_through_g1_and_shift_out() {
        let shown = screen(1, 12, &[b"\x1b)0q\x0eq\x0fq\x0eq"]);
        assert_eq!(rows(&shown), ["q─q─"]);
        let reset = screen(1, 12, &[b"\x1b(0q\x1bcq"]);
        assert_eq!(rows(&reset), ["q"], "a full reset clears the screen and the sets");
    }

    #[test]
    fn a_set_switch_split_across_reads_still_switches() {
        let stream: &[u8] = b"\x1b(0lqk\x1b(Bok";
        for split in 1..stream.len() {
            let shown = screen(1, 10, &[&stream[..split], &stream[split..]]);
            assert_eq!(rows(&shown), ["┌─┐ok"], "split at {split}");
        }
    }

    #[test]
    fn letters_inside_sequences_are_never_drawn_as_lines() {
        // A colour and a title while the line set is in use keep their letters.
        let shown = screen(1, 10, &[b"\x1b(0\x1b[31mq\x1b]0;lqk\x07q"]);
        assert_eq!(rows(&shown), ["──"]);
        assert_eq!(shown.cell(0, 0).map(vt100::Cell::fgcolor), Some(vt100::Color::Idx(1)));
    }

    /// A monitor's first frame the way btop draws it, kept in `tests/fixtures`: the alternate
    /// screen, HVP for every move, 256 colours and boxes of box drawing characters.
    #[test]
    fn a_monitor_s_first_frame_lands_on_its_rows() {
        let frame = include_bytes!("../../tests/fixtures/monitor-first-frame.txt");
        // Handed over in small reads, as a pseudo-terminal does.
        let chunks: Vec<&[u8]> = frame.chunks(7).collect();
        let shown = screen(8, 30, &chunks);
        assert_eq!(
            rows(&shown),
            [
                "┌─cpu──────────────────────┐",
                "│ user   12%               │",
                "│ system  3%               │",
                "└──────────────────────────┘",
                "┌─mem──────────────────────┐",
                "│ used  1.2G of 8G         │",
                "└──────────────────────────┘",
                " q quit",
            ]
        );
    }
}
