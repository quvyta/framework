//! Running a child process and reading its output line by line, for showing in a log view.
//!
//! Two modes, and the difference matters:
//!
//! - **Pipes** (the default) keep standard output and standard error apart, so a failure stays
//!   recognisable as a failure. Programs that check for a terminal drop their progress bar and
//!   their colour when they write to a pipe.
//! - **A pseudo-terminal** ([`Process::pty`]) gives the child a terminal of the size we choose,
//!   so it draws its progress. Both of its streams land on that one terminal, so every line
//!   arrives as [`Line::Out`].
//!
//! A line that a `\r` overwrites, such as each frame of a progress bar, is dropped as a screen
//! would drop it, unless the frames are asked for with [`Process::run_with_overwritten`].

use std::ffi::OsString;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::time::Duration;

/// How long the loop waits for the next line before it looks at `cancel` again.
const POLL: Duration = Duration::from_millis(10);

/// How much is read from a stream at a time.
pub(super) const CHUNK: usize = 4096;

/// The longest line held at once. A program that writes more without a newline has its line
/// delivered in pieces of at most this many bytes, so the reader never holds all of it.
const MAX_LINE: usize = 64 * 1024;

/// How many lines wait for `on_line` at most. Beyond that the readers stop reading, the pipe
/// fills and the child waits, so a child that writes faster than the application reads does not
/// pile its output up in memory.
const QUEUE: usize = 1024;

/// A child process whose output is read line by line, for showing in a log view.
///
/// ```no_run
/// use qframe::runtime::{Line, Process};
///
/// let mut lines = Vec::new();
/// let outcome = Process::new("sh")
///     .args(["-c", "echo ready"])
///     .env("LC_ALL", "C")
///     .run(&|| false, &mut |line| lines.push(line))?;
/// assert_eq!(lines, vec![Line::Out("ready".to_owned())]);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    program: OsString,
    args: Vec<OsString>,
    dir: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    pty: Option<(u16, u16)>,
    no_stdin: bool,
}

/// Where a line came from. With a pseudo-terminal both streams share one line, so only
/// [`Line::Out`] appears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A line the child wrote to its standard output.
    Out(String),
    /// A line the child wrote to its standard error.
    Err(String),
}

/// How the child ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessOutcome {
    /// The child ran to its end; `code` is `None` when a signal ended it.
    Finished {
        /// The exit code, or `None` when a signal ended the child.
        code: Option<i32>,
    },
    /// `cancel` turned true, so the child was killed and its pending output dropped.
    Cancelled,
}

impl Process {
    /// A child process that runs `program`, with pipes and the application's own environment.
    #[must_use]
    pub fn new(program: impl Into<OsString>) -> Self {
        Self { program: program.into(), args: Vec::new(), dir: None, env: Vec::new(), pty: None, no_stdin: false }
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

    /// Runs the child in `dir` instead of the application's working directory.
    #[must_use]
    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = Some(dir.into());
        self
    }

    /// Sets one environment variable for the child. The rest of the environment is inherited,
    /// and setting a variable the application already has replaces it for the child only.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Runs the child on a pseudo-terminal `cols` wide and `rows` tall, so programs that check
    /// for a terminal draw their progress and colour. Standard input stays the application's
    /// own unless [`Process::no_stdin`] is asked for, and the child keeps the controlling
    /// terminal, which is what keeps a warm `sudo` ticket shared. Without this the child gets
    /// pipes and sees no terminal.
    ///
    /// The child reads the size given here, not the real terminal's, so its progress bar fits
    /// the space the application is going to draw it in.
    #[must_use]
    pub fn pty(mut self, cols: u16, rows: u16) -> Self {
        self.pty = Some((cols, rows));
        self
    }

    /// Gives the child no standard input: it reads an empty stream (`/dev/null`) instead of the
    /// application's terminal. A program that asks a question then gets no answer rather than
    /// the keys meant for the application, which it would otherwise take from under it.
    ///
    /// On Unix the child also starts in a process group of its own, so cancelling ends the
    /// programs it started as well; see [`Process::run`]. It keeps the application's session
    /// and controlling terminal, so a warm `sudo` ticket still applies. A program that reads
    /// the terminal itself anyway, as `sudo` does to ask for a password, is stopped by the
    /// system until it is cancelled, because its group is not the one the terminal belongs to:
    /// warm the ticket first with a [`Handoff`](crate::runtime::Handoff) of `sudo -v`, or pass
    /// `sudo -n` so it fails at once instead of asking.
    #[must_use]
    pub fn no_stdin(mut self) -> Self {
        self.no_stdin = true;
        self
    }

