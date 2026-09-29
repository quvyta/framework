//! What a program in a terminal says besides drawing: its title, its folder, the bell, its
//! notifications and the text it wants copied, read from the byte stream by the screen's parser.

use std::collections::VecDeque;
use std::path::PathBuf;

use super::terminal_answers::Answers;
use super::terminal_focus::Focus;
use super::terminal_keyboard::Keyboard;
use super::terminal_session::TerminalChange;

/// The longest OSC string handed to the parser, in bytes. The parser keeps a whole OSC string in
/// memory until it ends, so a program that opens one and never closes it could make it grow
/// without end; past this length the rest of the string is dropped. Titles, folders,
/// notifications and copied text fit many times over.
const OSC_LIMIT: usize = 4096;

/// Notifications kept until the application reads them; a flood keeps the newest.
const NOTIFY_LIMIT: usize = 8;

/// Notices the parser has heard and the application has not read yet.
///
/// A title, a folder and a copied text replace the one before, the bell is one mark however often
/// it rang, and notifications queue up to [`NOTIFY_LIMIT`], so nothing here grows with the stream.
#[derive(Debug, Default)]
pub(crate) struct Notices {
    title: Option<String>,
    folder: Option<PathBuf>,
    copy: Option<String>,
    notes: VecDeque<(Option<String>, String)>,
    bell: bool,
}

impl Notices {
    pub(super) fn is_empty(&self) -> bool {
        self.title.is_none() && self.folder.is_none() && self.copy.is_none() && self.notes.is_empty() && !self.bell
    }

    /// Adds what was heard after these notices.
    pub(super) fn merge(&mut self, newer: Self) {
        if newer.title.is_some() {
            self.title = newer.title;
        }
        if newer.folder.is_some() {
            self.folder = newer.folder;
        }
        if newer.copy.is_some() {
            self.copy = newer.copy;
        }
        for (title, body) in newer.notes {
            self.notify(title, body);
        }
        self.bell |= newer.bell;
    }

    /// The oldest kind of notice still unread: the title, then the folder, then the text to copy,
    /// then notifications in order, then the bell.
    pub(super) fn pop(&mut self) -> Option<TerminalChange> {
        if let Some(title) = self.title.take() {
            return Some(TerminalChange::Title(title));
        }
        if let Some(folder) = self.folder.take() {
            return Some(TerminalChange::WorkingFolder(folder));
        }
        if let Some(text) = self.copy.take() {
            return Some(TerminalChange::Copied(text));
        }
        if let Some((title, body)) = self.notes.pop_front() {
            return Some(TerminalChange::Notify { title, body });
        }
        std::mem::take(&mut self.bell).then_some(TerminalChange::Bell)
    }

    fn notify(&mut self, title: Option<String>, body: String) {
        if self.notes.len() == NOTIFY_LIMIT {
            self.notes.pop_front();
        }
        self.notes.push_back((title, body));
    }
}

impl vt100::Callbacks for Notices {
    fn audible_bell(&mut self, _: &mut vt100::Screen) {
        self.bell = true;
    }

    // OSC 0 and 2 with exactly one parameter; a title with `;` in it arrives in `unhandled_osc`.
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title = Some(text(title));
    }

    // OSC 52 offering text to copy, `52;c;aGVsbG8=`. The parser leaves out an offer whose payload
    // is not base64, so what arrives here decodes or names nothing to copy.
    fn copy_to_clipboard(&mut self, _: &mut vt100::Screen, _: &[u8], data: &[u8]) {
        if let Ok(encoded) = std::str::from_utf8(data)
            && let Some(text) = crate::runtime::decode_base64(encoded)
            && !text.is_empty()
        {
            self.copy = Some(text);
        }
    }

    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        match params {
            [b"0" | b"2", rest @ ..] if !rest.is_empty() => self.title = Some(joined(rest)),
            [b"7", rest @ ..] if !rest.is_empty() => {
                if let Some(folder) = folder(&rest.join(&b';')) {
                    self.folder = Some(folder);
                }
            }
            // ConEmu uses OSC 9 with a number first for other things (9;4 is progress), which a
            // notification never starts with.
            [b"9", first, ..] if !first.iter().all(u8::is_ascii_digit) => {
                self.notify(None, joined(&params[1..]));
            }
            [b"777", b"notify", title, rest @ ..] => self.notify(Some(text(title)), joined(rest)),
            _ => {}
        }
    }
}

