//! Handing the terminal to another program for a while.
//!
//! An application that draws on the alternate screen in raw mode cannot show another program's
//! prompts: `sudo` writes its password prompt to the controlling terminal, our screen swallows
//! it, and both programs read the same keys. The answer is to step aside: leave raw mode and the
//! alternate screen, let the program own the terminal, then take the screen back and draw
//! everything again. The program gets a process group of its own for the time, so the keys'
//! signals reach it and not the application.
//!
//! [`Handoff`] describes such a step aside, and [`run`] carries it out with the screen the caller
//! hands it: the real terminal in [`Runtime`](super::Runtime), a recording stand-in in tests.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::process::{Command as Child, Stdio};

/// Hands the terminal to another program: leaves raw mode and the alternate screen, runs it
/// attached to the real terminal, then takes the screen back and draws everything again.
///
/// The program runs on the drawing thread and the application waits for it, which is what the
/// user expects: they are talking to another program. Background tasks keep running, but nothing
/// is drawn until the program ends.
///
/// On Unix, when the application is the foreground of its controlling terminal, the program
/// runs in a process group of its own that is made the terminal's foreground until it ends, as
/// a shell runs a job. The signals of the keys — `Ctrl-C` at a `sudo` prompt, `Ctrl-\` — and
/// resizes then reach the program and its children only: the application is never ended by
/// them, and its own signal dispositions are never changed. `Ctrl-Z` does not suspend: a
/// program that stops is continued at once, since the application offers no way back to it. A
/// process group is not a session, so the program keeps the controlling terminal and the
/// session a warm `sudo` ticket is kept for.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::runtime::{Handoff, HandoffOutcome};
///
/// enum Msg {
///     Authorize,
///     Authorized(HandoffOutcome),
/// }
///
/// fn update(msg: Msg) -> Command<Msg> {
///     match msg {
///         // `sudo -v` asks for the password itself, on the terminal it owns for those seconds.
///         Msg::Authorize => Command::handoff(
///             Handoff::new("sudo", Msg::Authorized).arg("-v").notice("Authorizing the installation…"),
///         ),
///         Msg::Authorized(_) => Command::none(),
///     }
/// }
/// ```
pub struct Handoff<Msg> {
    program: OsString,
    args: Vec<OsString>,
    dir: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    notice: Option<String>,
    pause: bool,
    on_finish: Box<dyn FnOnce(HandoffOutcome) -> Msg + Send>,
}

impl<Msg: Send + 'static> Handoff<Msg> {
    /// Runs `program`, delivering `on_finish(outcome)` once the application has the screen back.
    pub fn new(program: impl Into<OsString>, on_finish: impl FnOnce(HandoffOutcome) -> Msg + Send + 'static) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            dir: None,
            env: Vec::new(),
            notice: None,
            pause: false,
            on_finish: Box::new(on_finish),
        }
    }

    /// Adds one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds several arguments, in order.
    #[must_use]
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Runs the program in `dir` instead of the application's working directory.
    #[must_use]
    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = Some(dir.into());
        self
    }

    /// Sets an environment variable for the program. The rest of the environment is inherited.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// A line printed on the cleared screen before the program starts, so the user knows why the
    /// application stepped aside.
    #[must_use]
    pub fn notice(mut self, text: impl Into<String>) -> Self {
        self.notice = Some(text.into());
        self
    }

    /// Waits for a key press after the program ends, so its last output can be read. Off by
    /// default: a program that only takes a moment, such as `sudo -v`, has nothing to read.
    #[must_use]
    pub fn pause(mut self, pause: bool) -> Self {
        self.pause = pause;
        self
    }

    /// What a test sees of this handoff.
    pub(crate) fn request(&self) -> HandoffRequest {
        HandoffRequest {
            program: self.program.clone(),
            args: self.args.clone(),
            notice: self.notice.clone(),
            pause: self.pause,
        }
    }

    /// The message of `outcome`, for a harness that never runs the program.
    pub(crate) fn finish(self, outcome: HandoffOutcome) -> Msg {
        (self.on_finish)(outcome)
    }

    /// The same handoff delivering `map(message)` once the application has the screen back.
    pub(crate) fn map<B>(self, map: impl FnOnce(Msg) -> B + Send + 'static) -> Handoff<B> {
        let on_finish = self.on_finish;
        Handoff {
            program: self.program,
            args: self.args,
            dir: self.dir,
            env: self.env,
            notice: self.notice,
            pause: self.pause,
            on_finish: Box::new(move |outcome| map(on_finish(outcome))),
        }
    }
}