    /// Runs the child, handing every line to `on_line`, and returns how it ended.
    ///
    /// Lines arrive one by one, without their newline. A `\r` overwrites the line being built
    /// rather than starting a new one, which is how progress bars are written, and the last
    /// line is delivered even when the output does not end with a newline. Bytes that are not
    /// UTF-8 become the replacement character instead of being dropped. A line longer than
    /// 64 KiB is delivered in pieces of at most that size, cut between characters, so a program
    /// that never writes a newline cannot make the reader hold all of its output.
    ///
    /// When the child writes faster than `on_line` takes its lines, the reading waits and the
    /// child waits with it, rather than its output piling up in memory.
    ///
    /// `cancel` is asked between lines, and every few milliseconds while there is none, also
    /// after the child has closed its output but keeps running; when it turns true the child is
    /// killed, its pending output is dropped and the outcome is [`ProcessOutcome::Cancelled`].
    ///
    /// What cancelling kills depends on standard input. With [`Process::no_stdin`] on Unix, the
    /// child runs in a process group of its own and the whole group is killed, so the programs
    /// it started go with it (`podman` with `buildah` and the build's steps), except those that
    /// moved to a group or session of their own. Without it the child shares the application's
    /// standard input, which is the terminal: in a group of its own it would be stopped by the
    /// system the first time it read from it, so it stays in the application's group and only
    /// the child itself is killed; a program that started children of its own can leave them
    /// running.
    ///
    /// Meant to be called inside a [`Task`](crate::runtime::Task), with `cancel` reading
    /// [`TaskCx::is_cancelled`](crate::runtime::TaskCx::is_cancelled).
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the child cannot be started, when a pseudo-terminal was asked
    /// for and cannot be opened, or when a reading thread cannot be started.
    pub fn run(self, cancel: &dyn Fn() -> bool, on_line: &mut dyn FnMut(Line)) -> io::Result<ProcessOutcome> {
        self.run_inner(cancel, on_line, None)
    }

    /// Runs the child like [`Process::run`], and also hands every line a `\r` overwrites to
    /// `on_overwritten` instead of dropping it: the frames of a progress bar, as `cargo`,
    /// `pacman`, `curl` and `git` write them.
    ///
    /// A frame is the text built since the last line end or `\r`, delivered when the byte
    /// after the `\r` shows that the line really is overwritten; `\r\n` and `\r\r\n` stay
    /// plain line ends and give no frame, and an empty frame is not delivered. When the output
    /// ends right after a `\r`, its last frame is delivered too. Colour codes and erase codes
    /// such as `ESC [K` are passed on untouched. A frame comes tagged like a line: [`Line::Out`]
    /// or [`Line::Err`] for the stream it was written to, and always [`Line::Out`] on a
    /// pseudo-terminal. Lines and frames arrive in the order the child wrote them; `on_line`
    /// receives exactly what [`Process::run`] would hand it.
    ///
    /// ```no_run
    /// use qframe::runtime::{Line, Process};
    ///
    /// let (mut lines, mut frames) = (Vec::new(), Vec::new());
    /// Process::new("sh").args(["-c", r"printf '10%\r50%\rdone\n'"]).run_with_overwritten(
    ///     &|| false,
    ///     &mut |line| lines.push(line),
    ///     &mut |frame| frames.push(frame),
    /// )?;
    /// assert_eq!(lines, vec![Line::Out("done".to_owned())]);
    /// assert_eq!(frames, vec![Line::Out("10%".to_owned()), Line::Out("50%".to_owned())]);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// The same as [`Process::run`].
    pub fn run_with_overwritten(
        self,
        cancel: &dyn Fn() -> bool,
        on_line: &mut dyn FnMut(Line),
        on_overwritten: &mut dyn FnMut(Line),
    ) -> io::Result<ProcessOutcome> {
        self.run_inner(cancel, on_line, Some(on_overwritten))
    }

