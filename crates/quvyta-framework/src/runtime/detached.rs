//! Handing the terminal to a program only until it is ready, and leaving it running.
//!
//! Some programs ask the user something on the terminal and then keep working in the
//! background: `pkexec` asks for a password and becomes the privileged helper it started, which
//! serves the application for the rest of the session through its standard input and output.
//! A [`Handoff`](super::Handoff) waits for its program to end, so its screen would never come
//! back. A [`DetachedHandoff`] steps aside the same way but takes the screen back as soon as the
//! program writes its first line, and leaves it running as a [`LiveChild`].

use std::ffi::OsString;
#[cfg(not(unix))]
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
#[cfg(not(unix))]
use std::process::{Child, Command as ChildCommand, ExitStatus};
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use super::command::MapFn;
#[cfg(unix)]
use super::foreground::Foreground;
use super::handoff::{HandoffRequest, HandoffScreen, Program};
use super::live_child::{self, ChildLine, LiveChild, Sink};
use super::task::Delivery;

/// How often the wait for the first line looks at the program itself: whether it ended, or
/// stopped and must go on.
const LOOK: Duration = Duration::from_millis(20);

/// How long a program that ended is given for its output to arrive: a line written just before
/// the end may still be on its way through the pipe.
const LAST_WORDS: Duration = Duration::from_millis(100);

type LineMessage<Msg> = Arc<dyn Fn(ChildLine) -> Msg + Send + Sync>;

/// Hands the terminal to a program until it writes its first line on standard output, then
/// takes the screen back and leaves the program running in the background as a [`LiveChild`].
///
/// Everything up to the first line is a [`Handoff`](super::Handoff): the screen is released,
/// the notice printed, the program gets a process group of its own that is the terminal's
/// foreground, so it can ask on the terminal and the keys' signals (`Ctrl-C` at a password
/// prompt) reach it and not the application. Unlike a handoff the program's standard input and
/// output are pipes to the application; only its standard error is the terminal. `pkexec`
/// and `sudo` ask on the controlling terminal itself, not on standard input, so they still can.
///
/// The first line is the program saying it is ready: the terminal's foreground goes back to the
/// application, the screen is taken back and drawn again in full, and
/// [`DetachedOutcome::Detached`] arrives with the child and that line. Every later line arrives
/// through [`DetachedHandoff::on_line`], and [`ChildLine::Ended`] after the last.
///
/// A program that ends before its first line, such as `pkexec` after a cancelled or wrong
/// password (codes 126 and 127), gives [`DetachedOutcome::Finished`] as a handoff would.
///
/// ```
/// use qframe::prelude::*;
/// use qframe::runtime::{ChildLine, DetachedHandoff, DetachedOutcome, LiveChild};
///
/// enum Msg {
///     Start,
///     Started(DetachedOutcome),
///     Helper(ChildLine),
/// }
///
/// fn update(helper: &mut Option<LiveChild>, msg: Msg) -> Command<Msg> {
///     match msg {
///         // The helper asks for the password through pkexec, then prints `ready` and serves
///         // one request per line until its input ends.
///         Msg::Start => Command::handoff_detached(
///             DetachedHandoff::new("pkexec", Msg::Started)
///                 .args(["/usr/lib/example/helper", "--serve"])
///                 .notice("Asking for permission to manage packages…")
///                 .on_line(Msg::Helper),
///         ),
///         Msg::Started(DetachedOutcome::Detached { child, first_line: _ }) => {
///             let _ = child.write_line("list-updates");
///             *helper = Some(child);
///             Command::none()
///         }
///         Msg::Started(_) | Msg::Helper(_) => Command::none(),
///     }
/// }
/// ```
pub struct DetachedHandoff<Msg> {
    program: Program,
    on_start: Box<dyn FnOnce(DetachedOutcome) -> Msg + Send>,
    on_line: Option<LineMessage<Msg>>,
}

