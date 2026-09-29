//! The questions a program asks its terminal and waits for: where the cursor is (`CSI 6 n`),
//! whether the terminal is well (`CSI 5 n`), what kind of terminal it is (`CSI c`), which colours
//! it draws text and ground in (`OSC 10 ; ?`, `OSC 11 ; ?`) and whether a mode is set
//! (`CSI ? mode $ p`).
//!
//! Every real terminal answers these. A program left without an answer either waits for it before
//! it draws, or guesses: an editor asks for the ground colour to choose its light or dark
//! colours, and asks where the cursor is right after, so the second answer tells it the first
//! one is not coming. The answers are written where the question was read, in the order the
//! questions came, together with the keyboard protocol's own.

use crate::color::Rgb;

/// What this terminal says it is: a VT220-class terminal (62) with ANSI colour (22). Programs
/// read the class to decide which sequences they may send; nothing here claims more than the
/// screen draws.
const DEVICE: &[u8] = b"\x1b[?62;22c";

/// The colours the terminal draws with when a program names none, which is what it asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Colours {
    pub(crate) text: Rgb,
    pub(crate) ground: Rgb,
    pub(crate) cursor: Rgb,
}

/// Answers owed to one program; see the module documentation.
#[derive(Debug, Default)]
pub(crate) struct Answers {
    /// Known once the widget has drawn the session; until then a colour question stays
    /// unanswered, as it does on a terminal that does not know the request, rather than being
    /// answered with a colour the screen does not show.
    colours: Option<Colours>,
}

impl Answers {
    /// Remembers the colours the widget draws the session with, for the next question.
    pub(crate) fn set_colours(&mut self, colours: Colours) {
        self.colours = Some(colours);
    }

    /// The answer to a CSI sequence ending in `end`, whose intermediate bytes are `marker` and
    /// `second`, asked on `screen`; `focus_reports` says whether the program turned mode 1004 on,
    /// which the screen itself does not keep.
    pub(crate) fn csi(
        &self,
        screen: &vt100::Screen,
        focus_reports: bool,
        (marker, second): (Option<u8>, Option<u8>),
        params: &[&[u16]],
        end: char,
    ) -> Option<Vec<u8>> {
        let param = |index: usize| params.get(index).and_then(|param| param.first()).copied().unwrap_or(0);
        match (marker, second, end) {
            (None, None, 'n') => match param(0) {
                5 => Some(b"\x1b[0n".to_vec()),
                6 => {
                    let (row, col) = screen.cursor_position();
                    Some(format!("\x1b[{};{}R", u32::from(row) + 1, u32::from(col) + 1).into_bytes())
                }
                _ => None,
            },
            (None, None, 'c') if param(0) == 0 => Some(DEVICE.to_vec()),
            (Some(b'?'), Some(b'$'), 'p') => {
                let mode = param(0);
                let state = mode_state(screen, focus_reports, mode);
                Some(format!("\x1b[?{mode};{state}$y").into_bytes())
            }
            _ => None,
        }
    }

    /// The answer to an OSC string, when it asks for a colour this terminal knows.
    pub(crate) fn osc(&self, params: &[&[u8]]) -> Option<Vec<u8>> {
        let [number, b"?"] = params else { return None };
        let colours = self.colours?;
        let colour = match *number {
            b"10" => colours.text,
            b"11" => colours.ground,
            b"12" => colours.cursor,
            _ => return None,
        };
        let number = std::str::from_utf8(number).ok()?;
        // Sixteen bits a channel, as xterm answers: the byte written twice.
        let wide = |channel: u8| u16::from(channel) * 0x101;
        Some(
            format!("\x1b]{number};rgb:{:04x}/{:04x}/{:04x}\x1b\\", wide(colour.r), wide(colour.g), wide(colour.b))
                .into_bytes(),
        )
    }
}

