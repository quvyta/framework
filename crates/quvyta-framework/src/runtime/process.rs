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
//!
//! Output nobody reads line by line is the other kind: a build's whole log, a program's answer,
//! a flood of progress. [`Process::collect`] runs the child the same way but keeps only the end
//! of what it wrote, within [`Keep`], and gives it back as one piece of text. Output nobody reads
//! at all is the last kind: [`Process::stdout_to`] hands the child's standard output to a file of
//! the application, so an archive unpacked into one costs nothing of the application's memory.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

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
///
/// For output that is not read line by line, [`Process::collect`] keeps only the end of it, and
/// [`Process::stdout_to`] writes it into a file of the application's without reading it at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    program: OsString,
    args: Vec<OsString>,
    dir: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    pty: Option<(u16, u16)>,
    no_stdin: bool,
    /// The file the child's standard output is written to, when one was named.
    out: Option<Out>,
    /// The child gets nothing of the environment but what was named with `env`.
    cleared: bool,
}

/// The file a child's standard output is written to, shared so that a [`Process`] stays `Clone`.
///
/// A `File` is neither `Clone` nor comparable, and two of these are equal when they are the same
/// file, which is as far as a builder that names one needs to go.
#[derive(Debug, Clone)]
struct Out(Arc<File>);

impl PartialEq for Out {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Out {}

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

/// How much of a child's output to keep when it is [`collected`](Process::collect), and how long
/// it may run.
///
/// It is built with [`Keep::bytes`] and narrowed with [`Keep::lines`] and [`Keep::limit`]. What
/// is kept is the end of the output, never the beginning: a program that writes more than the
/// bytes asked for has its oldest output dropped as the newest arrives, and
/// [`Collected::trimmed`] says that it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Keep {
    bytes: usize,
    lines: Option<usize>,
    limit: Option<Duration>,
    after_exit: Option<Duration>,
}

impl Keep {
    /// Keeps at most `bytes` bytes of the output, the end of it. A program that writes more has
    /// its oldest output dropped while it runs and never costs more memory than this plus one
    /// read, and the cut falls between characters, so no character is half in the text. Zero
    /// bytes is a way of asking for the outcome alone.
    #[must_use]
    pub fn bytes(bytes: usize) -> Self {
        Self { bytes, lines: None, limit: None, after_exit: None }
    }

    /// Keeps at most `lines` lines as well, the last of them. The line a program is still writing
    /// counts as one, so output that ends without a newline keeps what was written.
    #[must_use]
    pub fn lines(mut self, lines: usize) -> Self {
        self.lines = Some(lines);
        self
    }

    /// Ends the program when `limit` is over, the way cancelling does: with
    /// [`Process::no_stdin`] on Unix its whole process group goes with it, and
    /// [`Collected::timed_out`] says that the time is what ended it.
    #[must_use]
    pub fn limit(mut self, limit: Duration) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Stops waiting for the output `wait` after the child itself has ended, for a command that
    /// leaves something running behind it, such as `npm run dev &` or `(sleep 4; …) &`, which
    /// keeps the output open long after the command is done. Once the child has ended and its
    /// output is still open after `wait`, its process group is ended (with [`Process::no_stdin`]
    /// on Unix) and what was kept comes back with the child's own exit code; it is not a time
    /// limit, so [`Collected::timed_out`] stays `false`.
    ///
    /// It is also the grace a stop gives: a cancel or [`Keep::limit`] first asks the child (and
    /// its group) to end with `TERM`, so a build or a test run can clean up after itself, and
    /// only ends it with `KILL` when it is still there after `wait`. Without it the grace is two
    /// seconds.
    #[must_use]
    pub fn after_exit(mut self, wait: Duration) -> Self {
        self.after_exit = Some(wait);
        self
    }
}

/// How long a stopped child is given to end by itself after `TERM` before it is killed, unless
/// [`Keep::after_exit`] says otherwise.
const GRACE: Duration = Duration::from_secs(2);

/// What a child wrote, kept to what [`Keep`] asked for, and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Collected {
    /// The end of the child's output, both of its streams in the order it wrote them. Bytes that
    /// are not UTF-8 became the replacement character, and the text begins on a character: the
    /// cut the size asked for made never falls inside one.
    pub text: String,
    /// How the child ended: [`ProcessOutcome::Cancelled`] when `cancel` turned true, and
    /// [`ProcessOutcome::Finished`] with no code when [`Keep::limit`] ended it.
    pub outcome: ProcessOutcome,
    /// Whether older output fell out to hold what [`Keep`] asked for.
    pub trimmed: bool,
    /// Whether [`Keep::limit`] ended the child.
    pub timed_out: bool,
    /// Whether `cancel` ended the child.
    pub cancelled: bool,
}

impl Process {
    /// A child process that runs `program`, with pipes and the application's own environment.
    #[must_use]
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            dir: None,
            env: Vec::new(),
            pty: None,
            no_stdin: false,
            out: None,
            cleared: false,
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