    /// Runs the child; frames are read at all only when someone takes them, so a plain run
    /// never queues them.
    fn run_inner(
        self,
        cancel: &dyn Fn() -> bool,
        on_line: &mut dyn FnMut(Line),
        mut on_overwritten: Option<&mut dyn FnMut(Line)>,
    ) -> io::Result<ProcessOutcome> {
        let frames = on_overwritten.is_some();
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        // The group is what lets cancelling reach the child's own children; see `run`'s notes.
        let group = self.no_stdin && cfg!(unix);
        if self.no_stdin {
            command.stdin(Stdio::null());
        } else {
            command.stdin(Stdio::inherit());
        }
        #[cfg(unix)]
        if group {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        if let Some(dir) = &self.dir {
            command.current_dir(dir);
        }
        for (key, value) in &self.env {
            command.env(key, value);
        }
        let (sender, receiver) = mpsc::sync_channel(QUEUE);
        let mut child = match self.pty {
            Some(size) => spawn_on_pty(command, size, &sender, frames, group)?,
            None => spawn_on_pipes(command, &sender, frames, group)?,
        };
        // The readers hold the only remaining senders, so the channel ends when they do.
        drop(sender);
        loop {
            if cancel() {
                kill(&mut child, group);
                return Ok(ProcessOutcome::Cancelled);
            }
            match receiver.recv_timeout(POLL) {
                Ok(Sent::Line(line)) => on_line(line),
                Ok(Sent::Overwritten(frame)) => {
                    if let Some(on_overwritten) = on_overwritten.as_deref_mut() {
                        on_overwritten(frame);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        // Its streams are closed, but the child may still be running: it closed them itself, or
        // they were handed to a program of its own. Waiting keeps asking `cancel`.
        loop {
            if let Some(status) = child.try_wait()? {
                return Ok(ProcessOutcome::Finished { code: status.code() });
            }
            if cancel() {
                kill(&mut child, group);
                return Ok(ProcessOutcome::Cancelled);
            }
            std::thread::sleep(POLL);
        }
    }
}

/// What a reading thread hands to the loop in [`Process::run_inner`]. One channel carries both
/// so lines and frames keep the order the child wrote them in.
enum Sent {
    Line(Line),
    Overwritten(Line),
}

/// Kills the child, and with `group` every process still in its process group, and waits for the
/// child so it leaves nothing behind.
fn kill(child: &mut Child, group: bool) {
    #[cfg(unix)]
    if group {
        let leader = rustix::process::Pid::from_child(child);
        // Fails only when nothing is left in the group; the child itself is killed below in any
        // case.
        let _ = rustix::process::kill_process_group(leader, rustix::process::Signal::KILL);
    }
    #[cfg(not(unix))]
    let _ = group;
    let _ = child.kill();
    let _ = child.wait();
}

/// Starts the child with a pipe per stream and a reading thread for each, so a failure stays
/// recognisable as one.
fn spawn_on_pipes(mut command: Command, sender: &SyncSender<Sent>, frames: bool, group: bool) -> io::Result<Child> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    drop(command);
    let taken = child.stdout.take().zip(child.stderr.take());
    let started = match taken {
        Some((out, err)) => spawn_reader("out", out, Line::Out, frames, sender.clone())
            .and_then(|()| spawn_reader("err", err, Line::Err, frames, sender.clone())),
        None => Err(io::Error::other("the child was started without its pipes")),
    };
    match started {
        Ok(()) => Ok(child),
        Err(error) => {
            kill(&mut child, group);
            Err(error)
        }
    }
}

/// Starts the child on a pseudo-terminal of the given size, reading the one stream both of its
/// streams land on.
#[cfg(unix)]
fn spawn_on_pty(
    mut command: Command,
    (cols, rows): (u16, u16),
    sender: &SyncSender<Sent>,
    frames: bool,
    group: bool,
) -> io::Result<Child> {
    use std::fs::File;
    use std::os::fd::OwnedFd;

    use rustix::fs::{Mode, OFlags};
    use rustix::io::{FdFlags, fcntl_setfd};
    use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
    use rustix::termios::{Winsize, tcsetwinsize};

    // Our own side must not reach the child: it would then hold the terminal open itself and
    // reading would never end. Where the flag can be given at once, no program another thread
    // starts in between can inherit it either; elsewhere it is set right after.
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "netbsd"))]
    let flags = OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC;
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "netbsd")))]
    let flags = OpenptFlags::RDWR | OpenptFlags::NOCTTY;
    let controller = openpt(flags)?;
    fcntl_setfd(&controller, FdFlags::CLOEXEC)?;
    grantpt(&controller)?;
    unlockpt(&controller)?;
    tcsetwinsize(&controller, Winsize { ws_row: rows, ws_col: cols, ws_xpixel: 0, ws_ypixel: 0 })?;
    let name = ptsname(&controller, Vec::new())?;
    // `NOCTTY` leaves the child on the application's controlling terminal instead of making this
    // pseudo-terminal the controlling one; a `sudo` ticket is held per controlling terminal, so
    // taking it away would ask for the password again.
    // `CLOEXEC` keeps this descriptor itself out of the child, which gets the terminal only as
    // its standard output and error, and out of any program another thread starts meanwhile:
    // a stray copy would keep the terminal open after the child closed its streams.
    let device: OwnedFd = rustix::fs::open(name, OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC, Mode::empty())?;
    command.stdout(Stdio::from(device.try_clone()?)).stderr(Stdio::from(device));
    let mut child = command.spawn()?;
    // The command holds the child's side of the terminal until it is dropped, and while it is
    // open the reading side never reaches its end of file.
    drop(command);
    match spawn_reader("pty", File::from(controller), Line::Out, frames, sender.clone()) {
        Ok(()) => Ok(child),
        Err(error) => {
            kill(&mut child, group);
            Err(error)
        }
    }
}

/// Without Unix there is no pseudo-terminal to open, so the caller is told instead of being
/// given a child that quietly sees no terminal.
#[cfg(not(unix))]
fn spawn_on_pty(
    _command: Command,
    _size: (u16, u16),
    _sender: &SyncSender<Sent>,
    _frames: bool,
    _group: bool,
) -> io::Result<Child> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "a pseudo-terminal needs a Unix system"))
}

/// Reads `source` on its own thread, sending one message per line, and with `frames` one per
/// overwritten frame, until the stream ends or the receiver is gone.
fn spawn_reader(
    name: &str,
    source: impl Read + Send + 'static,
    tag: fn(String) -> Line,
    frames: bool,
    sender: SyncSender<Sent>,
) -> io::Result<()> {
    std::thread::Builder::new()
        .name(format!("quvyta-process-{name}"))
        .spawn(move || read_lines(source, tag, frames, &sender))
        .map(|_| ())
}

