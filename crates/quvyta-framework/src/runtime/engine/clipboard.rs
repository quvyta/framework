//! Copying, pasting and reading the clipboard.

use std::time::Duration;

use super::Engine;
use crate::event::Event;
use crate::runtime::App;
use crate::runtime::clipboard::{ClipboardEvent, ReadStep};

/// What a clipboard read is for.
pub(super) enum ClipboardRead<Msg> {
    /// Paste the text into the focused widget, or offer it to the application.
    Paste,
    /// Deliver the text to the application, see [`Command::read_clipboard`](crate::runtime::Command::read_clipboard).
    Message(Box<dyn FnOnce(Option<String>) -> Msg>),
    /// Only learn whether there is text to paste, for Paste menu entries.
    Probe,
}

impl<A: App> Engine<A> {
    /// Offers pasted text to the focused widget, then to the application.
    pub(super) fn paste(&mut self, text: String, now: Duration) {
        let targets = self.keyboard_targets();
        self.dirty = true;
        if self.dispatch(&targets, &Event::Paste(text.clone()), now).is_none()
            && let Some(message) = self.app.clipboard(&ClipboardEvent::Pasted(text))
        {
            self.update(message);
        }
    }

    /// Remembers copied text and queues it for the terminal clipboard.
    pub(super) fn store_copy(&mut self, text: String) {
        self.clipboard_text = Some(text.clone());
        self.clipboard.push(text);
        self.interaction.can_paste = true;
    }

    /// Reads the clipboard for `read`: the system tool, then the terminal, then the text copied
    /// last inside the application; see [`ClipboardReader`]. Reads asked for while one runs
    /// share its result.
    pub(super) fn read_clipboard(&mut self, read: ClipboardRead<A::Msg>, now: Duration) {
        self.clipboard_reads.push(read);
        if self.clipboard_reader.is_reading() {
            return;
        }
        let step = self.clipboard_reader.start();
        self.clipboard_step(step, now);
    }

    pub(super) fn clipboard_step(&mut self, step: ReadStep, now: Duration) {
        match step {
            ReadStep::Waiting => {}
            ReadStep::AskTerminal => self.terminal_query = true,
            ReadStep::Done(text) => self.finish_clipboard(text, now),
        }
    }

    /// Checks on a clipboard read whose system tool runs on a thread.
    pub(crate) fn poll_clipboard(&mut self, now: Duration) {
        if self.clipboard_reader.is_reading() {
            let step = self.clipboard_reader.poll();
            self.clipboard_step(step, now);
        }
    }

    /// The terminal's answer to the OSC 52 clipboard query, `None` when it did not answer in time.
    pub(crate) fn terminal_clipboard(&mut self, text: Option<String>, now: Duration) {
        let step = self.clipboard_reader.terminal_answer(text);
        self.clipboard_step(step, now);
    }

    /// Delivers the text a clipboard read found, or the text copied last inside the application.
    pub(super) fn finish_clipboard(&mut self, text: Option<String>, now: Duration) {
        let text = text.or_else(|| self.clipboard_text.clone());
        self.interaction.can_paste = text.is_some();
        self.dirty = true;
        for read in std::mem::take(&mut self.clipboard_reads) {
            match read {
                ClipboardRead::Paste => {
                    if let Some(text) = &text {
                        self.paste(text.clone(), now);
                    }
                }
                ClipboardRead::Message(message) => {
                    let message = message(text.clone());
                    self.update(message);
                }
                ClipboardRead::Probe => {}
            }
        }
    }

    /// A copy made by a widget or the mouse selection: stored and reported to the application.
    pub(super) fn copy_from_ui(&mut self, text: String) {
        self.store_copy(text.clone());
        if let Some(message) = self.app.clipboard(&ClipboardEvent::Copied(text)) {
            self.update(message);
        }
    }
}