impl<Msg: Send + 'static> DetachedHandoff<Msg> {
    /// Runs `program`, delivering `on_start(outcome)` once the application has the screen back:
    /// after the program's first line, or after its end when it wrote none.
    pub fn new(program: impl Into<OsString>, on_start: impl FnOnce(DetachedOutcome) -> Msg + Send + 'static) -> Self {
        Self { program: Program::new(program.into()), on_start: Box::new(on_start), on_line: None }
    }

    /// Adds one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.program.args.push(arg.into());
        self
    }

    /// Adds several arguments, in order.
    #[must_use]
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.program.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Runs the program in `dir` instead of the application's working directory.
    #[must_use]
    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.program.dir = Some(dir.into());
        self
    }

    /// Sets an environment variable for the program. The rest of the environment is inherited.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.program.env.push((key.into(), value.into()));
        self
    }

    /// A line printed on the cleared screen before the program starts, so the user knows why the
    /// application stepped aside.
    #[must_use]
    pub fn notice(mut self, text: impl Into<String>) -> Self {
        self.program.notice = Some(text.into());
        self
    }

    /// Waits for a key press when the program ends without a first line, so what it wrote on the
    /// terminal — why a password was refused — can be read. A program that detaches never waits
    /// for it. Off by default.
    #[must_use]
    pub fn pause(mut self, pause: bool) -> Self {
        self.program.pause = pause;
        self
    }

    /// Turns the program's later output into messages: each line after the first as
    /// [`ChildLine::Line`], then [`ChildLine::Ended`] once its output closed and it ended. The
    /// lines keep coming for the program's whole life, however long after the handoff.
    /// Without it the output is read and dropped.
    #[must_use]
    pub fn on_line(mut self, message: impl Fn(ChildLine) -> Msg + Send + Sync + 'static) -> Self {
        self.on_line = Some(Arc::new(message));
        self
    }

    /// What a test sees of this handoff.
    pub(crate) fn request(&self) -> HandoffRequest {
        self.program.request()
    }

    /// The message of `outcome`. A detached child's later lines go, as messages of
    /// [`DetachedHandoff::on_line`], to `deliveries`, and each one wakes the loop.
    pub(crate) fn finish(self, outcome: DetachedOutcome, deliveries: Sender<Delivery<Msg>>) -> Msg {
        if let DetachedOutcome::Detached { child, .. } = &outcome {
            let sink: Sink = match self.on_line {
                Some(message) => Box::new(move |line| {
                    // A line arriving after the loop has gone has nowhere to be shown; the child
                    // is detached and outlives the application on purpose.
                    let _ = deliveries.send(Delivery::Message(message(line)));
                    super::signals::wake();
                }),
                None => Box::new(|_| {}),
            };
            child.attach(sink);
        }
        (self.on_start)(outcome)
    }

    /// The same handoff delivering `map(message)` for both of its messages.
    pub(crate) fn map<B: Send + 'static>(self, map: MapFn<Msg, B>) -> DetachedHandoff<B> {
        let on_start = self.on_start;
        let on_line = self.on_line.map(|message| {
            let map = Arc::clone(&map);
            Arc::new(move |line| map(message(line))) as LineMessage<B>
        });
        DetachedHandoff { program: self.program, on_start: Box::new(move |outcome| map(on_start(outcome))), on_line }
    }
}

/// How a [`DetachedHandoff`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetachedOutcome {
    /// The program wrote its first line and runs on in the background.
    Detached {
        /// The running program.
        child: LiveChild,
        /// Its first line, without the newline.
        first_line: String,
    },
    /// The program ended without a first line; `code` is `None` when a signal ended it.
    Finished {
        /// The exit code, or `None` after a signal such as an interrupt.
        code: Option<i32>,
    },
    /// The program could not be started, or the terminal could not be restored.
    Failed(String),
}

