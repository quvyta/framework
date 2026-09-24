//! Asking the terminal which pictures it can show, as the runtime starts, without its answers
//! ever reaching the application as keys.
//!
//! The question is written right after the terminal enters raw mode, before the input parser
//! has read a single byte from it. The answers are then read straight from the terminal device,
//! with a bounded wait, up to the device attributes that end them, so the parser never sees them.
//! A terminal that answers after the wait is over has its kitty answer picked out of the input by
//! [`LateAnswer`], and a kitty `OK` that arrives so still turns pictures to kitty from then on;
//! the parser drops a late device attributes answer by itself.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event as ct;

#[cfg(unix)]
use crate::graphics::{self, Graphics};

/// How long the terminal gets to answer. Every terminal answers the device attributes request,
/// and its answer ends the wait, so a local terminal answers in a few milliseconds. A terminal that
/// never answers holds the first frame for this long at most, so it stays short: starting never
/// waits on the network. A slow link's kitty answer that comes after it is still heard through
/// [`LateAnswer`] and turns pictures to kitty from the next frame on.
#[cfg(unix)]
pub(super) const PROBE_WAIT: Duration = Duration::from_millis(150);

/// How long after a probe that went unanswered a kitty answer is still picked out of the input.
#[cfg(any(unix, test))]
pub(super) const LATE: Duration = Duration::from_secs(10);

/// The most bytes read while waiting for the answers: both fit in well under a hundred.
#[cfg(unix)]
const MOST: usize = 4096;

/// What the probe found.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Probe {
    /// The terminal's answer, [`Graphics::HalfBlock`] when it gave none.
    pub(super) graphics: Graphics,
    /// Whether the answers ended with the device attributes before the wait was over. When not,
    /// a kitty answer may still be on its way.
    pub(super) answered: bool,
}

/// Writes [`graphics::QUERY`] to `out` and reads the answers from `tty` for up to `wait`.
///
/// # Errors
///
/// Returns an I/O error when the question cannot be written or the terminal cannot be read.
#[cfg(unix)]
pub(super) fn probe(tty: std::os::fd::BorrowedFd<'_>, out: &mut impl io::Write, wait: Duration) -> io::Result<Probe> {
    out.write_all(graphics::QUERY.as_bytes())?;
    out.flush()?;
    let replies = read_replies(tty, wait)?;
    Ok(Probe { graphics: graphics::classify(&replies), answered: graphics::answered(&replies) })
}