/// How a handoff ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffOutcome {
    /// The program ran; `code` is `None` when a signal ended it.
    Finished {
        /// The exit code, or `None` after a signal such as an interrupt.
        code: Option<i32>,
    },
    /// The program could not be started, or the terminal could not be restored.
    Failed(String),
}

/// A handoff a [`Harness`](super::Harness) recorded instead of running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffRequest {
    /// The program asked for.
    pub program: OsString,
    /// Its arguments, in order.
    pub args: Vec<OsString>,
    /// The line [`Handoff::notice`] would have printed.
    pub notice: Option<String>,
    /// Whether [`Handoff::pause`] was turned on.
    pub pause: bool,
}

/// What a handoff does to the terminal around the program. The terminal runtime passes the real
/// screen; tests pass closures that record instead.
pub(crate) struct HandoffScreen<'a> {
    /// Leaves application mode, clears the screen and prints the notice, if any. The program
    /// starts only when this succeeds.
    pub(crate) release: &'a mut dyn FnMut(Option<&str>) -> io::Result<()>,
    /// Takes the screen back and draws the whole application again. Runs however the program
    /// ended, so the terminal is never left behind.
    pub(crate) take: &'a mut dyn FnMut() -> io::Result<()>,
    /// Waits for one key press, for [`Handoff::pause`].
    pub(crate) wait_for_key: &'a mut dyn FnMut() -> io::Result<()>,
}

/// Runs `handoff` on the calling thread and returns the message of its outcome.
///
/// The program inherits the standard streams, so it reads the keyboard itself, and it stays in
/// this process's session: `sudo` keeps its ticket per controlling terminal, and a program of its
/// own session would not be given it. Its own process group, which the keys' signals go to, is
/// arranged by `foreground`.
pub(crate) fn run<Msg: Send + 'static>(handoff: Handoff<Msg>, screen: &mut HandoffScreen<'_>) -> Msg {
    let outcome = match (screen.release)(handoff.notice.as_deref()) {
        Ok(()) => {
            let outcome = spawn(&handoff);
            // The screen is still the program's; waiting here lets its last lines be read.
            if handoff.pause && matches!(outcome, HandoffOutcome::Finished { .. }) {
                let _ = (screen.wait_for_key)();
            }
            match (screen.take)() {
                Ok(()) => outcome,
                Err(error) => HandoffOutcome::Failed(error.to_string()),
            }
        }
        Err(error) => {
            // Application mode may be half gone; taking the screen back puts it right.
            let _ = (screen.take)();
            HandoffOutcome::Failed(error.to_string())
        }
    };
    handoff.finish(outcome)
}