/// Runs `handoff` on the calling thread until its program writes its first line or ends, and
/// returns the message of its outcome. The screen is left and taken back through `screen`, as
/// for a [`Handoff`](super::Handoff).
pub(crate) fn run<Msg: Send + 'static>(
    handoff: DetachedHandoff<Msg>,
    screen: &mut HandoffScreen<'_>,
    deliveries: &Sender<Delivery<Msg>>,
) -> Msg {
    let outcome = match (screen.release)(handoff.program.notice.as_deref()) {
        Ok(()) => {
            let outcome = start(&handoff.program);
            if handoff.program.pause && matches!(outcome, DetachedOutcome::Finished { .. }) {
                // The pause is a courtesy, so the person can read what the program left before
                // the application takes the screen back. A keyboard that cannot be read means
                // there was nobody to wait for, and going straight on is the better answer.
                let _ = (screen.wait_for_key)();
            }
            match (screen.take)() {
                Ok(()) => outcome,
                // A child whose screen could not come back is dropped with the outcome: its
                // input closes and it ends.
                Err(error) => DetachedOutcome::Failed(error.to_string()),
            }
        }
        Err(error) => {
            // Application mode may be half gone; taking the screen back puts it right.
            let _ = (screen.take)();
            DetachedOutcome::Failed(error.to_string())
        }
    };
    handoff.finish(outcome, deliveries.clone())
}

/// Starts the program with the terminal's foreground and pipes for its input and output, and
/// waits for its first line or its end.
fn start(program: &Program) -> DetachedOutcome {
    // Three paths below give up on a child that started but cannot be used, and each ends it the
    // same way: the kill fails only on a child that ended by itself, and the wait collects it so
    // nothing is left behind. Neither has an answer to add — the `Failed` outcome returned beside
    // it already carries the reason the person needs.
    let mut command = program.command();
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
    let (mut child, foreground) = match Foreground::spawn(&mut command) {
        Ok(started) => started,
        Err(error) => return DetachedOutcome::Failed(error.to_string()),
    };
    drop(command);
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        let _ = foreground.wait(&mut child);
        return DetachedOutcome::Failed("the program's pipes could not be opened".to_owned());
    };
    let (first_sender, first) = mpsc::sync_channel(1);
    let (attach, attached) = mpsc::sync_channel(1);
    let reader = std::thread::Builder::new()
        .name("quvyta-live-child".to_owned())
        .spawn(move || live_child::read(stdout, &first_sender, &attached));
    if let Err(error) = reader {
        let _ = child.kill();
        let _ = foreground.wait(&mut child);
        return DetachedOutcome::Failed(error.to_string());
    }
    let first_line = loop {
        match first.recv_timeout(LOOK) {
            Ok(line) => break line,
            Err(RecvTimeoutError::Disconnected) => break None,
            Err(RecvTimeoutError::Timeout) => match foreground.check(&mut child) {
                Ok(None) => {}
                Ok(Some(_)) => break first.recv_timeout(LAST_WORDS).ok().flatten(),
                Err(error) => {
                    let _ = child.kill();
                    let _ = foreground.wait(&mut child);
                    return DetachedOutcome::Failed(error.to_string());
                }
            },
        }
    };
    match first_line {
        Some(first_line) => {
            // The program runs on in its own group, which is now the terminal's background.
            let child = LiveChild::running(child, stdin, attach);
            match foreground.give_back() {
                Ok(()) => DetachedOutcome::Detached { child, first_line },
                Err(error) => DetachedOutcome::Failed(error.to_string()),
            }
        }
        None => {
            // Without its output there is nothing to detach; the closed input tells a program
            // that still reads it to finish, and it ends as a handoff's program would.
            drop(stdin);
            let status = foreground.wait(&mut child);
            let taken = foreground.give_back();
            match (status, taken) {
                (Ok(status), Ok(())) => DetachedOutcome::Finished { code: status.code() },
                (Err(error), _) | (_, Err(error)) => DetachedOutcome::Failed(error.to_string()),
            }
        }
    }
}

/// Without Unix there is no terminal foreground to lend: the program is only started, and
/// waited for as any child.
#[cfg(not(unix))]
struct Foreground;

#[cfg(not(unix))]
impl Foreground {
    fn spawn(command: &mut ChildCommand) -> io::Result<(Child, Self)> {
        Ok((command.spawn()?, Self))
    }

    fn wait(&self, child: &mut Child) -> io::Result<ExitStatus> {
        child.wait()
    }

    fn check(&self, child: &mut Child) -> io::Result<Option<ExitStatus>> {
        child.try_wait()
    }