/// Everything the parser hears besides drawing: the notices, which are handed to the application
/// and start over, the keyboard the program asked for, and the focus reports it asked for, which
/// last as long as the session, and the answers owed to its questions.
#[derive(Debug, Default)]
pub(crate) struct Heard {
    pub(crate) notices: Notices,
    pub(crate) keyboard: Keyboard,
    pub(crate) focus: Focus,
    pub(crate) answers: Answers,
    /// Answers owed to the program, in the order it asked, written back by the session's reading
    /// thread. One queue for every kind, since a program reads the order: an editor asks for the
    /// ground colour and then for the cursor, and takes the second answer arriving first as a
    /// terminal that does not know the colour.
    replies: Vec<u8>,
}

impl Heard {
    /// The answers owed to the program since the last call.
    pub(crate) fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }
}

impl vt100::Callbacks for Heard {
    fn audible_bell(&mut self, screen: &mut vt100::Screen) {
        self.notices.audible_bell(screen);
    }

    fn set_window_title(&mut self, screen: &mut vt100::Screen, title: &[u8]) {
        self.notices.set_window_title(screen, title);
    }

    fn copy_to_clipboard(&mut self, screen: &mut vt100::Screen, targets: &[u8], data: &[u8]) {
        self.notices.copy_to_clipboard(screen, targets, data);
    }