/// Sends every line of `source` as a message, stopping as soon as the receiver is gone.
fn read_lines(mut source: impl Read, tag: fn(String) -> Line, frames: bool, sender: &SyncSender<Sent>) {
    let mut chunk = [0_u8; CHUNK];
    let mut lines = Lines::default();
    // Both closures send; a failed send from either means nobody listens any more.
    let listening = std::cell::Cell::new(true);
    let mut on_line = |line| listening.set(listening.get() && sender.send(Sent::Line(tag(line))).is_ok());
    let mut on_frame = |frame| listening.set(listening.get() && sender.send(Sent::Overwritten(tag(frame))).is_ok());
    loop {
        match source.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                lines.feed_keeping(&chunk[..count], &mut on_line, frames.then_some(&mut on_frame));
                if !listening.get() {
                    return;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            // A pseudo-terminal answers with an I/O error once the child's side is gone, and a
            // broken pipe says the same thing; both are the end of the stream.
            Err(_) => break,
        }
    }
    lines.finish_keeping(&mut on_line, frames.then_some(&mut on_frame));
}

/// Splits a byte stream into lines, letting `\r` overwrite the line being built. What it
/// overwrites is dropped, or handed to a second callback by [`Lines::feed_keeping`].
#[derive(Debug, Default)]
pub(super) struct Lines {
    buffer: Vec<u8>,
    /// A `\r` was read and it is not yet known whether a `\n` follows it.
    pending_return: bool,
}

impl Lines {
    /// Feeds `bytes`, calling `emit` once per finished line.
    pub(super) fn feed(&mut self, bytes: &[u8], emit: &mut impl FnMut(String)) {
        self.feed_keeping(bytes, emit, None);
    }

    /// Feeds `bytes` like [`Lines::feed`], also handing each non-empty frame a `\r`
    /// overwrites to `overwritten` when it is given.
    pub(super) fn feed_keeping(
        &mut self,
        bytes: &[u8],
        emit: &mut impl FnMut(String),
        mut overwritten: Option<&mut dyn FnMut(String)>,
    ) {
        for &byte in bytes {
            if self.pending_return {
                // A terminal ends its lines with `\r\n`, so a `\r` right before a newline ends
                // the line rather than overwriting it. A second `\r` changes nothing, as on a
                // screen: a program's own `\r\n` arrives as `\r\r\n` from a pseudo-terminal.
                match byte {
                    b'\r' => continue,
                    b'\n' => {
                        self.pending_return = false;
                        emit(self.take());
                        continue;
                    }
                    _ => {
                        self.pending_return = false;
                        self.overwrite(&mut overwritten);
                    }
                }
            }
            match byte {
                b'\r' => self.pending_return = true,
                b'\n' => emit(self.take()),
                _ => {
                    self.buffer.push(byte);
                    if self.buffer.len() >= MAX_LINE {
                        self.emit_piece(emit);
                    }
                }
            }
        }
    }

    /// Delivers the full buffer as a line of its own, keeping back the start of a character
    /// that is not complete yet so no character is cut in two.
    fn emit_piece(&mut self, emit: &mut impl FnMut(String)) {
        // A character is at most four bytes, so only the last three can start one that is not
        // complete yet. Anything else that is not UTF-8 is replaced as usual.
        let len = self.buffer.len();
        let mut cut = len;
        for back in 1..=len.min(3) {
            let byte = self.buffer[len - back];
            if byte & 0b1100_0000 != 0b1000_0000 {
                let width = match byte {
                    0xc0..=0xdf => 2,
                    0xe0..=0xef => 3,
                    0xf0..=0xf7 => 4,
                    _ => 1,
                };
                if width > back {
                    cut = len - back;
                }
                break;
            }
        }
        let rest = self.buffer.split_off(cut);
        emit(self.take());
        self.buffer = rest;
    }

    /// Drops the line a `\r` overwrites, or hands it to `overwritten` when there is one.
    fn overwrite(&mut self, overwritten: &mut Option<&mut dyn FnMut(String)>) {
        match overwritten {
            Some(overwritten) if !self.buffer.is_empty() => overwritten(self.take()),
            _ => self.buffer.clear(),
        }
    }

    /// Delivers the last line when the stream ended without a newline.
    pub(super) fn finish(&mut self, emit: &mut impl FnMut(String)) {
        self.finish_keeping(emit, None);
    }

    /// Ends the stream like [`Lines::finish`]; a last line followed by a `\r` goes to
    /// `overwritten` when it is given.
    pub(super) fn finish_keeping(
        &mut self,
        emit: &mut impl FnMut(String),
        mut overwritten: Option<&mut dyn FnMut(String)>,
    ) {
        if self.pending_return {
            // The line was overwritten and nothing was written in its place.
            self.overwrite(&mut overwritten);
            self.pending_return = false;
        }
        if !self.buffer.is_empty() {
            emit(self.take());
        }
    }