/// Reads what `tty` has to say for up to `wait`, stopping as soon as the device attributes are in
/// or the other end closed. Waits for nothing more once the time is up, however the reads went.
#[cfg(unix)]
fn read_replies(tty: std::os::fd::BorrowedFd<'_>, wait: Duration) -> io::Result<Vec<u8>> {
    use rustix::event::{PollFd, PollFlags, Timespec, poll};

    let deadline = Instant::now() + wait;
    let mut replies = Vec::new();
    let mut chunk = [0_u8; 256];
    while !graphics::answered(&replies) && replies.len() < MOST {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let limit = Timespec::try_from(left).map_err(|_| io::Error::other("wait out of range"))?;
        let mut fds = [PollFd::new(&tty, PollFlags::IN)];
        match poll(&mut fds, Some(&limit)) {
            Ok(0) | Err(rustix::io::Errno::INTR) => continue,
            Ok(_) => {}
            Err(error) => return Err(error.into()),
        }
        let revents = fds[0].revents();
        if !revents.contains(PollFlags::IN) {
            // Hung up or broken with nothing left to read: no answer is coming.
            break;
        }
        match rustix::io::read(tty, &mut chunk) {
            Ok(0) => break,
            Ok(read) => replies.extend_from_slice(&chunk[..read]),
            Err(rustix::io::Errno::INTR | rustix::io::Errno::AGAIN) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(replies)
}

/// Picks a kitty answer that arrived after the probe stopped waiting out of the input, where the
/// parser turns `ESC _ G i=31;OK ESC \` into Alt+`_`, the characters, then Alt+`\`.
#[derive(Debug, Default)]
pub(super) struct LateAnswer {
    /// Until when an answer is still expected; `None` when none is.
    until: Option<Instant>,
    /// Events of an answer so far, handed back when it turns out not to be one.
    held: Vec<ct::Event>,
    /// The characters after the introducer.
    text: String,
    /// Whether an answer picked out said `OK`: the terminal draws kitty pictures after all.
    kitty: bool,
}

impl LateAnswer {
    /// Expects a late answer until `until`.
    #[cfg(any(unix, test))]
    pub(super) fn until(until: Instant) -> Self {
        Self { until: Some(until), ..Self::default() }
    }

    /// The events to handle for `event`: none while it belongs to an answer, the held ones and
    /// itself when what looked like an answer was not one. `more` tells whether more input is
    /// waiting: an answer arrives in one burst, a key typed by hand does not.
    pub(super) fn filter(&mut self, event: ct::Event, more: bool, now: Instant) -> Vec<ct::Event> {
        if self.held.is_empty() && self.until.is_none_or(|until| now >= until) {
            self.until = None;
            return vec![event];
        }
        let ct::Event::Key(key) = &event else {
            return self.pass(event);
        };
        let alt = key.modifiers == ct::KeyModifiers::ALT;
        let plain = !key.modifiers.intersects(ct::KeyModifiers::ALT | ct::KeyModifiers::CONTROL);
        match key.code {
            ct::KeyCode::Char('_') if alt && more && self.held.is_empty() => {
                self.held.push(event);
                Vec::new()
            }
            ct::KeyCode::Char('\\') if alt && self.text.starts_with("Gi=31") => {
                // The one answer the probe asked for; nothing more is expected.
                let kitty = self.text.ends_with(";OK");
                *self = Self { kitty, ..Self::default() };
                Vec::new()
            }
            ct::KeyCode::Char(c) if plain && !self.held.is_empty() && self.text.len() < 256 => {
                self.text.push(c);
                self.held.push(event);
                if "Gi=31".starts_with(self.text.as_str()) || self.text.starts_with("Gi=31") {
                    Vec::new()
                } else {
                    self.pass_held()
                }
            }
            _ => self.pass(event),
        }
    }

    /// Whether a late answer said the terminal draws kitty pictures; `true` once, so the caller
    /// switches when it hears it.
    pub(super) fn take_kitty(&mut self) -> bool {
        std::mem::take(&mut self.kitty)
    }

    fn pass(&mut self, event: ct::Event) -> Vec<ct::Event> {
        let mut events = self.pass_held();
        events.push(event);
        events
    }

    fn pass_held(&mut self) -> Vec<ct::Event> {
        self.text.clear();
        std::mem::take(&mut self.held)
    }
}

/// When a probe that did not finish still expects an answer: [`LATE`] from `now`.
#[cfg(any(unix, test))]
pub(super) fn late_from(now: Instant) -> LateAnswer {
    LateAnswer::until(now + LATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The key events the input parser makes of `bytes` sent by a terminal.
    fn keys(bytes: &str) -> Vec<ct::Event> {
        let mut events = Vec::new();
        let mut chars = bytes.chars();
        while let Some(c) = chars.next() {
            let (code, mods) = match c {
                '\x1b' => (chars.next().map_or(ct::KeyCode::Esc, ct::KeyCode::Char), ct::KeyModifiers::ALT),
                c if c.is_uppercase() => (ct::KeyCode::Char(c), ct::KeyModifiers::SHIFT),
                c => (ct::KeyCode::Char(c), ct::KeyModifiers::NONE),
            };
            events.push(ct::Event::Key(ct::KeyEvent::new(code, mods)));
        }
        events
    }

    /// What reaches the application of `events` read in one burst.
    fn handled(late: &mut LateAnswer, events: Vec<ct::Event>, now: Instant) -> Vec<ct::Event> {
        let count = events.len();
        events.into_iter().enumerate().flat_map(|(index, event)| late.filter(event, index + 1 < count, now)).collect()
    }

    #[test]
    fn a_late_kitty_answer_never_reaches_the_application() {
        let now = Instant::now();
        let mut late = late_from(now);
        assert!(handled(&mut late, keys("\x1b_Gi=31;OK\x1b\\"), now).is_empty());
        let mut late = late_from(now);
        let refused = handled(&mut late, keys("\x1b_Gi=31;ENOTSUPPORTED:not here\x1b\\"), now);
        assert!(refused.is_empty(), "a refusal is an answer too: {refused:?}");
    }

    #[test]
    fn a_late_kitty_ok_says_the_terminal_draws_kitty_pictures_once() {
        let now = Instant::now();
        let mut late = late_from(now);
        assert!(!late.take_kitty(), "nothing heard yet");
        assert!(handled(&mut late, keys("\x1b_Gi=31;OK\x1b\\"), now).is_empty());
        assert!(late.take_kitty(), "the OK is heard");
        assert!(!late.take_kitty(), "and only once");
        let mut late = late_from(now);
        handled(&mut late, keys("\x1b_Gi=31;ENOTSUPPORTED:not here\x1b\\"), now);
        assert!(!late.take_kitty(), "a refusal is not kitty");
    }

    #[test]
    fn keys_that_only_look_like_an_answer_are_handed_back() {
        let now = Instant::now();
        let mut late = late_from(now);
        let typed = keys("\x1b_Gx");
        assert_eq!(handled(&mut late, typed.clone(), now), typed, "not image 31");
        let alone = keys("\x1b_");
        assert_eq!(handled(&mut late, alone.clone(), now), alone, "alt _ typed by hand arrives alone");
        let plain = keys("hello");
        assert_eq!(handled(&mut late, plain.clone(), now), plain);
    }

    #[test]
    fn once_the_time_is_over_an_answer_is_input_again() {
        let now = Instant::now();
        let mut late = late_from(now);
        let answer = keys("\x1b_Gi=31;OK\x1b\\");
        assert_eq!(handled(&mut late, answer.clone(), now + LATE), answer);
        let mut none = LateAnswer::default();
        assert_eq!(handled(&mut none, answer.clone(), now), answer, "a probe that finished expects nothing");
    }

    #[cfg(unix)]
    mod unix {
        use std::io::{Read, Write};
        use std::os::fd::AsFd;

        use super::super::*;

        /// A pipe stands in for a terminal: nothing here ever talks to the real one.
        #[test]
        fn a_terminal_that_never_answers_gives_half_blocks_within_the_wait() {
            let (reader, writer) = std::io::pipe().expect("a pipe");
            let mut question = Vec::new();
            let started = Instant::now();
            let found = probe(reader.as_fd(), &mut question, PROBE_WAIT).expect("the probe runs");
            let took = started.elapsed();
            assert_eq!(found, Probe { graphics: Graphics::HalfBlock, answered: false });
            assert!(took >= PROBE_WAIT, "it waited the whole time: {took:?}");
            assert!(took < Duration::from_secs(10), "and not much longer: {took:?}");
            assert_eq!(question, graphics::QUERY.as_bytes(), "the question went out");
            drop(writer);
        }

        #[test]
        fn a_terminal_that_answers_is_read_up_to_its_attributes() {
            for (answer, graphics) in [
                (&b"\x1b_Gi=31;OK\x1b\\\x1b[?62;c"[..], Graphics::Kitty),
                (b"\x1b[?62;4;22c", Graphics::Sixel),
                (b"\x1b[?62;22c", Graphics::HalfBlock),
            ] {
                let (reader, mut writer) = std::io::pipe().expect("a pipe");
                writer.write_all(answer).expect("the answer");
                writer.write_all(b"typed later").expect("a key after it");
                let started = Instant::now();
                // Far longer than a test should wait: an answer that ends must end the read.
                let found = probe(reader.as_fd(), &mut Vec::new(), Duration::from_secs(60)).expect("the probe runs");
                assert_eq!(found, Probe { graphics, answered: true }, "{answer:?}");
                assert!(started.elapsed() < Duration::from_secs(30), "the attributes ended the wait");
            }
        }

        #[test]
        fn a_slow_link_does_not_hold_the_first_frame_and_its_late_kitty_answer_still_counts() {
            let (mut reader, mut writer) = std::io::pipe().expect("a pipe");
            let answering = std::thread::spawn(move || {
                // A long SSH round trip: the answers come 600 ms after the question.
                std::thread::sleep(Duration::from_millis(600));
                writer.write_all(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;c").expect("the answer");
            });
            let found = probe(reader.as_fd(), &mut Vec::new(), PROBE_WAIT).expect("the probe runs");
            // Had the probe waited for the answer, it would have read kitty here.
            assert_eq!(
                found,
                Probe { graphics: Graphics::HalfBlock, answered: false },
                "the first frame is drawn with half blocks, without waiting for the link"
            );
            answering.join().expect("the answering side");
            let mut late_bytes = Vec::new();
            reader.read_to_end(&mut late_bytes).expect("the late answer");
            let now = Instant::now();
            let mut late = late_from(now);
            super::handled(&mut late, super::keys(&String::from_utf8(late_bytes).expect("text")), now);
            assert!(late.take_kitty(), "the answer that came late turns pictures to kitty");
        }

        #[test]
        fn a_terminal_that_goes_away_ends_the_wait() {
            let (reader, writer) = std::io::pipe().expect("a pipe");
            drop(writer);
            let started = Instant::now();
            let found = probe(reader.as_fd(), &mut Vec::new(), Duration::from_secs(60)).expect("the probe runs");
            assert_eq!(found.graphics, Graphics::HalfBlock);
            assert!(started.elapsed() < Duration::from_secs(30));
        }
    }
}