/// DECRQM's answer for `mode`: 1 set, 2 reset, 0 a mode this terminal does not know. Saying a
/// mode is unknown is the honest answer for one the screen does not draw, so a program does not
/// count on it, such as synchronized output.
fn mode_state(screen: &vt100::Screen, focus_reports: bool, mode: u16) -> u8 {
    let on = |set: bool| if set { 1 } else { 2 };
    match mode {
        1 => on(screen.application_cursor()),
        25 => on(!screen.hide_cursor()),
        47 | 1047 | 1049 => on(screen.alternate_screen()),
        1000 => on(screen.mouse_protocol_mode() == vt100::MouseProtocolMode::PressRelease),
        1002 => on(screen.mouse_protocol_mode() == vt100::MouseProtocolMode::ButtonMotion),
        1003 => on(screen.mouse_protocol_mode() == vt100::MouseProtocolMode::AnyMotion),
        1004 => on(focus_reports),
        1006 => on(screen.mouse_protocol_encoding() == vt100::MouseProtocolEncoding::Sgr),
        2004 => on(screen.bracketed_paste()),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen_after(bytes: &[u8]) -> vt100::Parser {
        let mut parser = vt100::Parser::new(24, 80, 0);
        parser.process(bytes);
        parser
    }

    fn csi(
        answers: &Answers,
        screen: &vt100::Screen,
        marker: (Option<u8>, Option<u8>),
        params: &[u16],
        end: char,
    ) -> Option<String> {
        let params: Vec<&[u16]> = params.iter().map(std::slice::from_ref).collect();
        answers.csi(screen, false, marker, &params, end).map(|bytes| String::from_utf8(bytes).expect("ascii"))
    }

    #[test]
    fn the_cursor_is_reported_where_the_screen_has_it() {
        let parser = screen_after(b"\x1b[5;12Habc");
        let answer = csi(&Answers::default(), parser.screen(), (None, None), &[6], 'n');
        assert_eq!(answer.as_deref(), Some("\x1b[5;15R"), "row 5, column 12 plus three letters");
    }

    #[test]
    fn a_status_and_a_device_question_are_answered() {
        let parser = screen_after(b"");
        let answers = Answers::default();
        assert_eq!(csi(&answers, parser.screen(), (None, None), &[5], 'n').as_deref(), Some("\x1b[0n"));
        assert_eq!(csi(&answers, parser.screen(), (None, None), &[], 'c').as_deref(), Some("\x1b[?62;22c"));
        assert_eq!(csi(&answers, parser.screen(), (None, None), &[0], 'c').as_deref(), Some("\x1b[?62;22c"));
        assert_eq!(csi(&answers, parser.screen(), (None, None), &[7], 'n'), None, "an unknown report asks nothing");
        assert_eq!(
            csi(&answers, parser.screen(), (Some(b'>'), None), &[], 'c'),
            None,
            "the second device question is not this one"
        );
    }

    #[test]
    fn a_mode_question_says_set_reset_or_unknown() {
        let parser = screen_after(b"\x1b[?2004h\x1b[?1049h");
        let answers = Answers::default();
        let asked = |mode: u16| csi(&answers, parser.screen(), (Some(b'?'), Some(b'$')), &[mode], 'p');
        assert_eq!(asked(2004).as_deref(), Some("\x1b[?2004;1$y"), "bracketed paste is on");
        assert_eq!(asked(1049).as_deref(), Some("\x1b[?1049;1$y"), "the alternate screen is up");
        assert_eq!(asked(1002).as_deref(), Some("\x1b[?1002;2$y"), "mouse drags are off");
        assert_eq!(asked(2026).as_deref(), Some("\x1b[?2026;0$y"), "synchronized output is not drawn here");
        let focus: &[&[u16]] = &[&[1004]];
        let told = answers.csi(parser.screen(), true, (Some(b'?'), Some(b'$')), focus, 'p');
        assert_eq!(told.as_deref(), Some(&b"\x1b[?1004;1$y"[..]), "the focus reports the session keeps");
    }

    #[test]
    fn colours_are_told_once_the_widget_has_drawn_them() {
        let mut answers = Answers::default();
        assert_eq!(answers.osc(&[b"11", b"?"]), None, "no colour is made up before one is known");
        answers.set_colours(Colours {
            text: Rgb::new(0xee, 0xdd, 0xcc),
            ground: Rgb::new(0x12, 0x34, 0x56),
            cursor: Rgb::new(1, 2, 3),
        });
        let told = |number: &[u8]| answers.osc(&[number, b"?"]).map(|bytes| String::from_utf8(bytes).expect("ascii"));
        assert_eq!(told(b"11").as_deref(), Some("\x1b]11;rgb:1212/3434/5656\x1b\\"));
        assert_eq!(told(b"10").as_deref(), Some("\x1b]10;rgb:eeee/dddd/cccc\x1b\\"));
        assert_eq!(told(b"12").as_deref(), Some("\x1b]12;rgb:0101/0202/0303\x1b\\"));
        assert_eq!(answers.osc(&[b"11", b"#000000"]), None, "setting a colour is not a question");
        assert_eq!(answers.osc(&[b"4", b"1", b"?"]), None, "the palette is not answered");
    }
}