    fn give_back(self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;
    use std::sync::mpsc::{self, Receiver};
    use std::time::Duration;

    use super::{DetachedHandoff, DetachedOutcome, run};
    use crate::runtime::ChildLine;
    use crate::runtime::handoff::HandoffScreen;
    use crate::runtime::task::Delivery;

    /// How long a line may take to arrive on a loaded machine.
    const PATIENCE: Duration = Duration::from_secs(30);

    /// What the application hears: the outcome, then the child's lines.
    #[derive(Debug, PartialEq)]
    enum Heard {
        Started(DetachedOutcome),
        Said(ChildLine),
    }

    /// Runs `script` through `sh` as a detached handoff against a stand-in screen. Returns the
    /// outcome, what was done to the screen, in order, and where the child's later lines arrive.
    fn detach(
        handoff: DetachedHandoff<Heard>,
        release_fails: bool,
    ) -> (DetachedOutcome, Vec<String>, Receiver<Delivery<Heard>>) {
        let steps = RefCell::new(Vec::new());
        let mut release = |notice: Option<&str>| -> io::Result<()> {
            steps.borrow_mut().push(notice.map_or_else(|| "release".to_owned(), |text| format!("release {text}")));
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
        let (deliveries, lines) = mpsc::channel();
        let message = run(
            handoff,
            &mut HandoffScreen { release: &mut release, take: &mut take, wait_for_key: &mut wait_for_key },
            &deliveries,
        );
        let Heard::Started(outcome) = message else {
            panic!("the handoff delivers its outcome first: {message:?}");
        };
        (outcome, steps.into_inner(), lines)
    }

    fn shell(script: &str) -> DetachedHandoff<Heard> {
        DetachedHandoff::new("sh", Heard::Started).args(["-c", script]).on_line(Heard::Said)
    }

    /// The next line the child said.
    fn next(lines: &Receiver<Delivery<Heard>>) -> ChildLine {
        match lines.recv_timeout(PATIENCE) {
            Ok(Delivery::Message(Heard::Said(line))) => line,
            Ok(Delivery::Message(other)) => panic!("only lines follow the outcome: {other:?}"),
            Ok(Delivery::Ended) => panic!("a child's lines are not background work that ends"),
            Err(error) => panic!("no line arrived: {error}"),
        }
    }

    #[test]
    fn the_first_line_brings_the_screen_back_and_the_child_runs_on() {
        let (outcome, steps, lines) = detach(shell("echo ready; echo more; cat").notice("Starting the helper"), false);
        let DetachedOutcome::Detached { child, first_line } = outcome else {
            panic!("the program said it was ready: {outcome:?}");
        };
        assert_eq!(first_line, "ready");
        assert_eq!(steps, ["release Starting the helper", "take"], "the screen came back while the child runs");
        assert_eq!(child.try_wait().expect("its state"), None, "the child is still running");
        assert_eq!(next(&lines), ChildLine::Line("more".to_owned()), "a line right after the first is kept");
        child.write_line("ping").expect("the child reads its input");
        assert_eq!(next(&lines), ChildLine::Line("ping".to_owned()), "what the application wrote reached the child");
        child.write_line("päckage ünïcode").expect("the child reads its input");
        assert_eq!(next(&lines), ChildLine::Line("päckage ünïcode".to_owned()));
        child.close_stdin();
        assert_eq!(next(&lines), ChildLine::Ended { code: Some(0) }, "`cat` ended at the end of its input");
        assert_eq!(child.try_wait().expect("its state"), Some(Some(0)));
        let refused = child.write_line("late").expect_err("the input is closed");
        assert_eq!(refused.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn dropping_the_last_clone_closes_the_childs_input() {
        let (outcome, _, lines) = detach(shell("echo ready; cat; echo bye"), false);
        let DetachedOutcome::Detached { child, .. } = outcome else {
            panic!("the program said it was ready: {outcome:?}");
        };
        let kept = child.clone();
        drop(child);
        kept.write_line("still open").expect("a clone keeps the input open");
        assert_eq!(next(&lines), ChildLine::Line("still open".to_owned()));
        // The application's state goes when its run ends, and the child with it.
        drop(kept);
        assert_eq!(next(&lines), ChildLine::Line("bye".to_owned()), "the child read the end of its input");
        assert_eq!(next(&lines), ChildLine::Ended { code: Some(0) });
    }

    #[test]
    fn a_program_that_ends_before_its_first_line_finishes_as_a_handoff_would() {
        let (outcome, steps, _) = detach(shell("exit 126"), false);
        assert_eq!(outcome, DetachedOutcome::Finished { code: Some(126) }, "pkexec's code for a refused password");
        assert_eq!(steps, ["release", "take"]);
        // Standard error is the terminal, not the pipe, so what it says there is no first line.
        let (outcome, _, _) = detach(shell("echo refused >&2; exit 127"), false);
        assert_eq!(outcome, DetachedOutcome::Finished { code: Some(127) });
        let (outcome, _, _) = detach(shell("kill -TERM $$"), false);
        assert_eq!(outcome, DetachedOutcome::Finished { code: None }, "a signal leaves no code");
    }

    #[test]
    fn a_program_that_closes_its_output_is_waited_for() {
        let (outcome, _, _) = detach(shell("exec >&-; sleep 0.2; exit 4"), false);
        assert_eq!(outcome, DetachedOutcome::Finished { code: Some(4) }, "the end of the output is not the end");
    }

    #[test]
    fn a_line_said_just_before_the_end_still_detaches_and_the_end_follows() {
        let (outcome, _, lines) = detach(shell("echo ready; exit 5"), false);
        let DetachedOutcome::Detached { first_line, .. } = outcome else {
            panic!("the line came first: {outcome:?}");
        };
        assert_eq!(first_line, "ready");
        assert_eq!(next(&lines), ChildLine::Ended { code: Some(5) });
    }

    #[test]
    fn a_child_can_be_killed() {
        let (outcome, _, lines) = detach(shell("echo ready; exec sleep 30"), false);
        let DetachedOutcome::Detached { child, .. } = outcome else {
            panic!("the program said it was ready: {outcome:?}");
        };
        assert!(child.id().is_some(), "a real child has a process id");
        child.kill().expect("our own child may be killed");
        assert_eq!(next(&lines), ChildLine::Ended { code: None }, "killed by a signal");
    }

    #[test]
    fn pause_waits_for_a_key_only_when_the_program_ended_without_detaching() {
        let (_, finished, _) = detach(shell("exit 1").pause(true), false);
        assert_eq!(finished, ["release", "key", "take"], "the reason can be read before the screen comes back");
        let (outcome, detached, _) = detach(shell("echo ready; cat").pause(true), false);
        assert!(matches!(outcome, DetachedOutcome::Detached { .. }), "{outcome:?}");
        assert_eq!(detached, ["release", "take"], "a program that is ready leaves nothing to read");
    }

    #[test]
    fn an_unstartable_program_fails_and_the_screen_still_comes_back() {
        let (outcome, steps, _) = detach(DetachedHandoff::new("quvyta-no-such-program", Heard::Started), false);
        let DetachedOutcome::Failed(reason) = outcome else {
            panic!("a program that is not there cannot have started: {outcome:?}");
        };
        assert!(!reason.is_empty(), "the reason names what went wrong");
        assert_eq!(steps, ["release", "take"]);
    }

    #[test]
    fn a_terminal_that_cannot_be_released_fails_without_running_the_program() {
        let (outcome, steps, _) = detach(shell("echo ready; cat"), true);
        assert_eq!(outcome, DetachedOutcome::Failed("no terminal".to_owned()));
        assert_eq!(steps, ["release", "take"], "application mode is put back");
    }

    #[test]
    fn arguments_the_directory_and_the_environment_reach_the_program() {
        let handoff = DetachedHandoff::new("sh", Heard::Started)
            .arg("-c")
            .arg(r#"echo "$(pwd) $QUVYTA_DETACHED_TEST"; cat"#)
            .dir("/")
            .env("QUVYTA_DETACHED_TEST", "ok");
        let request = handoff.request();
        assert_eq!(request.program, std::ffi::OsString::from("sh"));
        assert!(!request.pause);
        let (outcome, _, _) = detach(handoff, false);
        let DetachedOutcome::Detached { first_line, .. } = outcome else {
            panic!("the program said it was ready: {outcome:?}");
        };
        assert_eq!(first_line, "/ ok");
    }
}