/// Starts the program attached to the terminal and waits for it.
fn spawn<Msg>(handoff: &Handoff<Msg>) -> HandoffOutcome {
    let mut child = Child::new(&handoff.program);
    child.args(&handoff.args).stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    if let Some(dir) = &handoff.dir {
        child.current_dir(dir);
    }
    for (key, value) in &handoff.env {
        child.env(key, value);
    }
    #[cfg(unix)]
    let status = super::foreground::status(&mut child);
    #[cfg(not(unix))]
    let status = child.status();
    match status {
        Ok(status) => HandoffOutcome::Finished { code: status.code() },
        Err(error) => HandoffOutcome::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `handoff` against a stand-in screen and returns its outcome and, in order, what it
    /// did to that screen. With `release_fails` the screen cannot be left, as one without a
    /// terminal behind it cannot.
    fn run_with(handoff: Handoff<HandoffOutcome>, release_fails: bool) -> (HandoffOutcome, Vec<String>) {
        let steps = std::cell::RefCell::new(Vec::new());
        let mut release = |notice: Option<&str>| -> io::Result<()> {
            steps.borrow_mut().push(match notice {
                Some(text) => format!("release {text}"),
                None => "release".to_owned(),
            });
            if release_fails { Err(io::Error::other("no terminal")) } else { Ok(()) }
        };
        let mut take = || -> io::Result<()> {
            steps.borrow_mut().push("take".to_owned());
            Ok(())
        };
        let mut wait_for_key = || -> io::Result<()> {
            steps.borrow_mut().push("key".to_owned());
            Ok(())
        };
        let outcome = run(
            handoff,
            &mut HandoffScreen { release: &mut release, take: &mut take, wait_for_key: &mut wait_for_key },
        );
        (outcome, steps.into_inner())
    }

    /// A handoff whose message is the outcome itself, running `script` through `sh`.
    fn shell(script: &str) -> Handoff<HandoffOutcome> {
        Handoff::new("sh", |outcome| outcome).arg("-c").arg(script)
    }

    #[test]
    fn the_screen_is_released_around_the_program_and_taken_back() {
        let (outcome, steps) = run_with(shell("exit 0").notice("Installing packages…"), false);
        assert_eq!(outcome, HandoffOutcome::Finished { code: Some(0) });
        assert_eq!(steps, ["release Installing packages…", "take"], "the program ran while the screen was released");
    }

    #[test]
    fn the_exit_code_reaches_the_message() {
        let (zero, _) = run_with(shell("exit 0"), false);
        assert_eq!(zero, HandoffOutcome::Finished { code: Some(0) });
        let (seven, _) = run_with(shell("exit 7"), false);
        assert_eq!(seven, HandoffOutcome::Finished { code: Some(7) });
        // A signal leaves no exit code, so an interrupted program is `None` rather than a number.
        let (signal, _) = run_with(shell("kill -TERM $$"), false);
        assert_eq!(signal, HandoffOutcome::Finished { code: None });
    }

    #[test]
    fn arguments_the_directory_and_the_environment_reach_the_program() {
        let (outcome, _) = run_with(
            Handoff::new("sh", |outcome| outcome)
                .args(["-c", r#"test "$(pwd)" = / && test "$QUVYTA_HANDOFF_TEST" = ok"#])
                .dir("/")
                .env("QUVYTA_HANDOFF_TEST", "ok"),
            false,
        );
        assert_eq!(outcome, HandoffOutcome::Finished { code: Some(0) });
    }

    #[test]
    fn an_unstartable_program_fails_and_the_screen_still_comes_back() {
        let handoff = Handoff::new("quvyta-no-such-program", |outcome| outcome);
        let (outcome, steps) = run_with(handoff, false);
        let HandoffOutcome::Failed(reason) = outcome else {
            panic!("a program that is not there cannot have finished: {outcome:?}");
        };
        assert!(!reason.is_empty(), "the reason names what went wrong");
        assert_eq!(steps, ["release", "take"], "the terminal is taken back even so");
    }

    #[test]
    fn a_terminal_that_cannot_be_released_fails_without_running_the_program() {
        let handoff = shell("exit 0");
        let (outcome, steps) = run_with(handoff, true);
        assert_eq!(outcome, HandoffOutcome::Failed("no terminal".to_owned()));
        assert_eq!(steps, ["release", "take"], "application mode is put back");
    }

    #[test]
    fn pause_waits_for_a_key_only_when_it_is_asked_for() {
        let (_, waited) = run_with(shell("exit 0").pause(true), false);
        assert_eq!(waited, ["release", "key", "take"], "the key is awaited before the screen is taken back");
        let (_, quiet) = run_with(shell("exit 0").pause(false), false);
        assert_eq!(quiet, ["release", "take"]);
        let (_, missing) = run_with(Handoff::new("quvyta-no-such-program", |o| o).pause(true), false);
        assert_eq!(missing, ["release", "take"], "a program that never ran leaves nothing to read");
    }

    #[test]
    fn the_request_a_harness_records_carries_the_program_and_its_options() {
        let handoff = shell("less /etc/hostname").notice("Reading").pause(true);
        let request = handoff.request();
        assert_eq!(request.program, OsString::from("sh"));
        assert_eq!(request.args, ["-c", "less /etc/hostname"].map(OsString::from));
        assert_eq!(request.notice.as_deref(), Some("Reading"));
        assert!(request.pause);
    }
}