    fn unhandled_osc(&mut self, screen: &mut vt100::Screen, params: &[&[u8]]) {
        match self.answers.osc(params) {
            Some(answer) => self.replies.extend_from_slice(&answer),
            None => self.notices.unhandled_osc(screen, params),
        }
    }

    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        marker: Option<u8>,
        second: Option<u8>,
        params: &[&[u16]],
        end: char,
    ) {
        self.keyboard.csi(screen.alternate_screen(), marker, params, end);
        self.replies.append(&mut self.keyboard.take_replies());
        self.focus.csi(marker, params, end);
        if let Some(answer) = self.answers.csi(screen, self.focus.reporting(), (marker, second), params, end) {
            self.replies.extend_from_slice(&answer);
        }
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Parameters split at `;` put back together: a title or a message may contain one.
fn joined(params: &[&[u8]]) -> String {
    text(&params.join(&b';'))
}

/// The path of an OSC 7 `file://host/path` address, percent-decoded. The host is not checked:
/// over ssh it names the other machine, and the path is still what the program reports.
fn folder(address: &[u8]) -> Option<PathBuf> {
    let rest = address.strip_prefix(b"file://")?;
    let path = &rest[rest.iter().position(|&byte| byte == b'/')?..];
    Some(path_from_bytes(percent_decoded(path)))
}

fn percent_decoded(bytes: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        let escaped = (byte == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(value) => {
                decoded.push(value);
                index += 3;
            }
            // A `%` without two hex digits is kept as it is.
            None => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    decoded
}

#[cfg(unix)]
fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(bytes).into()
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: Vec<u8>) -> PathBuf {
    String::from_utf8_lossy(&bytes).into_owned().into()
}

/// Where the byte stream stands for [`OscLimit`].
#[derive(Debug, Clone, Copy, Default)]
enum Place {
    #[default]
    Ground,
    /// Right after an ESC outside an OSC string.
    Escape,
    /// Inside an OSC string, with this many bytes of it passed on.
    Osc(usize),
    /// Right after an ESC inside an OSC string: it ends the string, and a `]` opens the next.
    OscEscape,
}

/// Cuts OSC strings longer than [`OSC_LIMIT`] before they reach the parser, across reads.
///
/// Follows the parser's own rules for where a string starts (ESC `]`) and ends (BEL, ESC, CAN or
/// SUB), so the parser still sees the end and dispatches the shortened string.
#[derive(Debug, Default)]
pub(super) struct OscLimit {
    place: Place,
}

impl OscLimit {
    /// Hands `bytes` to `sink` in runs, leaving out what lies beyond the limit of an OSC string.
    pub(super) fn feed(&mut self, bytes: &[u8], mut sink: impl FnMut(&[u8])) {
        let mut start = 0;
        for (index, &byte) in bytes.iter().enumerate() {
            if !self.step(byte) {
                if start < index {
                    sink(&bytes[start..index]);
                }
                start = index + 1;
            }
        }
        if start < bytes.len() {
            sink(&bytes[start..]);
        }
    }

    /// Moves past one byte; `false` when the byte is dropped.
    fn step(&mut self, byte: u8) -> bool {
        const BEL: u8 = 0x07;
        const CAN: u8 = 0x18;
        const SUB: u8 = 0x1a;
        const ESC: u8 = 0x1b;
        let (place, keep) = match (self.place, byte) {
            (Place::Osc(_), BEL | CAN | SUB) => (Place::Ground, true),
            (Place::Osc(_), ESC) => (Place::OscEscape, true),
            (Place::Osc(length), _) if length < OSC_LIMIT => (Place::Osc(length + 1), true),
            (Place::Osc(length), _) => (Place::Osc(length), false),
            (Place::Escape | Place::OscEscape, b']') => (Place::Osc(0), true),
            (_, ESC) => (Place::Escape, true),
            _ => (Place::Ground, true),
        };
        self.place = place;
        keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses `chunks` one after another, as separate reads, and returns what was heard.
    fn heard(chunks: &[&[u8]]) -> (Vec<TerminalChange>, String) {
        let mut parser = vt100::Parser::new_with_callbacks(4, 40, 0, Notices::default());
        let mut limit = OscLimit::default();
        for chunk in chunks {
            limit.feed(chunk, |run| parser.process(run));
        }
        let mut changes = Vec::new();
        while let Some(change) = parser.callbacks_mut().pop() {
            changes.push(change);
        }
        (changes, parser.screen().contents())
    }

    #[test]
    fn titles_folders_bells_and_notifications() {
        let (changes, _) = heard(&[b"\x1b]0;one\x07"]);
        assert_eq!(changes, [TerminalChange::Title("one".into())]);
        let (changes, _) = heard(&[b"\x1b]2;a;b\x1b\\"]);
        assert_eq!(changes, [TerminalChange::Title("a;b".into())], "ST ends it and a ; stays");
        let (changes, _) = heard(&[b"\x1b]7;file://host/tmp/x%20y\x1b\\"]);
        assert_eq!(changes, [TerminalChange::WorkingFolder("/tmp/x y".into())]);
        let (changes, _) = heard(&[b"\x1b]7;file://host/a%2g%\x07"]);
        assert_eq!(changes, [TerminalChange::WorkingFolder("/a%2g%".into())], "bad escapes are kept");
        let (changes, _) = heard(&[b"\x1b]7;/not/an/address\x07"]);
        assert_eq!(changes, []);
        let (changes, _) = heard(&[b"\x1b]9;hi; there\x07"]);
        assert_eq!(changes, [TerminalChange::Notify { title: None, body: "hi; there".into() }]);
        let (changes, _) = heard(&[b"\x1b]9;4;1;50\x07"]);
        assert_eq!(changes, [], "ConEmu progress is not a notification");
        let (changes, _) = heard(&[b"\x1b]777;notify;T;B\x07"]);
        assert_eq!(changes, [TerminalChange::Notify { title: Some("T".into()), body: "B".into() }]);
        let (changes, screen) = heard(&[b"a\x07b\x07"]);
        assert_eq!((changes, screen.as_str()), (vec![TerminalChange::Bell], "ab"), "two rings, one mark");
    }

    #[test]
    fn text_a_program_offers_to_copy_is_heard() {
        let (changes, _) = heard(&[b"\x1b]52;c;aGVsbG8=\x07"]);
        assert_eq!(changes, [TerminalChange::Copied("hello".into())]);
        let (changes, _) = heard(&[b"\x1b]52;p;w6dheQ==\x1b\\"]);
        assert_eq!(changes, [TerminalChange::Copied("çay".into())], "ST ends it and any target names it");
        let (changes, _) = heard(&[b"\x1b]52;0;Zm9vYmFy\x07"]);
        assert_eq!(changes, [TerminalChange::Copied("foobar".into())], "a cut buffer is an offer as well");
    }

    #[test]
    fn a_read_request_and_text_that_names_nothing_are_dropped() {
        for asked in [
            &b"\x1b]52;c;?\x07"[..],
            b"\x1b]52;p;?\x1b\\",
            b"\x1b]52;c;\x07",
            b"\x1b]52;c;//79\x07",
            b"\x1b]52;c;not!base64\x07",
            b"\x1b]52;c",
        ] {
            let (changes, _) = heard(&[asked]);
            assert_eq!(changes, [], "{asked:?} names nothing to copy");
        }
    }

    #[test]
    fn only_the_newest_offer_waits_to_be_read() {
        let stream: &[u8] = b"\x1b]52;c;Zmlyc3Q=\x07\x1b]52;c;c2Vjb25k\x07";
        let (changes, _) = heard(&[stream]);
        assert_eq!(changes, [TerminalChange::Copied("second".into())], "one clipboard holds one thing");
    }

    #[test]
    fn a_copy_split_across_reads_is_heard_whole() {
        let stream: &[u8] = b"\x1b]52;c;aGVsbG8=\x07";
        for split in 1..stream.len() {
            let (changes, _) = heard(&[&stream[..split], &stream[split..]]);
            assert_eq!(changes, [TerminalChange::Copied("hello".into())], "split at {split}");
        }
    }

    #[test]
    fn a_copy_longer_than_an_osc_string_is_cut_with_the_rest() {
        // `ZWVl` is "eee", so the offer is text from end to end and the part of it that is left
        // is text as well.
        let long = [b"\x1b]52;c;".as_slice(), "ZWVl".repeat(2500).as_bytes(), b"\x07after"].concat();
        let (changes, screen) = heard(&[&long]);
        let [TerminalChange::Copied(text)] = changes.as_slice() else { panic!("{changes:?}") };
        assert_eq!(text.len(), (OSC_LIMIT - "52;c;".len()) * 3 / 4, "as much text as the limit leaves");
        assert!(text.bytes().all(|byte| byte == b'e'), "{text:?}");
        assert_eq!(screen, "after", "and the text after it still shows");
    }

    #[test]
    fn sequences_split_across_reads_are_heard_whole() {
        let stream: &[u8] = b"\x1b]0;title\x07\x1b]7;file://h/tmp\x1b\\\x1b]777;notify;T;B\x07\x07";
        for split in 1..stream.len() {
            let (changes, _) = heard(&[&stream[..split], &stream[split..]]);
            assert_eq!(
                changes,
                [
                    TerminalChange::Title("title".into()),
                    TerminalChange::WorkingFolder("/tmp".into()),
                    TerminalChange::Notify { title: Some("T".into()), body: "B".into() },
                    TerminalChange::Bell,
                ],
                "split at {split}"
            );
        }
    }

    #[test]
    fn a_bel_inside_an_osc_string_is_its_end_not_a_bell() {
        let (changes, _) = heard(&[b"\x1b]0;x\x07"]);
        assert!(!changes.contains(&TerminalChange::Bell));
    }

    #[test]
    fn an_endless_osc_string_is_cut_and_the_text_after_it_still_shows() {
        let mut limit = OscLimit::default();
        let mut passed = 0;
        limit.feed(b"\x1b]0;", |run| passed += run.len());
        for _ in 0..1000 {
            limit.feed(&[b'x'; 1024], |run| passed += run.len());
        }
        assert!(passed <= OSC_LIMIT + 4, "passed {passed} bytes of a megabyte");
        let long = [b"\x1b]0;".as_slice(), &[b'x'; 10_000], b"\x07after"].concat();
        let (changes, screen) = heard(&[&long]);
        assert_eq!(changes, [TerminalChange::Title("x".repeat(OSC_LIMIT - 2))]);
        assert_eq!(screen, "after");
    }

    #[test]
    fn notifications_keep_the_newest_few() {
        let stream: Vec<u8> = (0..20).flat_map(|n| format!("\x1b]9;n{n}\x07").into_bytes()).collect();
        let (changes, _) = heard(&[&stream]);
        assert_eq!(changes.len(), NOTIFY_LIMIT);
        assert_eq!(changes[0], TerminalChange::Notify { title: None, body: "n12".into() });
    }
}
