//! Ending because the system asked: telling the application, and quitting when its grace is
//! over.

use std::time::Duration;

use super::Engine;
use crate::i18n;
use crate::runtime::App;
use crate::runtime::termination::{self, Step, Termination};

impl<A: App> Engine<A> {
    /// Hears a signal of `cause` at `now`. The first one of a run, and a hangup during a
    /// terminate, reach [`App::terminating`]: `None` quits, a message is applied and the
    /// application has until the grace is over. A repeated hangup changes nothing; a terminate
    /// while one is pending quits.
    pub(crate) fn terminate(&mut self, cause: Termination, now: Duration) {
        if self.quit {
            return;
        }
        self.clock = now;
        match termination::receive(&mut self.ending, cause, now) {
            Step::Ignore => {}
            Step::End => self.quit = true,
            Step::Ask(cause) => {
                let message = {
                    let app = &self.app;
                    i18n::scope(self.env.i18n_arc(), || app.terminating(cause))
                };
                match message {
                    Some(message) => self.update(message),
                    None => self.quit = true,
                }
            }
        }
    }

    /// When the run quits even if the application has not, while a termination is pending.
    pub(crate) fn ending_deadline(&self) -> Option<Duration> {
        self.ending.map(|ending| ending.deadline)
    }

    /// Quits when the grace of a pending termination is over at `now`.
    pub(crate) fn end_when_due(&mut self, now: Duration) {
        if self.ending_deadline().is_some_and(|deadline| deadline <= now) {
            self.quit = true;
        }
    }
}
