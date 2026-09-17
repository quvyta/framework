//! Reading the terminal's clipboard: the OSC 52 query and picking its answer out of the input.

use std::io;
use std::time::Duration;

use crossterm::event as ct;
use crossterm::execute;

use super::app::App;
use super::clipboard::{OSC52_QUERY, decode_base64};
use super::engine::Engine;

/// How long the terminal gets to answer the OSC 52 clipboard query before the application's own
/// copy is used.
const OSC52_WAIT: Duration = Duration::from_millis(200);

/// How long after a query its answer is still recognised, so a late answer (a terminal that asked
/// the user first) is dropped instead of arriving as typed keys.
const OSC52_LATE: Duration = Duration::from_secs(30);

/// Asks the terminal for its clipboard when the engine's clipboard reader wants it, and picks the
/// answer out of the input.
#[derive(Default)]
pub(super) struct TerminalClipboard {
    /// When the last query was sent.
    asked: Option<Duration>,
    /// Whether the engine still waits for the answer.
    waiting: bool,
    reply: Osc52Reply,
}

impl TerminalClipboard {
    /// Sends a query the engine asked for, and gives up on an answer that takes too long.
    pub(super) fn update<A: App>(&mut self, engine: &mut Engine<A>, now: Duration) -> io::Result<()> {
        engine.poll_clipboard(now);
        if std::mem::take(&mut engine.terminal_query) {
            execute!(io::stdout(), crossterm::style::Print(OSC52_QUERY))?;
            self.asked = Some(now);
            self.waiting = true;
        }
        if self.waiting && self.asked.is_some_and(|asked| now >= asked + OSC52_WAIT) {
            self.waiting = false;
            engine.terminal_clipboard(None, now);
        }
        Ok(())
    }

    /// When the loop must wake up to stop waiting for an answer.
    pub(super) fn deadline(&self) -> Option<Duration> {
        self.asked.filter(|_| self.waiting).map(|asked| asked + OSC52_WAIT)
    }

    /// The events to handle for `event`: none while it belongs to an answer, the held events when
    /// what looked like an answer was not one. `more` tells whether more input is waiting: an
    /// answer arrives in one burst, a key typed by hand does not.
    pub(super) fn filter<A: App>(
        &mut self,
        event: ct::Event,
        more: bool,
        engine: &mut Engine<A>,
        now: Duration,
    ) -> Vec<ct::Event> {
        if self.asked.is_none_or(|asked| now >= asked + OSC52_LATE) && !self.reply.is_open() {
            return vec![event];
        }
        match self.reply.feed(event, more) {
            Feed::Pending => Vec::new(),
            Feed::Answer(text) => {
                if std::mem::take(&mut self.waiting) {
                    engine.terminal_clipboard(text, now);
                }
                Vec::new()
            }
            Feed::Pass(events) => events,
        }
    }
}

/// What an input event meant to an OSC 52 answer.
#[derive(Debug, PartialEq, Eq)]
enum Feed {
    /// Part of an answer, nothing to handle yet.
    Pending,
    /// The answer ended: the clipboard text, `None` when it was empty or unreadable.
    Answer(Option<String>),
    /// Not an answer: handle these events as input.
    Pass(Vec<ct::Event>),
}

/// Reassembles a terminal's OSC 52 answer, `ESC ] 52 ; c ; <base64> BEL` (or `ESC \`), from the
/// key events the input parser turns its bytes into: Alt+`]`, the characters, then Ctrl+G or
/// Alt+`\`.
#[derive(Debug, Default)]
struct Osc52Reply {
    /// Events of the introducer, handed back when it turns out not to be an answer.
    held: Vec<ct::Event>,
    /// Characters of the introducer and parameters matched so far.
    head: String,
    data: Option<String>,
}

impl Osc52Reply {
    fn is_open(&self) -> bool {
        !self.held.is_empty()
    }

