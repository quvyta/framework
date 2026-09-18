//! Ending because the system asked: the causes an application hears, and the one rule that
//! decides what each further signal of a run does.

use std::time::Duration;

/// Why the system, rather than the user, is ending the application.
///
/// On Unix the terminal [`Runtime`](super::Runtime) catches `SIGTERM`, `SIGINT` and `SIGHUP` for
/// as long as it runs, and a [`Harness`](super::Harness) simulates them with
/// [`Harness::terminate`](super::Harness::terminate). The application hears the cause through
/// [`App::terminating`](super::App::terminating), which is how it tells "the system is ending
/// us" from "the user asked to quit" ([`App::before_quit`](super::App::before_quit)).
///
/// Whatever the application answers, the run ends in bounded time:
///
/// - Answering `None` quits at once.
/// - A message keeps the application running while it finishes, e.g. saves and returns
///   [`Command::quit`](super::Command::quit). After [`Termination::grace`] the runtime quits
///   without it.
/// - A second `SIGTERM` or `SIGINT` ends the run at once. When the application does not return
///   to the loop within a second after that, or after its grace, the runtime restores the
///   terminal itself and the process ends by the signal, as it would have without the
///   framework.
///
/// Every quit restores the terminal (raw mode off, the normal screen, the cursor) as long as the
/// terminal still exists; after a hangup nothing is written to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Termination {
    /// `SIGTERM`, or `SIGINT` from `kill -INT`: another program or the system (a service
    /// manager, a shutdown, `kill`) asks the application to end. The terminal is still there, so
    /// the application may save and quit, and it may even ask the user something, within the
    /// grace. By default the application answers as for a quit the user asked for, with
    /// [`App::before_quit`](super::App::before_quit).
    ///
    /// Inside the application `ctrl c` is a key, not this signal: raw mode turns the terminal's
    /// interrupt key off.
    Terminate,
    /// `SIGHUP`: the terminal went away, because the SSH connection dropped, the terminal
    /// window or the `tmux` pane closed. Nobody can answer a question any more and nothing is
    /// drawn after it, so the application gets one chance to save, without a dialog. By default
    /// it quits at once.
    ///
    /// A hangup that repeats (the shell forwards its own and the system sends another when the
    /// shell ends) changes nothing. A `SIGHUP` sent by hand while the terminal is still there is
    /// heard the same way, and the terminal is restored on the way out.
    Hangup,
}

impl Termination {
    /// How long the application has, after this cause, to quit on its own before the runtime
    /// quits without it: five seconds after [`Termination::Terminate`], three after
    /// [`Termination::Hangup`]. A hangup during a pending terminate shortens what is left to
    /// the hangup's grace.
    ///
    /// ```
    /// use std::time::Duration;
    /// use qframe::runtime::Termination;
    ///
    /// assert_eq!(Termination::Terminate.grace(), Duration::from_secs(5));
    /// assert_eq!(Termination::Hangup.grace(), Duration::from_secs(3));
    /// ```
    #[must_use]
    pub const fn grace(self) -> Duration {
        match self {
            Self::Terminate => Duration::from_secs(5),
            Self::Hangup => Duration::from_secs(3),
        }
    }
}

/// A termination the application was told about and has not finished yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ending {
    /// The cause the application heard last.
    pub(crate) cause: Termination,
    /// When the run quits even if the application has not.
    pub(crate) deadline: Duration,
}

/// What a run does with one more signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Tell the application, through `App::terminating`, with this cause.
    Ask(Termination),
    /// Nothing new: a hangup repeated.
    Ignore,
    /// End the run now.
    End,
}

/// Moves `ending` along for a signal of `cause` arriving at `now`. The engine and the signal
/// watcher both follow this rule, each with its own clock, so they agree on every step.
///
/// The first signal asks. A hangup after a terminate asks again, since the answer to the
/// terminate may be a question nobody can see now, and leaves the shorter grace. A hangup after
/// a hangup is the same hangup heard twice. A terminate after anything is the user or the system
/// insisting, and ends the run.
pub(crate) fn receive(ending: &mut Option<Ending>, cause: Termination, now: Duration) -> Step {
    match (*ending, cause) {
        (None, _) => {
            *ending = Some(Ending { cause, deadline: now + cause.grace() });
            Step::Ask(cause)
        }
        (Some(Ending { cause: Termination::Hangup, .. }), Termination::Hangup) => Step::Ignore,
        (Some(current), Termination::Hangup) => {
            let deadline = current.deadline.min(now + Termination::Hangup.grace());
            *ending = Some(Ending { cause, deadline });
            Step::Ask(cause)
        }
        (Some(_), Termination::Terminate) => Step::End,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn the_first_signal_asks_and_starts_its_grace() {
        let mut ending = None;
        assert_eq!(receive(&mut ending, Termination::Terminate, SECOND), Step::Ask(Termination::Terminate));
        assert_eq!(ending, Some(Ending { cause: Termination::Terminate, deadline: SECOND * 6 }));
        let mut ending = None;
        assert_eq!(receive(&mut ending, Termination::Hangup, SECOND), Step::Ask(Termination::Hangup));
        assert_eq!(ending, Some(Ending { cause: Termination::Hangup, deadline: SECOND * 4 }));
    }

    #[test]
    fn a_second_terminate_ends_whatever_came_first() {
        for first in [Termination::Terminate, Termination::Hangup] {
            let mut ending = None;
            receive(&mut ending, first, Duration::ZERO);
            assert_eq!(receive(&mut ending, Termination::Terminate, SECOND), Step::End, "after {first:?}");
        }
    }

    #[test]
    fn a_repeated_hangup_changes_nothing() {
        let mut ending = None;
        receive(&mut ending, Termination::Hangup, Duration::ZERO);
        let before = ending;
        assert_eq!(receive(&mut ending, Termination::Hangup, SECOND), Step::Ignore);
        assert_eq!(ending, before, "the deadline stays where the first hangup put it");
    }

    #[test]
    fn a_hangup_during_a_terminate_asks_again_with_the_shorter_grace() {
        let mut ending = None;
        receive(&mut ending, Termination::Terminate, Duration::ZERO);
        assert_eq!(receive(&mut ending, Termination::Hangup, SECOND), Step::Ask(Termination::Hangup));
        assert_eq!(ending, Some(Ending { cause: Termination::Hangup, deadline: SECOND * 4 }));
        // Late in the terminate's grace, what is left of it is shorter than a hangup's.
        let mut ending = None;
        receive(&mut ending, Termination::Terminate, Duration::ZERO);
        receive(&mut ending, Termination::Hangup, SECOND * 4);
        assert_eq!(ending.map(|ending| ending.deadline), Some(SECOND * 5));
    }
}