    /// Gives the child nothing of the application's environment: what it gets is what was set
    /// with [`Process::env`] and nothing else, so a program sees what the application said it
    /// would see rather than what the shell the application was started from happened to have.
    ///
    /// `PATH` is one of the variables that has to be named, since it is what the child looks
    /// other programs up with, as in the example. `HOME`, `LANG` and the rest are just as gone.
    ///
    /// ```no_run
    /// use qframe::runtime::{Line, Process, ProcessOutcome};
    ///
    /// let path = std::env::var("PATH").expect("a path to look other programs up with");
    /// let mut lines = Vec::new();
    /// let outcome = Process::new("sh")
    ///     .args(["-c", "printf 'sadece bu'"])
    ///     .clear_env()
    ///     .env("PATH", path)
    ///     .run(&|| false, &mut |line| lines.push(line))?;
    /// assert_eq!(lines, vec![Line::Out("sadece bu".to_owned())]);
    /// assert_eq!(outcome, ProcessOutcome::Finished { code: Some(0) });
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// [`Process::env`] still sets a variable here, but that is now all the child gets.
    #[must_use]
    pub fn clear_env(mut self) -> Self {
        self.cleared = true;
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

    /// Sends the child's standard output straight to `file`, so a program that writes a lot of it
    /// costs the application nothing: a `gzip -dc` unpacking an archive, a `tar` writing an
    /// artefact, a program's answer written where it belongs. The child writes into the file
    /// itself and not one byte of it passes through the application's memory, however big it is.
    ///
    /// Only the error stream is read, so a failure stays recognisable as a failure: with
    /// [`Process::collect`] the text is the end of what the child wrote there, within [`Keep`], and
    /// with [`Process::run`] every line of it arrives as [`Line::Err`]. The file is used as it is,
    /// so the child writes at the position the descriptor already has, and nothing of the file is
    /// trimmed, read or finished by us: what it holds when the child ends is what the program
    /// wrote. Ending the child ends it the way it always is, so a cancel and a [`Keep::limit`]
    /// take the process group with it and nothing more reaches the file.
    ///
    /// [`Process::pty`] takes the child's standard output instead, since on a pseudo-terminal that
    /// stream is the terminal itself.
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::time::Duration;
    /// use qframe::runtime::{Keep, Process};
    ///
    /// let file = File::create("unpacked.sql")?;
    /// let keep = Keep::bytes(4096).limit(Duration::from_secs(30));
    /// let collected = Process::new("xz").args(["-dc", "dump.sql.xz"]).no_stdin()
    ///     .stdout_to(file)
    ///     .collect(keep, &|| false)?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    #[must_use]
    pub fn stdout_to(mut self, file: File) -> Self {
        self.out = Some(Out(Arc::new(file)));
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
        // The group is what lets cancelling reach the child's own children; see `run`'s notes.
        let group = self.no_stdin && cfg!(unix);
        let command = self.command(group);
        let (sender, receiver) = mpsc::sync_channel(QUEUE);
        let mut child = match self.pty {
            Some(size) => spawn_on_pty(command, size, &sender, frames, group)?,
            None => spawn_on_pipes(command, self.out.as_ref(), &sender, frames, group)?,
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

    /// Runs the child, keeping only the end of what it writes, and gives that back as one piece
    /// of text.
    ///
    /// Both of the child's streams land on one pipe, so the text is in the order the child wrote
    /// it and a failure stays among the rest of the output; with [`Process::pty`] both land on
    /// the terminal itself, as they always do there. Nothing is delivered line by line: the point
    /// is to hold the end of the output, so a program that prints megabytes, a build's whole log,
    /// a flood of progress, costs no more memory than [`Keep`] asks for however long it writes.
    /// A file named for the output with [`Process::stdout_to`] is the exception: the file gets the
    /// standard output as it is and only the error stream is read and kept.
    ///
    /// [`Keep::limit`] ends the child the way `cancel` does, its process group too where
    /// [`Process::no_stdin`] asked for one, and both give back the output kept so far in bounded
    /// time.
    ///
    /// Meant to be called inside a [`Task`](crate::runtime::Task), with `cancel` reading
    /// [`TaskCx::is_cancelled`](crate::runtime::TaskCx::is_cancelled).
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use qframe::runtime::{Keep, Process, ProcessOutcome};
    ///
    /// let collected = Process::new("sh")
    ///     .args(["-c", "echo ready"])
    ///     .env("LC_ALL", "C")
    ///     .collect(Keep::bytes(4096).lines(20).limit(Duration::from_secs(30)), &|| false)?;
    /// assert_eq!(collected.text, "ready\n");
    /// assert_eq!(collected.outcome, ProcessOutcome::Finished { code: Some(0) });
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// The same as [`Process::run`].
    pub fn collect(self, keep: Keep, cancel: &dyn Fn() -> bool) -> io::Result<Collected> {
        self.collect_inner(keep, cancel, None)
    }

    /// [`collect`](Self::collect), handing the end of the output kept so far to `on_tail` while
    /// the child runs, so a person sees a build or a test run go by rather than a turning wheel:
    /// once new output has come, and at most every 100 ms. The text is what [`Keep`] holds at
    /// that moment, cut as it cuts, so a call never sees more lines than it keeps. A child that
    /// writes nothing is never called about. `on_tail` runs on the calling thread, inside the
    /// task, so send what it is given on from there.
    ///
    /// ```no_run
    /// use qframe::runtime::{Keep, Process};
    ///
    /// let collected = Process::new("cargo").args(["test"]).no_stdin()
    ///     .collect_watching(Keep::bytes(8192).lines(3), &|| false, |tail| eprintln!("{tail}"))?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// The same as [`Process::run`].
    pub fn collect_watching(
        self,
        keep: Keep,
        cancel: &dyn Fn() -> bool,
        mut on_tail: impl FnMut(&str),
    ) -> io::Result<Collected> {
        self.collect_inner(keep, cancel, Some(&mut on_tail))
    }

    fn collect_inner(
        self,
        keep: Keep,
        cancel: &dyn Fn() -> bool,
        mut on_tail: Option<&mut dyn FnMut(&str)>,
    ) -> io::Result<Collected> {
        let Keep { bytes, lines, limit, after_exit } = keep;
        let grace = after_exit.unwrap_or(GRACE);
        let group = self.no_stdin && cfg!(unix);
        let command = self.command(group);
        let merged = Arc::new(Mutex::new(Merged { tail: Tail::new(bytes, lines), open: true, fed: 0 }));
        // The amount of output a watcher was last told about, and when.
        let (mut told, mut told_at) = (0u64, Instant::now() - WATCH_EVERY);
        let mut child = match self.pty {
            Some(size) => spawn_merged_on_pty(command, size, group, &merged)?,
            None => spawn_merged(command, self.out.as_ref(), group, &merged)?,
        };
        let deadline = limit.map(|limit| Instant::now() + limit);
        // The stream ending is not the child ending: it may have closed its output and kept
        // running. The loop waits in short steps and asks both again in each of them, so nothing
        // waits for a child that will not answer.
        // When the child itself ended while something it started still holds the output open.
        let mut ended: Option<(Instant, Option<i32>)> = None;
        let (outcome, timed_out, cancelled) = loop {
            if cancel() {
                stop(&mut child, group, grace, &merged);
                break (ProcessOutcome::Cancelled, false, true);
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                stop(&mut child, group, grace, &merged);
                break (ProcessOutcome::Finished { code: None }, true, false);
            }
            let open = lock(&merged).open;
            if let Some(on_tail) = on_tail.as_mut()
                && told_at.elapsed() >= WATCH_EVERY
            {
                let fresh = {
                    let mut merged = lock(&merged);
                    (merged.fed != told).then(|| (merged.fed, merged.tail.text()))
                };
                if let Some((fed, text)) = fresh {
                    on_tail(&text);
                    (told, told_at) = (fed, Instant::now());
                }
            }
            if ended.is_none()
                && let Some(status) = child.try_wait()?
            {
                ended = Some((Instant::now(), status.code()));
            }
            if let Some((at, code)) = ended {
                if !open {
                    break (ProcessOutcome::Finished { code }, false, false);
                }
                // What the child left behind keeps the output open; past the wait it goes, and
                // the command is reported as the command ended.
                if let Some(wait) = after_exit
                    && at.elapsed() >= wait
                {
                    kill(&mut child, group);
                    break (ProcessOutcome::Finished { code }, false, false);
                }
            }
            std::thread::sleep(POLL);
        };
        let mut merged = lock(&merged);
        // The last of the output may have come after the watcher was last told; it hears the end.
        if let Some(on_tail) = on_tail
            && merged.fed != told
        {
            on_tail(&merged.tail.text());
        }
        Ok(Collected { text: merged.tail.text(), trimmed: merged.tail.trimmed, outcome, timed_out, cancelled })
    }

    /// The command line with everything but the streams: the group the child runs in, where it
    /// runs, and the environment it is given.
    fn command(&self, group: bool) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
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
        #[cfg(not(unix))]
        let _ = group;
        if let Some(dir) = &self.dir {
            command.current_dir(dir);
        }
        if self.cleared {
            command.env_clear();
        }
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command
    }
}

/// What a reading thread hands to the loop in [`Process::run_inner`]. One channel carries both
/// so lines and frames keep the order the child wrote them in.
enum Sent {
    Line(Line),
    Overwritten(Line),
}

/// Asks the child, and with `group` its whole process group, to end with `TERM`, gives it `grace`
/// to do so and to say its last words, then kills what is left. A build or a test run that is
/// stopped can clean up after itself this way; one that does not listen is ended all the same.
fn stop(child: &mut Child, group: bool, grace: Duration, merged: &Mutex<Merged>) {
    #[cfg(unix)]
    {
        let pid = rustix::process::Pid::from_child(child);
        // Fails only when nothing is left to ask; the kill below ends what is there in any case.
        let _ = if group {
            rustix::process::kill_process_group(pid, rustix::process::Signal::TERM)
        } else {
            rustix::process::kill_process(pid, rustix::process::Signal::TERM)
        };
        let end = Instant::now() + grace;
        // The child ending is not all of it: what it wrote on its way out is still in the pipe,
        // so the wait goes on until the output closes too, or the grace is over.
        while Instant::now() < end && (child.try_wait().ok().flatten().is_none() || lock(merged).open) {
            std::thread::sleep(POLL);
        }
    }
    #[cfg(not(unix))]
    let _ = (grace, merged);
    kill(child, group);
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
    // The kill fails only on a child that ended by itself, which is what was wanted, and the
    // wait collects it rather than being asked anything. This function has no answer to give:
    // its callers have already decided the child is to go.
    let _ = child.kill();
    let _ = child.wait();
}

/// Starts the child with a pipe per stream and a reading thread for each, so a failure stays
/// recognisable as one. A file named for the output is that stream as it is: its descriptor is
/// cloned for the child and only the error stream has a reader, so a failure is all that reaches
/// the application.
fn spawn_on_pipes(
    mut command: Command,
    out: Option<&Out>,
    sender: &SyncSender<Sent>,
    frames: bool,
    group: bool,
) -> io::Result<Child> {
    match out {
        // The child's own descriptor for the file, so it writes at the position this one has and
        // the application keeps its end where it was.
        Some(file) => command.stdout(Stdio::from(file.0.try_clone()?)),
        None => command.stdout(Stdio::piped()),
    };
    command.stderr(Stdio::piped());
    let mut child = command.spawn()?;
    drop(command);
    let started = match out {
        Some(_) => match child.stderr.take() {
            Some(err) => spawn_reader("err", err, Line::Err, frames, sender.clone()),
            None => Err(io::Error::other("the child was started without its pipes")),
        },
        None => match child.stdout.take().zip(child.stderr.take()) {
            Some((out, err)) => spawn_reader("out", out, Line::Out, frames, sender.clone())
                .and_then(|()| spawn_reader("err", err, Line::Err, frames, sender.clone())),
            None => Err(io::Error::other("the child was started without its pipes")),
        },
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
    size: (u16, u16),
    sender: &SyncSender<Sent>,
    frames: bool,
    group: bool,
) -> io::Result<Child> {
    let terminal = open_pty(&mut command, size)?;
    let mut child = command.spawn()?;
    // The command holds the child's side of the terminal until it is dropped, and while it is
    // open the reading side never reaches its end of file.
    drop(command);
    match spawn_reader("pty", terminal, Line::Out, frames, sender.clone()) {
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
    mut command: Command,
    size: (u16, u16),
    _sender: &SyncSender<Sent>,
    _frames: bool,
    _group: bool,
) -> io::Result<Child> {
    open_pty(&mut command, size)
}

/// Starts the child with one pipe carrying both of its streams, so what it writes is read in the
/// order it wrote it, and a thread that keeps only the end of it. A file named for the output is
/// that stream as it is, so the pipe carries the error stream alone and the file gets the rest.
fn spawn_merged(mut command: Command, out: Option<&Out>, group: bool, merged: &Shared) -> io::Result<Child> {
    let (reader, writer) = io::pipe()?;
    command.stdout(match out {
        Some(file) => Stdio::from(file.0.try_clone()?),
        None => Stdio::from(writer.try_clone()?),
    });
    command.stderr(writer);
    let mut child = command.spawn()?;
    // The command holds the write end of the pipe until it is dropped, and while it is open the
    // reading end never reaches its end of file.
    drop(command);
    match spawn_collector(reader, Arc::clone(merged)) {
        Ok(()) => Ok(child),
        Err(error) => {
            kill(&mut child, group);
            Err(error)
        }
    }
}

/// Starts the child on a pseudo-terminal, where both of its streams land on the one stream
/// already, and keeps the end of that.
#[cfg(unix)]
fn spawn_merged_on_pty(mut command: Command, size: (u16, u16), group: bool, merged: &Shared) -> io::Result<Child> {
    let terminal = open_pty(&mut command, size)?;
    let mut child = command.spawn()?;
    // The command holds the child's side of the terminal until it is dropped, and while it is
    // open the reading side never reaches its end of file.
    drop(command);
    match spawn_collector(terminal, Arc::clone(merged)) {
        Ok(()) => Ok(child),
        Err(error) => {
            kill(&mut child, group);
            Err(error)
        }
    }
}

/// Without Unix there is no pseudo-terminal, as in [`spawn_on_pty`].
#[cfg(not(unix))]
fn spawn_merged_on_pty(mut command: Command, size: (u16, u16), _group: bool, _merged: &Shared) -> io::Result<Child> {
    open_pty(&mut command, size)
}

/// Opens a pseudo-terminal `cols` wide and `rows` tall, gives the child both of its streams on
/// it and hands back the one stream they land on.
#[cfg(unix)]
fn open_pty(command: &mut Command, (cols, rows): (u16, u16)) -> io::Result<std::fs::File> {
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
    Ok(File::from(controller))
}

/// Without Unix there is no pseudo-terminal to open, so the caller is told instead of being
/// given a child that quietly sees no terminal.
#[cfg(not(unix))]
fn open_pty(_command: &mut Command, _size: (u16, u16)) -> io::Result<std::fs::File> {
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

/// Starts a thread that keeps the end of `source` in `merged`, which the caller reads as the
/// child writes.
fn spawn_collector(source: impl Read + Send + 'static, merged: Shared) -> io::Result<()> {
    std::thread::Builder::new()
        .name("quvyta-process-merged".to_owned())
        .spawn(move || read_tail(source, &merged))
        .map(|_| ())
}

/// The one stream both of a collected child's streams land on: what is worth keeping of it, and
/// whether the thread reading it still holds it. The thread fills this in and the caller reads
/// it, so the caller can take what has been kept the moment it stops the child.
#[derive(Debug)]
struct Merged {
    tail: Tail,
    open: bool,
    /// Counts the reads fed to the tail, so a watcher is told only about output it has not seen.
    fed: u64,
}

/// The shortest time between two calls of a [`Process::collect_watching`] watcher.
const WATCH_EVERY: Duration = Duration::from_millis(100);

/// The one stream, shared by the thread that reads it and the caller that waits for it.
type Shared = Arc<Mutex<Merged>>;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keeps what one read brought in `merged` until the stream ends, and lets go of it only then,
/// so the caller never hears the end before the last of the output is in.
fn read_tail(mut source: impl Read, merged: &Shared) {
    let mut chunk = [0_u8; CHUNK];
    loop {
        match source.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                let mut merged = lock(merged);
                merged.tail.feed(&chunk[..count]);
                merged.fed += 1;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            // A pseudo-terminal answers with an I/O error once the child's side is gone, and a
            // broken pipe says the same thing; both are the end of the stream.
            Err(_) => break,
        }
    }
    lock(merged).open = false;
}

/// The end of a byte stream: at most the bytes asked for, and at most the lines asked for when
/// there are any, the oldest dropped as the newest arrives. Nothing is held beyond this and one
/// read, so a program that writes for an hour costs what it is allowed to.
#[derive(Debug)]
struct Tail {
    bytes: VecDeque<u8>,
    /// How many line ends the kept bytes hold.
    lines: usize,
    /// Whether the last byte read was a line end, so nothing is being written after it.
    ends_line: bool,
    /// How many bytes are kept.
    kept: usize,
    /// How many lines are kept, when the caller named a count.
    lines_kept: Option<usize>,
    /// Whether anything was dropped for being older than what is kept.
    trimmed: bool,
}

impl Tail {
    /// A tail keeping `bytes` bytes, and `lines` lines as well when the caller named a count.
    fn new(bytes: usize, lines: Option<usize>) -> Self {
        Self { bytes: VecDeque::new(), lines: 0, ends_line: true, kept: bytes, lines_kept: lines, trimmed: false }
    }

    /// Adds what one read brought, dropping the oldest until what is kept is within what was
    /// asked for.
    fn feed(&mut self, bytes: &[u8]) {
        let Some(&last) = bytes.last() else {
            return;
        };
        self.ends_line = last == b'\n';
        self.lines += bytes.iter().filter(|&&byte| byte == b'\n').count();
        self.bytes.extend(bytes);
        if let Some(kept) = self.lines_kept {
            for _ in 0..self.counting().saturating_sub(kept) {
                self.drop_oldest_line();
            }
        }
        for _ in 0..self.bytes.len().saturating_sub(self.kept) {
            if self.bytes.pop_front() == Some(b'\n') {
                self.lines -= 1;
            }
            self.trimmed = true;
        }
    }

    /// How many lines the kept bytes hold: the line ends in them, and the line being written,
    /// which has no end yet.
    fn counting(&self) -> usize {
        self.lines + usize::from(!self.ends_line && !self.bytes.is_empty())
    }

    /// Drops the oldest line, whatever is left of it and its end with it.
    fn drop_oldest_line(&mut self) {
        self.trimmed = true;
        while let Some(byte) = self.bytes.pop_front() {
            if byte == b'\n' {
                self.lines -= 1;
                break;
            }
        }
    }

    /// The kept bytes as text, from a character: a cut made for the size asked for can leave the
    /// first half of a character behind, and half a character is no text.
    fn text(&mut self) -> String {
        let bytes = self.bytes.make_contiguous();
        let start = bytes.iter().copied().take(3).take_while(|byte| byte & 0b1100_0000 == 0b1000_0000).count();
        String::from_utf8_lossy(&bytes[start..]).into_owned()
    }
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
    use std::fs::File;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::{Collected, Keep, Line, Lines, MAX_LINE, Process, ProcessOutcome, Tail};

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

    /// Runs `process` to its end and keeps only the end of what it wrote, never cancelling.
    fn collect(process: Process, keep: Keep) -> Collected {
        process.collect(keep, &|| false).expect("the shell starts")
    }

    /// A folder of its own for a test, empty at the start, so nothing is written to the folders of
    /// the person the tests run as.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-process-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("test folder");
        dir
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
    fn a_cleared_environment_gives_the_child_only_what_it_was_told() {
        // A test process cannot set a variable for itself, since `set_var` is unsafe, so the
        // home it really runs with is what the first run reads.
        let script = r#"printf '%s' "${HOME-unset}""#;
        let (lines, _) = shell(script);
        assert_ne!(lines, vec![Line::Out("unset".to_owned())], "the test process really has a home");
        let path = std::env::var("PATH").expect("the test process was started with a path");
        let (lines, _) = run(Process::new("sh").args(["-c", script]).clear_env().env("PATH", path.clone()));
        assert_eq!(lines, vec![Line::Out("unset".to_owned())], "nothing is inherited");
        // What was named is what the child gets, the path included: it looks other programs up
        // with that.
        let (lines, _) = run(Process::new("sh")
            .args(["-c", r#"printf '%s' "${PATH-unset}""#])
            .clear_env()
            .env("PATH", path.clone()));
        assert_eq!(lines, vec![Line::Out(path)]);
        let (lines, _) =
            run(Process::new("sh").args(["-c", r#"printf '%s' "${EV-unset}""#]).clear_env().env("EV", "1"));
        assert_eq!(lines, vec![Line::Out("1".to_owned())], "a variable that was given arrives");
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
        let dir = folder("flood");
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

    #[test]
    fn collected_output_arrives_in_the_order_both_streams_were_written() {
        let script = "printf a; printf b >&2; printf c";
        let collected = collect(Process::new("sh").args(["-c", script]), Keep::bytes(64));
        assert_eq!(collected.text, "abc");
        assert_eq!(collected.outcome, ProcessOutcome::Finished { code: Some(0) });
        assert!(!collected.trimmed, "nothing was dropped: {}", collected.text);
        assert!(!collected.timed_out && !collected.cancelled);
        // A pseudo-terminal is the one place the two streams share a line anyway, and it comes
        // through the same one pipe.
        #[cfg(unix)]
        {
            let collected = collect(Process::new("sh").args(["-c", script]).pty(80, 24), Keep::bytes(64));
            assert_eq!(collected.text, "abc");
        }
    }

    #[test]
    fn only_the_last_bytes_of_a_flood_of_output_are_kept() {
        // Eight megabytes through, sixty-four kilobytes out: the end of it, and nothing else.
        let script = "head -c 8388608 /dev/zero | tr '\\0' x; printf 'SON'";
        let collected = collect(Process::new("sh").args(["-c", script]), Keep::bytes(65536));
        assert_eq!(collected.text.len(), 65536);
        assert!(collected.text.ends_with("SON"), "the newest bytes are the ones kept");
        assert!(collected.text[..65533].bytes().all(|byte| byte == b'x'), "only the flood is before them");
        assert!(collected.trimmed, "eight megabytes did not fit in sixty-four");
    }

    #[test]
    fn the_lines_asked_for_are_the_last_ones() {
        let script = "for name in bir iki uc dort bes; do printf '%s\\n' \"$name\"; done";
        let collected = collect(Process::new("sh").args(["-c", script]), Keep::bytes(4096).lines(3));
        assert_eq!(collected.text, "uc\ndort\nbes\n");
        assert!(collected.trimmed);
        // The line a program is still writing counts as one, so nothing is lost while it writes.
        let collected = collect(Process::new("sh").args(["-c", r"printf 'bir\niki\nuc'"]), Keep::bytes(4096).lines(3));
        assert_eq!(collected.text, "bir\niki\nuc");
        assert!(!collected.trimmed);
        // And bytes still decide when both are asked for.
        let collected = collect(Process::new("sh").args(["-c", script]), Keep::bytes(9).lines(3));
        assert_eq!(collected.text, "dort\nbes\n");
    }

    #[test]
    fn a_tail_keeps_the_end_of_what_it_is_fed_and_never_half_a_character() {
        let mut tail = Tail::new(16, None);
        tail.feed(b"bir iki ");
        tail.feed(b"uc dort");
        assert_eq!(tail.text(), "bir iki uc dort");
        assert!(!tail.trimmed, "nothing fell out of sixteen bytes");
        tail.feed(b"!\n");
        assert_eq!(tail.text(), "ir iki uc dort!\n", "the oldest byte fell out");
        assert!(tail.trimmed);
        // One `ç` at a time, so its two bytes straddle every cut.
        let mut tail = Tail::new(5, None);
        for _ in 0..8 {
            tail.feed("ç".as_bytes());
        }
        assert_eq!(tail.text(), "çç", "the half a character at the front is dropped");
    }

    #[test]
    fn a_tail_keeps_the_last_lines_it_was_given() {
        let mut tail = Tail::new(64, Some(2));
        for name in ["bir\n", "iki\n", "uc\n", "dort"] {
            tail.feed(name.as_bytes());
        }
        assert_eq!(tail.text(), "uc\ndort", "the line being written counts as one");
        assert!(tail.trimmed);
    }

    #[test]
    fn collecting_nothing_but_the_outcome_is_allowed() {
        let collected = collect(Process::new("sh").args(["-c", "printf 'gorunmez'"]), Keep::bytes(0));
        assert_eq!(collected.text, "");
        assert_eq!(collected.outcome, ProcessOutcome::Finished { code: Some(0) });
        assert!(collected.trimmed);
    }

    #[test]
    fn the_standard_output_can_go_straight_to_a_file_and_only_the_error_stream_is_kept() {
        let dir = folder("into-file");
        let path = dir.join("unpacked.bin");
        // Eight megabytes of output and three hundred lines of failure, written one after the
        // other, so what the file holds and what is kept cannot be each other's.
        let script = "head -c 8388608 /dev/zero; \
                      n=0; while [ \"$n\" -lt 300 ]; do printf 'satir %s\\n' \"$n\" >&2; n=$((n+1)); done";
        let file = File::create(&path).expect("the file is created");
        let collected = collect(Process::new("sh").args(["-c", script]).stdout_to(file), Keep::bytes(64));
        assert_eq!(collected.outcome, ProcessOutcome::Finished { code: Some(0) });
        assert_eq!(
            std::fs::metadata(&path).expect("the file is there").len(),
            8_388_608,
            "every byte of it is in the file"
        );
        // Only the error stream was read, and only the end of it: the flood never passed through.
        assert_eq!(collected.text.len(), 64, "{}", collected.text);
        assert!(collected.text.ends_with("satir 299\n"), "the newest line is kept: {}", collected.text);
        assert!(!collected.text.contains("satir 1"), "the older lines fell out: {}", collected.text);
        assert!(collected.trimmed);
        std::fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_file_named_for_the_output_leaves_the_error_stream_the_only_one_to_read() {
        let dir = folder("into-file-lines");
        let path = dir.join("out.txt");
        let file = File::create(&path).expect("the file is created");
        let (lines, outcome) = run(Process::new("sh").args(["-c", "printf cikti; printf hata >&2"]).stdout_to(file));
        assert_eq!(lines, vec![Line::Err("hata".to_owned())], "the output went to the file: {lines:?}");
        assert_eq!(std::fs::read_to_string(&path).expect("the file is there"), "cikti");
        assert_eq!(outcome, ProcessOutcome::Finished { code: Some(0) });
        std::fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_cancelled_child_writes_nothing_more_to_the_file() {
        let dir = folder("into-file-cancel");
        let path = dir.join("flood.bin");
        let file = File::create(&path).expect("the file is created");
        let script = "while :; do printf '0123456789012345678901234567890123456789'; done";
        let started = std::time::Instant::now();
        let process = Process::new("sh").args(["-c", script]).no_stdin().stdout_to(file);
        let collected = process
            .collect(Keep::bytes(1024), &|| started.elapsed() > Duration::from_millis(500))
            .expect("the shell starts");
        assert!(collected.cancelled && !collected.timed_out, "{collected:?}");
        let size = || std::fs::metadata(&path).expect("the file is there").len();
        let stopped = size();
        assert!(stopped > 0, "the child wrote into the file before it was cancelled");
        // The process group went with the cancel, so the file cannot grow again. Generous, and
        // still finite: a child that is really gone writes nothing at all.
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(200));
            assert_eq!(size(), stopped, "the file grew after the child was cancelled");
        }
        std::fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_command_that_leaves_something_behind_is_done_once_it_has_ended() {
        let dir = folder("after-exit");
        let started = std::time::Instant::now();
        let collected = Process::new("sh")
            .args(["-c", "(sleep 4; printf late > late.txt) & printf done"])
            .dir(&dir)
            .no_stdin()
            .collect(Keep::bytes(1024).after_exit(Duration::from_secs(2)), &|| false)
            .expect("the shell starts");
        assert!(started.elapsed() < Duration::from_secs(15), "it did not wait for what it left behind");
        assert_eq!(collected.text, "done");
        assert_eq!(collected.outcome, ProcessOutcome::Finished { code: Some(0) });
        assert!(!collected.timed_out && !collected.cancelled, "{collected:?}");
        // What it left behind went with its group, so it never writes its file.
        std::thread::sleep(Duration::from_secs(6));
        assert!(!dir.join("late.txt").exists(), "the program left behind was ended");
        std::fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_stopped_child_is_asked_first_and_has_its_last_words_kept() {
        let started = std::time::Instant::now();
        let collected = Process::new("sh")
            .args(["-c", "trap 'printf bye; exit 0' TERM; sleep 30 & wait"])
            .no_stdin()
            .collect(Keep::bytes(1024), &|| started.elapsed() > Duration::from_secs(1))
            .expect("the shell starts");
        assert!(collected.cancelled, "{collected:?}");
        assert!(collected.text.contains("bye"), "TERM came first and what it said is kept: {collected:?}");
        assert!(started.elapsed() < Duration::from_secs(15), "and the stop is bounded");
    }

    #[test]
    fn a_watcher_sees_the_output_go_by_while_the_child_runs() {
        let mut seen = Vec::new();
        let collected = Process::new("sh")
            .args(["-c", "printf a; sleep 1; printf b; sleep 1; printf c"])
            .no_stdin()
            .collect_watching(Keep::bytes(1024), &|| false, |tail| seen.push(tail.to_owned()))
            .expect("the shell starts");
        assert_eq!(collected.text, "abc");
        assert!(seen.len() >= 2, "told more than once while it ran: {seen:?}");
        assert!(seen.iter().any(|tail| tail.contains('a') && !tail.contains('c')), "before the end: {seen:?}");
        assert_eq!(seen.last().map(String::as_str), Some("abc"), "and the last call has it all: {seen:?}");
    }

    #[test]
    fn a_watcher_sees_no_more_lines_than_are_kept_and_nothing_of_a_silent_child() {
        let mut seen = Vec::new();
        Process::new("sh")
            .args(["-c", "printf '1\\n2\\n'; sleep 0.3; printf '3\\n4\\n'"])
            .no_stdin()
            .collect_watching(Keep::bytes(1024).lines(2), &|| false, |tail| seen.push(tail.to_owned()))
            .expect("the shell starts");
        assert!(!seen.is_empty());
        assert!(seen.iter().all(|tail| tail.lines().count() <= 2), "{seen:?}");
        let mut silent = 0;
        Process::new("sleep")
            .args(["1"])
            .no_stdin()
            .collect_watching(Keep::bytes(1024), &|| false, |_| silent += 1)
            .expect("sleep starts");
        assert_eq!(silent, 0, "nothing written, nothing told");
    }

    #[test]
    fn cancelling_a_collected_child_stops_it_the_way_a_limit_does() {
        // The child prints the program it leaves behind first, so on Linux the test can look for it
        // in `/proc` without running `ps`.
        let script = "sleep 30 & printf 'hazir\\n%s\\n' \"$!\"; sleep 30";
        let started = std::time::Instant::now();
        let process = Process::new("sh").args(["-c", script]).no_stdin();
        let collected = process
            .collect(Keep::bytes(1024).limit(Duration::from_secs(30)), &|| started.elapsed() > Duration::from_secs(1))
            .expect("the shell starts");
        assert!(collected.cancelled && !collected.timed_out, "{collected:?}");
        assert_eq!(collected.outcome, ProcessOutcome::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(15), "took {:?}", started.elapsed());
        let mut lines = collected.text.lines();
        assert_eq!(lines.next(), Some("hazir"), "what was written before the cancel is still there: {collected:?}");
        // The child is in a process group of its own, so the cancel reached the `sleep` too.
        #[cfg(target_os = "linux")]
        {
            let pid = lines.next().unwrap_or_default().to_owned();
            assert!(pid.bytes().all(|byte| byte.is_ascii_digit()), "a process id: {pid:?}");
            let started = std::time::Instant::now();
            while !ended(&pid) {
                assert!(started.elapsed() < Duration::from_secs(20), "the `sleep` is still running: {pid}");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_limit_ends_the_child_and_the_programs_it_started() {
        // The child prints the program it leaves behind first, so the test can look for it in
        // `/proc` without running `ps`.
        let script = "sleep 30 & printf '%s\\n' \"$!\"; sleep 30";
        let started = std::time::Instant::now();
        let process = Process::new("sh").args(["-c", script]).no_stdin();
        let collected =
            process.collect(Keep::bytes(1024).limit(Duration::from_secs(1)), &|| false).expect("the shell starts");
        assert!(collected.timed_out && !collected.cancelled, "{collected:?}");
        assert_eq!(collected.outcome, ProcessOutcome::Finished { code: None }, "a signal ended it");
        assert!(started.elapsed() < Duration::from_secs(15), "took {:?}", started.elapsed());
        // The child is in a process group of its own, so the limit reached the `sleep` too.
        let pid = collected.text.trim().to_owned();
        assert!(pid.bytes().all(|byte| byte.is_ascii_digit()), "a process id: {pid:?}");
        let started = std::time::Instant::now();
        while !ended(&pid) {
            assert!(started.elapsed() < Duration::from_secs(20), "the `sleep` is still running: {pid}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