    /// Takes the next input event; `more` tells whether more input is already waiting.
    fn feed(&mut self, event: ct::Event, more: bool) -> Feed {
        let ct::Event::Key(key) = &event else {
            return self.pass(event);
        };
        let alt = key.modifiers == ct::KeyModifiers::ALT;
        let ctrl = key.modifiers == ct::KeyModifiers::CONTROL;
        let plain = !key.modifiers.intersects(ct::KeyModifiers::ALT | ct::KeyModifiers::CONTROL);
        match (&mut self.data, key.code) {
            (None, ct::KeyCode::Char(']')) if alt && more && self.held.is_empty() => {
                self.held.push(event);
                Feed::Pending
            }
            (None, ct::KeyCode::Char(c)) if plain && !self.held.is_empty() => {
                self.head.push(c);
                self.held.push(event);
                // `52;` then the selection letters and `;`.
                if !"52;".starts_with(self.head.as_str()) && !self.head.starts_with("52;") {
                    return self.pass_held();
                }
                if self.head.len() > 3 && c == ';' {
                    self.data = Some(String::new());
                } else if self.head.len() > 3 && !c.is_ascii_alphanumeric() {
                    return self.pass_held();
                }
                Feed::Pending
            }
            (Some(_), ct::KeyCode::Char('g')) if ctrl => self.finish(),
            (Some(_), ct::KeyCode::Char('\\')) if alt => self.finish(),
            (Some(data), ct::KeyCode::Char(c)) if plain => {
                data.push(c);
                Feed::Pending
            }
            (Some(_), _) => {
                // Anything else ends a broken answer; the event itself is input.
                *self = Self::default();
                Feed::Pass(vec![event])
            }
            _ => self.pass(event),
        }
    }

    fn finish(&mut self) -> Feed {
        let data = std::mem::take(self).data.unwrap_or_default();
        Feed::Answer(decode_base64(&data).filter(|text| !text.is_empty()))
    }

    fn pass(&mut self, event: ct::Event) -> Feed {
        let mut events = std::mem::take(self).held;
        events.push(event);
        Feed::Pass(events)
    }

    fn pass_held(&mut self) -> Feed {
        Feed::Pass(std::mem::take(self).held)
    }
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
                '\x07' => (ct::KeyCode::Char('g'), ct::KeyModifiers::CONTROL),
                c if c.is_uppercase() => (ct::KeyCode::Char(c), ct::KeyModifiers::SHIFT),
                c => (ct::KeyCode::Char(c), ct::KeyModifiers::NONE),
            };
            events.push(ct::Event::Key(ct::KeyEvent::new(code, mods)));
        }
        events
    }

    fn feed_all(reply: &mut Osc52Reply, events: Vec<ct::Event>) -> Vec<Feed> {
        let count = events.len();
        events.into_iter().enumerate().map(|(index, event)| reply.feed(event, index + 1 < count)).collect()
    }

    #[test]
    fn picks_the_osc52_answer_out_of_the_input() {
        let mut reply = Osc52Reply::default();
        let feeds = feed_all(&mut reply, keys("\x1b]52;c;ZGVwbG95LWFwaQ==\x07"));
        assert!(feeds[..feeds.len() - 1].iter().all(|feed| *feed == Feed::Pending), "{feeds:?}");
        assert_eq!(feeds.last(), Some(&Feed::Answer(Some("deploy-api".into()))));
        let feeds = feed_all(&mut reply, keys("\x1b]52;c;w6dheQ==\x1b\\"));
        assert_eq!(feeds.last(), Some(&Feed::Answer(Some("çay".into()))), "ST ends it as well as BEL");
        let feeds = feed_all(&mut reply, keys("\x1b]52;c;\x07"));
        assert_eq!(feeds.last(), Some(&Feed::Answer(None)), "an empty clipboard");
    }

    #[test]
    fn keys_that_only_look_like_an_answer_are_handed_back() {
        let mut reply = Osc52Reply::default();
        let feeds = feed_all(&mut reply, keys("\x1b]5x"));
        assert_eq!(feeds[..2], [Feed::Pending, Feed::Pending]);
        assert_eq!(feeds[2], Feed::Pass(keys("\x1b]5x")), "alt+] 5 x is typed input");
        let alone = keys("\x1b]").remove(0);
        assert_eq!(reply.feed(alone.clone(), false), Feed::Pass(vec![alone]), "a lone alt+] is a key");
        let plain = keys("q").remove(0);
        assert_eq!(reply.feed(plain.clone(), true), Feed::Pass(vec![plain]));
    }
}