    /// The line built so far, with anything that is not UTF-8 replaced rather than dropped.
    fn take(&mut self) -> String {
        let line = String::from_utf8_lossy(&self.buffer).into_owned();
        self.buffer.clear();
        line
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Line, Lines, MAX_LINE, Process, ProcessOutcome};

    /// Runs a shell command to its end and returns its lines and outcome.
    fn shell(script: &str) -> (Vec<Line>, ProcessOutcome) {
        run(Process::new("sh").args(["-c", script]))
    }

    /// Runs `process` to its end, never cancelling.
    fn run(process: Process) -> (Vec<Line>, ProcessOutcome) {
        let mut lines = Vec::new();
        let outcome = process.run(&|| false, &mut |line| lines.push(line)).expect("the shell starts");
        (lines, outcome)
    }

    #[test]
    fn keeps_the_two_streams_apart_and_reports_the_exit_code() {
        let (lines, outcome) = shell("echo bir; echo iki >&2; exit 3");
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines.contains(&Line::Out("bir".to_owned())), "{lines:?}");
        assert!(lines.contains(&Line::Err("iki".to_owned())), "{lines:?}");
        assert_eq!(outcome, ProcessOutcome::Finished { code: Some(3) });
    }

    #[test]
    fn delivers_the_last_line_without_a_newline() {
        let (lines, outcome) = shell("printf 'son satir'");
        assert_eq!(lines, vec![Line::Out("son satir".to_owned())]);
        assert_eq!(outcome, ProcessOutcome::Finished { code: Some(0) });
    }

    #[test]
    fn carriage_returns_collapse_into_one_line() {
        let (lines, _) = shell(r"printf 'a\rbb\rccc\n'");
        assert_eq!(lines, vec![Line::Out("ccc".to_owned())]);
    }

    #[test]
    fn invalid_utf8_becomes_the_replacement_character() {
        let (lines, _) = shell(r"printf 'a\377b\n'");
        assert_eq!(lines, vec![Line::Out("a\u{fffd}b".to_owned())]);
    }

    #[test]
    fn the_environment_is_inherited_and_one_variable_can_be_replaced() {
        let (lines, _) = shell("echo ${PATH:+inherited}");
        assert_eq!(lines, vec![Line::Out("inherited".to_owned())]);
        let (lines, _) = run(Process::new("sh").args(["-c", "echo $LC_ALL"]).env("LC_ALL", "C"));
        assert_eq!(lines, vec![Line::Out("C".to_owned())]);
    }

    #[test]
    fn runs_in_the_directory_it_is_given() {
        let (lines, _) = run(Process::new("sh").args(["-c", "pwd"]).dir("/"));
        assert_eq!(lines, vec![Line::Out("/".to_owned())]);
    }

    #[test]
    fn cancelling_kills_a_long_running_child() {
        let seen = AtomicUsize::new(0);
        let outcome = Process::new("sh")
            .args(["-c", "while true; do echo tik; sleep 0.05; done"])
            .run(&|| seen.load(Ordering::Relaxed) > 0, &mut |line| {
                assert_eq!(line, Line::Out("tik".to_owned()));
                seen.fetch_add(1, Ordering::Relaxed);
            })
            .expect("the shell starts");
        assert_eq!(outcome, ProcessOutcome::Cancelled);
        assert!(seen.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn a_missing_program_is_an_error_and_not_a_panic() {
        let error = Process::new("quvyta-no-such-program")
            .run(&|| false, &mut |_| unreachable!("a missing program writes nothing"))
            .expect_err("a missing program cannot run");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn on_a_pseudo_terminal_the_child_sees_a_terminal_of_the_size_we_gave() {
        // `stty` reads its standard input, which is the application's own; reading the size the
        // child was given means asking about the stream it writes to.
        let (lines, outcome) = run(Process::new("sh").args(["-c", "test -t 1 && stty size <&1"]).pty(100, 24));
        assert_eq!(lines, vec![Line::Out("24 100".to_owned())]);
        assert_eq!(outcome, ProcessOutcome::Finished { code: Some(0) });
    }

    #[cfg(unix)]
    #[test]
    fn on_a_pseudo_terminal_both_streams_arrive_as_output() {
        let (lines, outcome) = run(Process::new("sh").args(["-c", "echo bir; echo iki >&2"]).pty(80, 24));
        assert_eq!(lines, vec![Line::Out("bir".to_owned()), Line::Out("iki".to_owned())]);
        assert_eq!(outcome, ProcessOutcome::Finished { code: Some(0) });
    }

    /// The process group, session and controlling terminal in a line of `/proc/<pid>/stat`.
    #[cfg(target_os = "linux")]
    fn stat_ids(stat: &str) -> [String; 3] {
        // The command name may hold spaces; after its closing parenthesis come state, parent,
        // group, session and terminal.
        let fields: Vec<&str> = stat[stat.rfind(')').expect("name") + 2..].split(' ').collect();
        [fields[2], fields[3], fields[4]].map(str::to_owned)
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn without_stdin_the_child_reads_an_empty_stream() {
        let script = r#"readlink /proc/$$/fd/0; read answer; echo "read $?""#;
        for process in [Process::new("sh").args(["-c", script]), Process::new("sh").args(["-c", script]).pty(80, 24)] {
            let (lines, outcome) = run(process.no_stdin());
            assert_eq!(lines, vec![Line::Out("/dev/null".to_owned()), Line::Out("read 1".to_owned())]);
            assert_eq!(outcome, ProcessOutcome::Finished { code: Some(0) });
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn only_a_child_without_stdin_gets_a_group_of_its_own_and_it_keeps_the_session() {
        let script = "cat /proc/$$/stat";
        let ours = stat_ids(&std::fs::read_to_string("/proc/self/stat").expect("stat"));
        let ids = |process: Process| {
            let (lines, _) = run(process);
            let [Line::Out(stat)] = &lines[..] else { panic!("one line: {lines:?}") };
            stat_ids(stat)
        };
        let shared = ids(Process::new("sh").args(["-c", script]));
        assert_eq!(shared, ours, "a child reading the terminal stays in the application's group");
        for process in [Process::new("sh").args(["-c", script]), Process::new("sh").args(["-c", script]).pty(80, 24)] {
            let [group, session, terminal] = ids(process.no_stdin());
            assert_ne!(group, ours[0], "a group of its own");
            // `sudo` keeps its ticket per controlling terminal and session, so both must stay.
            assert_eq!(session, ours[1], "the application's session");
            assert_eq!(terminal, ours[2], "the application's controlling terminal");
        }
    }

    /// Whether `pid` has ended: gone, or ended and waiting for its parent to collect it.
    #[cfg(target_os = "linux")]
    fn ended(pid: &str) -> bool {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .map_or(true, |stat| stat[stat.rfind(')').expect("name") + 2..].starts_with('Z'))
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cancelling_a_child_without_stdin_ends_the_programs_it_started() {
        for pty in [false, true] {
            let seen = std::cell::RefCell::new(Vec::new());
            let process = Process::new("sh").args(["-c", "sleep 60 & echo $!; sleep 60 & echo $!; wait"]).no_stdin();
            let process = if pty { process.pty(80, 24) } else { process };
            let outcome = process
                .run(&|| seen.borrow().len() == 2, &mut |line| match line {
                    Line::Out(pid) => seen.borrow_mut().push(pid),
                    Line::Err(text) => panic!("nothing on standard error: {text}"),
                })
                .expect("the shell starts");
            assert_eq!(outcome, ProcessOutcome::Cancelled);
            let pids = seen.into_inner();
            let started = std::time::Instant::now();
            while !pids.iter().all(|pid| ended(pid)) {
                assert!(started.elapsed() < std::time::Duration::from_secs(20), "still running: {pids:?} (pty {pty})");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cancelling_a_child_that_shares_stdin_ends_only_the_child() {
        // Documented: without `no_stdin` the child stays in the application's group, and its
        // own children outlive it. They are ended here by hand so the test leaves nothing behind.
        let seen = std::cell::RefCell::new(Vec::new());
        let outcome = Process::new("sh")
            .args(["-c", "sleep 60 & echo $!; wait"])
            .run(&|| seen.borrow().len() == 1, &mut |line| {
                if let Line::Out(pid) = line {
                    seen.borrow_mut().push(pid);
                }
            })
            .expect("the shell starts");
        assert_eq!(outcome, ProcessOutcome::Cancelled);
        let pid = seen.into_inner().remove(0);
        std::thread::sleep(std::time::Duration::from_millis(200));
        let survived = !ended(&pid);
        let raw: i32 = pid.parse().expect("a process id");
        if let Some(pid) = rustix::process::Pid::from_raw(raw) {
            let _ = rustix::process::kill_process(pid, rustix::process::Signal::KILL);
        }
        assert!(survived, "the grandchild outlives a cancel of the child");
    }

    #[test]
    fn a_line_ended_twice_by_a_return_is_kept() {
        // A program that ends its own lines with `\r\n` writes `\r\r\n` on a pseudo-terminal,
        // which turns every `\n` into `\r\n`. The line is on the screen, so it is not lost.
        let mut lines = Lines::default();
        let mut seen = Vec::new();
        lines.feed(b"hazir\r\r\nbitti\r\r\r\n", &mut |line| seen.push(line));
        assert_eq!(seen, vec!["hazir".to_owned(), "bitti".to_owned()]);
    }

    #[cfg(unix)]
    #[test]
    fn a_pseudo_terminal_line_ended_by_the_program_itself_arrives_whole() {
        let (lines, _) = run(Process::new("sh").args(["-c", r"printf 'bir\r\niki\r\n'"]).pty(80, 24));
        assert_eq!(lines, vec![Line::Out("bir".to_owned()), Line::Out("iki".to_owned())]);
    }

    #[test]
    fn a_line_without_an_end_is_delivered_in_pieces_of_bounded_size() {
        // A program that never writes a newline must not make the reader hold all of it.
        let (lines, _) = shell("head -c 300000 /dev/zero | tr '\\0' a");
        let total: usize = lines
            .iter()
            .map(|line| match line {
                Line::Out(text) => {
                    assert!(text.len() <= MAX_LINE, "a piece of {} bytes", text.len());
                    assert!(text.bytes().all(|byte| byte == b'a'));
                    text.len()
                }
                Line::Err(text) => panic!("nothing was written to standard error: {text}"),
            })
            .sum();
        assert_eq!(total, 300_000, "nothing is lost between the pieces");
    }

    #[test]
    fn a_long_line_is_never_cut_inside_a_character() {
        let mut lines = Lines::default();
        let mut seen = Vec::new();
        // One byte of padding, so the two-byte `ç` straddles every piece boundary.
        let mut text = vec![b'a'];
        for _ in 0..MAX_LINE {
            text.extend_from_slice("ç".as_bytes());
        }
        lines.feed(&text, &mut |line| seen.push(line));
        lines.finish(&mut |line| seen.push(line));
        assert!(seen.len() > 1, "the line was split");
        assert!(seen.iter().all(|line| !line.contains('\u{fffd}')), "no character was cut in two");
        assert_eq!(seen.concat().as_bytes(), text.as_slice());
    }

    #[test]
    fn a_child_that_closes_its_output_can_still_be_cancelled() {
        // Its streams end at once, but it keeps running; cancelling must still stop it.
        let started = std::time::Instant::now();
        let outcome = Process::new("sh")
            .args(["-c", "exec >&- 2>&-; sleep 20"])
            .run(&|| started.elapsed() > std::time::Duration::from_millis(200), &mut |_| {})
            .expect("the shell starts");
        assert_eq!(outcome, ProcessOutcome::Cancelled);
        assert!(started.elapsed() < std::time::Duration::from_secs(10), "took {:?}", started.elapsed());
    }

    #[test]
    fn a_flood_of_output_waits_for_the_reader_instead_of_piling_up() {
        let dir = std::env::temp_dir().join(format!("quvyta-process-flood-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test directory");
        let marker = dir.join("done");
        let script = format!("yes | head -n 200000; touch '{}'", marker.display());
        let mut first = true;
        let mut finished_while_the_reader_slept = false;
        let mut count = 0_usize;
        let outcome = Process::new("sh")
            .args(["-c", &script])
            .run(&|| false, &mut |_| {
                count += 1;
                if first {
                    first = false;
                    // Far longer than writing 200000 short lines takes when nothing holds it back.
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    finished_while_the_reader_slept = marker.exists();
                }
            })
            .expect("the shell starts");
        assert_eq!(outcome, ProcessOutcome::Finished { code: Some(0) });
        assert_eq!(count, 200_000);
        assert!(!finished_while_the_reader_slept, "the child wrote everything into memory while nobody read");
        std::fs::remove_dir_all(&dir).expect("clean");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_child_on_a_pseudo_terminal_holds_it_only_on_its_own_streams() {
        // Any other descriptor of the terminal would outlive the streams in the child and in the
        // programs it starts, and keep the reader waiting after they are closed.
        let script = r#"t=$(readlink /proc/$$/fd/1); n=0; for f in /proc/$$/fd/*; do [ "$(readlink "$f")" = "$t" ] && n=$((n+1)); done; echo $n"#;
        let (lines, _) = run(Process::new("sh").args(["-c", script]).pty(80, 24));
        assert_eq!(lines, vec![Line::Out("2".to_owned())], "standard output and standard error, nothing else");
    }

    #[test]
    fn a_line_split_across_reads_stays_one_line() {
        let mut lines = Lines::default();
        let mut seen = Vec::new();
        let mut emit = |line: String| seen.push(line);
        lines.feed(b"ilk par", &mut emit);
        lines.feed(b"\xc3", &mut emit);
        lines.feed(b"\xa7a\r\nson", &mut emit);
        lines.finish(&mut emit);
        assert_eq!(seen, vec!["ilk parça".to_owned(), "son".to_owned()]);
    }

    /// Feeds `bytes` in one go and ends the stream, returning the lines and the overwritten
    /// frames.
    fn split_keeping_frames(bytes: &[u8]) -> (Vec<String>, Vec<String>) {
        let mut lines = Lines::default();
        let (mut seen, mut frames) = (Vec::new(), Vec::new());
        lines.feed_keeping(bytes, &mut |line| seen.push(line), Some(&mut |frame| frames.push(frame)));
        lines.finish_keeping(&mut |line| seen.push(line), Some(&mut |frame| frames.push(frame)));
        (seen, frames)
    }

    #[test]
    fn frames_overwritten_by_a_return_are_kept_only_when_asked_for() {
        let (seen, frames) = split_keeping_frames(b"bir\riki\ruc\rbitti\r\n");
        assert_eq!(seen, vec!["bitti".to_owned()]);
        assert_eq!(frames, vec!["bir".to_owned(), "iki".to_owned(), "uc".to_owned()]);
        let mut lines = Lines::default();
        let mut seen = Vec::new();
        lines.feed(b"bir\riki\ruc\rbitti\r\n", &mut |line| seen.push(line));
        lines.finish(&mut |line| seen.push(line));
        assert_eq!(seen, vec!["bitti".to_owned()]);
    }

    #[test]
    fn a_line_ended_by_returns_and_a_newline_is_no_frame() {
        let (seen, frames) = split_keeping_frames(b"hazir\r\r\nbitti\r\n\rbos\r\r\r\n");
        assert_eq!(seen, vec!["hazir".to_owned(), "bitti".to_owned(), "bos".to_owned()]);
        assert!(frames.is_empty(), "{frames:?}");
    }

    #[test]
    fn a_stream_ending_in_a_return_delivers_its_last_frame() {
        let (seen, frames) = split_keeping_frames(b"once\r10%\r20%\r");
        assert!(seen.is_empty(), "{seen:?}");
        assert_eq!(frames, vec!["once".to_owned(), "10%".to_owned(), "20%".to_owned()]);
        // A return split from what follows it by a read still waits for that byte.
        let mut lines = Lines::default();
        let (mut seen, mut frames) = (Vec::new(), Vec::new());
        lines.feed_keeping(b"30%\r", &mut |line| seen.push(line), Some(&mut |frame| frames.push(frame)));
        assert!(frames.is_empty(), "a return before a newline is not yet known to overwrite");
        lines.feed_keeping(b"\n", &mut |line| seen.push(line), Some(&mut |frame| frames.push(frame)));
        assert_eq!((seen, frames), (vec!["30%".to_owned()], Vec::new()));
    }

    #[test]
    fn frames_keep_their_colour_and_erase_codes() {
        let (seen, frames) = split_keeping_frames(b"\x1b[1mFetch\x1b[0m 1\r\x1b[K\x1b[92mDone\x1b[0m\r\n");
        assert_eq!(frames, vec!["\x1b[1mFetch\x1b[0m 1".to_owned()]);
        assert_eq!(seen, vec!["\x1b[K\x1b[92mDone\x1b[0m".to_owned()]);
    }

    #[test]
    fn every_frame_of_a_recorded_cargo_install_is_kept() {
        let recorded = include_bytes!("../../tests/fixtures/cargo-install-pty.txt");
        let (seen, frames) = split_keeping_frames(recorded);
        assert_eq!(frames.len(), 159, "every overwritten frame");
        assert_eq!(seen.len(), 76, "the lines themselves are unchanged");
        let building: Vec<&String> = frames.iter().filter(|frame| frame.contains("Building")).collect();
        assert_eq!(building.len(), 51);
        assert!(building[0].contains("] 0/46: anstyle"), "{:?}", building[0]);
        assert!(building.iter().any(|frame| frame.contains("] 45/46: hexyl")), "{building:?}");
        // Without asking, the same bytes give the same lines and no frame reaches anyone.
        let mut lines = Lines::default();
        let mut plain = Vec::new();
        lines.feed(recorded, &mut |line| plain.push(line));
        lines.finish(&mut |line| plain.push(line));
        assert_eq!(plain, seen);
    }

    /// Runs `process` to its end asking for overwritten frames, returning the lines and frames in
    /// the order they arrived.
    fn run_keeping_frames(process: Process) -> Vec<(bool, Line)> {
        let seen = std::cell::RefCell::new(Vec::new());
        process
            .run_with_overwritten(&|| false, &mut |line| seen.borrow_mut().push((false, line)), &mut |frame| {
                seen.borrow_mut().push((true, frame));
            })
            .expect("the shell starts");
        seen.into_inner()
    }

    #[test]
    fn overwritten_frames_arrive_through_a_pipe_in_order_and_tagged_by_stream() {
        let seen = run_keeping_frames(Process::new("sh").args(["-c", r"printf 'a\rb\rc\n'; printf '1%%\r2%%\r' >&2"]));
        let out: Vec<_> = seen.iter().filter(|(_, line)| matches!(line, Line::Out(_))).cloned().collect();
        let err: Vec<_> = seen.iter().filter(|(_, line)| matches!(line, Line::Err(_))).cloned().collect();
        assert_eq!(
            out,
            vec![
                (true, Line::Out("a".to_owned())),
                (true, Line::Out("b".to_owned())),
                (false, Line::Out("c".to_owned()))
            ]
        );
        assert_eq!(err, vec![(true, Line::Err("1%".to_owned())), (true, Line::Err("2%".to_owned()))]);
    }

    #[cfg(unix)]
    #[test]
    fn overwritten_frames_arrive_from_a_pseudo_terminal() {
        let seen = run_keeping_frames(Process::new("sh").args(["-c", r"printf 'a\rb\rc\nd\r\n'"]).pty(80, 24));
        assert_eq!(
            seen,
            vec![
                (true, Line::Out("a".to_owned())),
                (true, Line::Out("b".to_owned())),
                (false, Line::Out("c".to_owned())),
                (false, Line::Out("d".to_owned())),
            ]
        );
    }
}
