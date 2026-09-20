//! A process running in a pseudo-terminal, its output parsed into a screen of cells.

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};

use super::terminal_notice::{Notices, OscLimit};

/// Lines kept above the screen for scrolling back, unless [`TerminalBuilder::scrollback`] says
/// otherwise.
const SCROLLBACK: usize = 5000;

/// The screen a program starts on, rows and columns, unless [`TerminalBuilder::size`] says
/// otherwise; a [`Terminal`](super::Terminal) widget resizes it to its own size.
const SIZE: (u16, u16) = (24, 80);

/// Tells a program that what follows was pasted, not typed, when it turned bracketed paste on.
const PASTE_START: &str = "\x1b[200~";

/// Tells the program the pasted text ends here.
const PASTE_END: &str = "\x1b[201~";

/// The text of a paste without the two markers, so nothing inside it can end the paste early or
/// start a second one. Borrows the text when it holds neither.
fn without_markers(text: &str) -> Cow<'_, str> {
    if !text.contains(PASTE_START) && !text.contains(PASTE_END) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(text.replace(PASTE_START, "").replace(PASTE_END, ""))
}

/// Something that changed in a [`TerminalSession`], see [`TerminalWatch::next`].
///
/// Only output and the end of the program; [`TerminalWatch::next_change`] also reports what the
/// program says about itself, as a [`TerminalChange`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalEvent {
    /// New output was parsed; draw again.
    Output,
    /// The process ended with this exit code, or the session was dropped (`None`).
    Exited(Option<u32>),
}

/// Something that changed in a [`TerminalSession`], see [`TerminalWatch::next_change`].
///
/// Besides output and the end of the program, what the program says about itself through
/// escape sequences: its title, its folder, the bell and notifications. The notices are
/// heard in the output that also brings an [`Output`](Self::Output).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TerminalChange {
    /// New output was parsed; draw again.
    Output,
    /// The program set its window title (OSC 0 or 2). Empty when it cleared it.
    Title(String),
    /// The program reported its working folder (OSC 7, `file://host/path`), percent-decoded.
    /// Shells send it when set up to, usually at every prompt.
    WorkingFolder(PathBuf),
    /// The program rang the bell (BEL outside an escape sequence). Rings the application has not
    /// read yet count as one.
    Bell,
    /// The program asked for a desktop notification: OSC 9 (`body` only) or OSC 777
    /// (`notify;title;body`). Unread ones beyond the newest eight are dropped.
    Notify {
        /// The title, when the program gave one.
        title: Option<String>,
        /// The message.
        body: String,
    },
    /// The process ended with this exit code, or the session was dropped (`None`).
    Exited(Option<u32>),
}

/// What the reader thread, the watch and the widget share.
struct Shared {
    parser: Mutex<vt100::Parser<Notices>>,
    signal: Mutex<Signal>,
    changed: Condvar,
    /// The shortest time between two reports of output.
    coalesce: Duration,
    /// When the program last wrote and when it was last written to. Its own lock: the reader
    /// thread marks output without waiting for a watch, and the times are read while drawing.
    quiet: Mutex<Quiet>,
}

/// When the two sides of a session last said something; see [`TerminalSession::last_output`] and
/// [`TerminalSession::last_input`].
#[derive(Debug, Clone, Copy)]
struct Quiet {
    /// When the reader thread last took bytes from the program.
    output: Instant,
    /// When keys were last written to the program by the person at the keyboard.
    input: Instant,
}

#[derive(Default)]
struct Signal {
    /// Bumped for every chunk of output.
    generation: u64,
    /// The generation the watch reported last.
    seen: u64,
    /// When the watch last reported output.
    reported: Option<Instant>,
    /// Notices heard and not reported yet.
    notices: Notices,
    exited: Option<Option<u32>>,
    /// The size the widget last asked for, applied by the watch off the render path.
    wanted: Option<(u16, u16)>,
    size: (u16, u16),
}

/// The process side: writing input, resizing and ending it.
struct Process {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

impl Drop for Process {
    fn drop(&mut self) {
        // The last handle is gone: end the process so its reader and watch finish too.
        let _ = self.killer.kill();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic in another thread must not take the terminal down with it; the data stays usable.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How to start a [`TerminalSession`]: program, arguments, folder, environment, first size,
/// scrollback and how often output is reported. Made by [`TerminalSession::builder`].
///
/// Every option is independent and has the default [`TerminalSession::spawn`] uses.
#[derive(Debug, Clone)]
#[must_use]
pub struct TerminalBuilder {
    program: OsString,
    args: Vec<OsString>,
    folder: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    size: (u16, u16),
    scrollback: usize,
    coalesce: Duration,
}

impl TerminalBuilder {
    /// Adds arguments after the ones already given. Default: none.
    pub fn args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.args.extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));
        self
    }

    /// The folder the program starts in. Default: the user's home folder, which is also used
    /// when `folder` is not a folder.
    pub fn folder(mut self, folder: impl AsRef<Path>) -> Self {
        self.folder = Some(folder.as_ref().to_owned());
        self
    }

    /// Sets an environment variable for the program, on top of the application's own
    /// environment. The program always gets `TERM=xterm-256color` and `COLORTERM=truecolor`;
    /// setting either here replaces it. A name set twice keeps the last value.
    pub fn env(mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env.push((name.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }

    /// The screen size the program sees when it starts, in columns and rows, so its first
    /// drawing already fits; give the size the widget will have. A widget showing the session
    /// resizes it to its own size later. Sizes below 2 count as 2. Default: 80 × 24.
    pub fn size(mut self, columns: u16, rows: u16) -> Self {
        self.size = (rows, columns);
        self
    }

    /// Lines kept above the screen for scrolling back; `0` keeps none. Each kept line holds its
    /// cells, so a wide, full screen costs a few kilobytes per line. Default: 5000.
    pub fn scrollback(mut self, lines: usize) -> Self {
        self.scrollback = lines;
        self
    }

    /// Reports output at most once per `interval`: a program writing fast, such as `yes` or a
    /// busy log, asks for one frame per interval instead of one per read. Output is never held
    /// back longer than `interval`. Other changes are not delayed. Default: zero, every read is
    /// reported as soon as the watch runs.
    pub fn coalesce(mut self, interval: Duration) -> Self {
        self.coalesce = interval;
        self
    }

    /// Starts the program.
    ///
    /// # Errors
    ///
    /// Fails when no pseudo-terminal can be opened or the program cannot start.
    pub fn spawn(self) -> io::Result<TerminalSession> {
        let size = (self.size.0.max(2), self.size.1.max(2));
        let pair = native_pty_system().openpty(pty_size(size)).map_err(io::Error::other)?;
        let mut command = CommandBuilder::new(&self.program);
        command.args(&self.args);
        if let Some(folder) = &self.folder {
            command.cwd(folder);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        for (name, value) in &self.env {
            command.env(name, value);
        }
        let mut child = pair.slave.spawn_command(command).map_err(io::Error::other)?;
        // Without the slave end here, the reader sees end-of-file once the child exits.
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let writer = pair.master.take_writer().map_err(io::Error::other)?;
        let killer = child.clone_killer();
        let pid = child.process_id();

        let started = Instant::now();
        let shared = Arc::new(Shared {
            parser: Mutex::new(vt100::Parser::new_with_callbacks(size.0, size.1, self.scrollback, Notices::default())),
            signal: Mutex::new(Signal { size, ..Signal::default() }),
            changed: Condvar::new(),
            coalesce: self.coalesce,
            quiet: Mutex::new(Quiet { output: started, input: started }),
        });
        let thread_shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            let mut limit = OscLimit::default();
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        // Marked where the bytes arrive, before they are parsed: what a caller
                        // asks is when the program last wrote, not when a frame was drawn.
                        lock(&thread_shared.quiet).output = Instant::now();
                        let heard = {
                            let mut parser = lock(&thread_shared.parser);
                            limit.feed(&buffer[..count], |run| parser.process(run));
                            std::mem::take(parser.callbacks_mut())
                        };
                        let mut signal = lock(&thread_shared.signal);
                        // A watch only needs waking for the first unreported chunk: while one is
                        // pending it is already on its way, or waiting out the coalescing.
                        let wake = signal.generation == signal.seen || !heard.is_empty();
                        signal.generation += 1;
                        signal.notices.merge(heard);
                        drop(signal);
                        if wake {
                            thread_shared.changed.notify_all();
                        }
                    }
                }
            }
            let code = child.wait().ok().map(|status| status.exit_code());
            lock(&thread_shared.signal).exited = Some(code);
            thread_shared.changed.notify_all();
        });
        let process = Process { master: pair.master, writer, killer };
        Ok(TerminalSession { shared, process: Arc::new(Mutex::new(process)), pid })
    }
}

/// A program (usually a shell) running in a pseudo-terminal.
///
/// Output is read on a background thread and parsed into a screen with scrollback; a
/// [`Terminal`](super::Terminal) widget draws that screen and sends keys to the program.
/// Handles are cheap to clone; when the last one is dropped the program is ended.
///
/// The runtime learns about new output through a [`TerminalWatch`]: run
/// [`TerminalWatch::next`] in a [`Command::perform`](crate::runtime::Command::perform), and
/// start it again when its message arrives, until it reports [`TerminalEvent::Exited`].
/// [`TerminalWatch::next_change`] also reports the program's title, folder, bell and
/// notifications. [`TerminalSession::builder`] sets the environment, the first size, the
/// scrollback and how often output is reported.
#[derive(Clone)]
pub struct TerminalSession {
    shared: Arc<Shared>,
    process: Arc<Mutex<Process>>,
    pid: Option<u32>,
}

impl std::fmt::Debug for TerminalSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalSession").field("exited", &self.exit()).finish_non_exhaustive()
    }
}

impl TerminalSession {
    /// Starts the user's shell (`$SHELL`, else `/bin/sh`) in `folder`.
    ///
    /// # Errors
    ///
    /// Fails when no pseudo-terminal can be opened or the shell cannot start.
    pub fn shell(folder: &Path) -> io::Result<Self> {
        let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
        Self::spawn(&shell, &[] as &[&str], folder)
    }

    /// Starts `program` with `args` in `folder`, on a 24 × 80 screen until a widget shows it.
    /// The short form of [`TerminalSession::builder`] with every other option at its default.
    ///
    /// # Errors
    ///
    /// Fails when no pseudo-terminal can be opened or the program cannot start.
    pub fn spawn(program: &OsStr, args: &[impl AsRef<OsStr>], folder: &Path) -> io::Result<Self> {
        Self::builder(program).args(args).folder(folder).spawn()
    }

    /// Starts describing a session for `program`; finish with [`TerminalBuilder::spawn`].
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use qframe::widgets::TerminalSession;
    /// let session = TerminalSession::builder("/bin/sh")
    ///     .args(["-l"])
    ///     .folder("/tmp")
    ///     .env("EDITOR", "vi")
    ///     .size(100, 30)
    ///     .scrollback(2000)
    ///     .coalesce(Duration::from_millis(16))
    ///     .spawn()?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn builder(program: impl AsRef<OsStr>) -> TerminalBuilder {
        TerminalBuilder {
            program: program.as_ref().to_owned(),
            args: Vec::new(),
            folder: None,
            env: Vec::new(),
            size: SIZE,
            scrollback: SCROLLBACK,
            coalesce: Duration::ZERO,
        }
    }

    /// A watch that reports output and the end of the program; see [`TerminalWatch::next`].
    #[must_use]
    pub fn watch(&self) -> TerminalWatch {
        TerminalWatch { shared: Arc::clone(&self.shared), process: Arc::downgrade(&self.process) }
    }

    /// Sends bytes to the program as if typed, which is also what
    /// [`last_input`](Self::last_input) reports: this is the path a keystroke takes, so anything
    /// sent here counts as the person's input. To write on the application's own behalf without
    /// being mistaken for the person, use [`paste`](Self::paste).
    ///
    /// # Errors
    ///
    /// Fails when the program no longer reads its input: once its end is known the write is
    /// refused, because a pseudo-terminal keeps taking bytes that nobody will ever read.
    pub fn write(&self, bytes: &[u8]) -> io::Result<()> {
        self.send(bytes)?;
        lock(&self.shared.quiet).input = Instant::now();
        Ok(())
    }

    /// Sends `text` as a paste: wrapped in the bracketed-paste markers when the program turned
    /// that mode on, and as plain text when it did not. A program that asked for the markers then
    /// knows the text was pasted and does not read its line breaks as Enter, so a whole message
    /// arrives as one piece instead of as several half-finished lines.
    ///
    /// The markers are removed from `text` first, both of them: a `\x1b[201~` inside it would end
    /// the paste early and leave the rest to be read as keys, and a `\x1b[200~` would start a
    /// second paste inside the first. This matters when the text comes from somewhere else, such
    /// as another program's output or a message from another part of the system.
    ///
    /// A paste is the application writing, not the person typing, so it does not move
    /// [`last_input`](Self::last_input). A person pasting into a
    /// [`Terminal`](super::Terminal) widget does move it: the person is at the keyboard there.
    ///
    /// # Errors
    ///
    /// Fails when the program no longer reads its input, as [`write`](Self::write) does.
    pub fn paste(&self, text: &str) -> io::Result<()> {
        let bracketed = lock(&self.shared.parser).screen().bracketed_paste();
        let text = without_markers(text);
        let mut bytes = Vec::with_capacity(text.len() + PASTE_START.len() + PASTE_END.len());
        if bracketed {
            bytes.extend_from_slice(PASTE_START.as_bytes());
        }
        bytes.extend_from_slice(text.as_bytes());
        if bracketed {
            bytes.extend_from_slice(PASTE_END.as_bytes());
        }
        self.send(&bytes)
    }

    /// When the program last wrote anything, or when the session started if it has written
    /// nothing yet. The reading thread marks the moment its bytes arrive, so this is the
    /// program's own pace and has nothing to do with drawing.
    ///
    /// An application waits on it before writing to a program on its own: a program in the middle
    /// of answering is a program whose input is not being read yet, and text sent then lands
    /// inside whatever it is doing.
    #[must_use]
    pub fn last_output(&self) -> Instant {
        lock(&self.shared.quiet).output
    }

    /// When keys were last written to the program by the person at the keyboard, or when the
    /// session started if none have been. [`write`](Self::write) and every key a
    /// [`Terminal`](super::Terminal) widget sends move it; [`paste`](Self::paste) does not,
    /// because that is the application writing.
    ///
    /// Together with [`last_output`](Self::last_output) it answers the only question worth asking
    /// before writing into a terminal somebody is using: has the program been quiet *and* has the
    /// person been quiet. Writing while the person is mid-sentence cuts the sentence in half.
    #[must_use]
    pub fn last_input(&self) -> Instant {
        lock(&self.shared.quiet).input
    }

    /// The person pasted into a [`Terminal`](super::Terminal) widget: the paste of
    /// [`paste`](Self::paste), counted as the person's input because a person made it.
    pub(crate) fn paste_typed(&self, text: &str) -> io::Result<()> {
        self.paste(text)?;
        lock(&self.shared.quiet).input = Instant::now();
        Ok(())
    }

    /// Writes to the program without saying who asked for it.
    fn send(&self, bytes: &[u8]) -> io::Result<()> {
        if self.exit().is_some() {
            // The pseudo-terminal still takes bytes after the program is gone and nobody ever
            // reads them, so the answer has to come from the recorded end instead.
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "the program has ended"));
        }
        let mut process = lock(&self.process);
        process.writer.write_all(bytes)?;
        process.writer.flush()
    }

    /// Ends the program.
    pub fn kill(&self) {
        let _ = lock(&self.process).killer.kill();
    }

    /// The program's process id, `None` when the system did not give one. The program leads
    /// its own process group, with the same id, which also holds what it started in the
    /// foreground.
    #[must_use]
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Ends the program politely: SIGHUP to its process group, as when a terminal window
    /// closes, then SIGKILL if it has not ended within `grace`. Returns at once; the wait runs
    /// on a background thread and the watch reports the end as usual. Keep the session while the
    /// grace runs: dropping the last handle ends the program at once. Does nothing once the
    /// program has ended. Where there are no process groups it is [`kill`](Self::kill).
    pub fn terminate(&self, grace: Duration) {
        if self.exit().is_some() {
            return;
        }
        let Some(group) = self.pid.and_then(process_group) else {
            self.kill();
            return;
        };
        signal_group(group, Hangup::Polite);
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || {
            let deadline = Instant::now() + grace;
            let mut signal = lock(&shared.signal);
            while signal.exited.is_none() {
                let now = Instant::now();
                if now >= deadline {
                    // Sent under the lock, right after seeing no end recorded: the reader records
                    // the end as soon as it has collected the process, so the id still names it.
                    signal_group(group, Hangup::Forced);
                    return;
                }
                signal = shared.changed.wait_timeout(signal, deadline - now).unwrap_or_else(PoisonError::into_inner).0;
            }
        });
    }

    /// The exit code once the program ended (`Some(None)` when it could not be read).
    #[must_use]
    pub fn exit(&self) -> Option<Option<u32>> {
        lock(&self.shared.signal).exited
    }

    /// Asks for a new screen size; applied by the running watch, never on the drawing thread.
    pub(crate) fn request_size(&self, rows: u16, cols: u16) {
        let mut signal = lock(&self.shared.signal);
        if signal.size != (rows, cols) && signal.wanted != Some((rows, cols)) {
            signal.wanted = Some((rows, cols));
            drop(signal);
            self.shared.changed.notify_all();
        }
    }

    /// The parsed screen, locked while the guard lives.
    pub(crate) fn parser(&self) -> MutexGuard<'_, vt100::Parser<Notices>> {
        lock(&self.shared.parser)
    }
}

/// The two signals of [`TerminalSession::terminate`].
#[derive(Debug, Clone, Copy)]
enum Hangup {
    /// SIGHUP: the terminal went away; most programs save and end.
    Polite,
    /// SIGKILL: ends the program whatever it does.
    Forced,
}

#[cfg(unix)]
type Group = rustix::process::Pid;

#[cfg(unix)]
fn process_group(pid: u32) -> Option<Group> {
    rustix::process::Pid::from_raw(i32::try_from(pid).ok()?)
}

#[cfg(unix)]
fn signal_group(group: Group, hangup: Hangup) {
    use rustix::process::Signal;
    let signal = match hangup {
        Hangup::Polite => Signal::HUP,
        Hangup::Forced => Signal::KILL,
    };
    // Fails only when the group is already gone, which is what was wanted.
    let _ = rustix::process::kill_process_group(group, signal);
}

#[cfg(not(unix))]
type Group = std::convert::Infallible;

#[cfg(not(unix))]
fn process_group(_: u32) -> Option<Group> {
    None
}

#[cfg(not(unix))]
fn signal_group(group: Group, _: Hangup) {
    match group {}
}

fn pty_size((rows, cols): (u16, u16)) -> PtySize {
    PtySize { rows: rows.max(2), cols: cols.max(2), pixel_width: 0, pixel_height: 0 }
}

/// Waits for changes of a [`TerminalSession`] on a background thread.
///
/// It holds no strong handle to the program, so dropping the session still ends it.
/// [`next_change_within`](Self::next_change_within) is the same wait with a bound, for tests that
/// must not hang on a program that says nothing.
pub struct TerminalWatch {
    shared: Arc<Shared>,
    process: Weak<Mutex<Process>>,
}

/// What [`TerminalWatch::wait`] found.
enum Found {
    Event(TerminalEvent),
    Notice(TerminalChange),
}

impl Found {
    fn into_change(self) -> TerminalChange {
        match self {
            Found::Event(TerminalEvent::Output) => TerminalChange::Output,
            Found::Event(TerminalEvent::Exited(code)) => TerminalChange::Exited(code),
            Found::Notice(change) => change,
        }
    }
}

impl TerminalWatch {
    /// Blocks until new output arrives or the program ends. Pending size changes are applied
    /// here. Call it inside [`Command::perform`](crate::runtime::Command::perform), never in
    /// `update` or `view`.
    ///
    /// Titles, folders, bells and notifications are not reported here; use
    /// [`next_change`](Self::next_change) for them.
    #[must_use]
    pub fn next(&self) -> TerminalEvent {
        loop {
            // Without a deadline the wait always finds something; a notice is not an event.
            if let Some(Found::Event(event)) = self.wait(false, None) {
                return event;
            }
        }
    }

    /// Blocks until something changes: output, the end of the program, or a title, folder,
    /// bell or notification from it (see [`TerminalChange`]). Pending size changes are applied
    /// here. Call it inside [`Command::perform`](crate::runtime::Command::perform), never in
    /// `update` or `view`, and start it again when its message arrives, until it reports
    /// [`TerminalChange::Exited`].
    ///
    /// Notices come before the output they arrived with, one per call; output follows the
    /// session's [`coalesce`](TerminalBuilder::coalesce) interval.
    #[must_use]
    pub fn next_change(&self) -> TerminalChange {
        loop {
            // Without a deadline the wait always finds something.
            if let Some(found) = self.wait(true, None) {
                return found.into_change();
            }
        }
    }

    /// Waits for a change as [`next_change`](Self::next_change) does, but gives up after
    /// `bound` and answers `None` when nothing was reported in that time.
    ///
    /// Made for tests: a screen test drives a
    /// [`Command::perform`](crate::runtime::Command::perform) on the spot, so watching a live but
    /// silent program with `next_change` would never come back. A test asks for a change within a
    /// generous bound instead, and `None` says the program had nothing to say. The session stays
    /// usable: ask again, or write to the program and ask again. A running application has a
    /// thread for its watch and keeps using `next_change`.
    ///
    /// Pending size changes are applied here as well. When the session
    /// [coalesces](TerminalBuilder::coalesce), output pending inside the interval is still held
    /// back, so a bound shorter than the interval can answer `None` although output has arrived.
    #[must_use]
    pub fn next_change_within(&self, bound: Duration) -> Option<TerminalChange> {
        // A bound so far off that the clock cannot name it is the unbounded wait.
        let deadline = Instant::now().checked_add(bound);
        Some(self.wait(true, deadline)?.into_change())
    }

    /// Waits for the next change, `None` once `deadline` has passed with nothing to report.
    fn wait(&self, notices: bool, deadline: Option<Instant>) -> Option<Found> {
        let mut signal = lock(&self.shared.signal);
        loop {
            if let Some(size) = signal.wanted.take() {
                drop(signal);
                self.resize(size);
                signal = lock(&self.shared.signal);
                signal.size = size;
                continue;
            }
            if notices && let Some(change) = signal.notices.pop() {
                return Some(Found::Notice(change));
            }
            if signal.generation != signal.seen {
                let now = Instant::now();
                if let Some(due) = signal.reported.map(|at| at + self.shared.coalesce)
                    && due > now
                {
                    signal = self.hold(signal, Some(due), deadline)?;
                    continue;
                }
                signal.seen = signal.generation;
                signal.reported = Some(now);
                return Some(Found::Event(TerminalEvent::Output));
            }
            if let Some(code) = signal.exited {
                return Some(Found::Event(TerminalEvent::Exited(code)));
            }
            if self.process.strong_count() == 0 {
                return Some(Found::Event(TerminalEvent::Exited(None)));
            }
            signal = self.hold(signal, None, deadline)?;
        }
    }

    /// Waits on the session's condition variable until something changes, until `until` when a
    /// coalescing interval is being waited out, and until `deadline` when the caller gave one,
    /// whichever comes first. `None` once the deadline has passed.
    fn hold<'a>(
        &self,
        signal: MutexGuard<'a, Signal>,
        until: Option<Instant>,
        deadline: Option<Instant>,
    ) -> Option<MutexGuard<'a, Signal>> {
        let wake = match (until, deadline) {
            (Some(until), Some(deadline)) => Some(until.min(deadline)),
            (until, deadline) => until.or(deadline),
        };
        let Some(wake) = wake else {
            return Some(self.shared.changed.wait(signal).unwrap_or_else(PoisonError::into_inner));
        };
        let now = Instant::now();
        if deadline.is_some_and(|deadline| now >= deadline) {
            return None;
        }
        let rest = wake.saturating_duration_since(now);
        Some(self.shared.changed.wait_timeout(signal, rest).unwrap_or_else(PoisonError::into_inner).0)
    }

    fn resize(&self, size: (u16, u16)) {
        if let Some(process) = self.process.upgrade() {
            let _ = lock(&process).master.resize(pty_size(size));
        }
        lock(&self.shared.parser).screen_mut().set_size(size.0.max(2), size.1.max(2));
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::*;

    /// Generous: the tests run next to many others on a busy machine.
    const PATIENCE: Duration = Duration::from_secs(20);

    fn sh(script: &str) -> TerminalBuilder {
        TerminalSession::builder("/bin/sh").args(["-c", script]).folder(Path::new("/"))
    }

    /// Changes until the program ends, the end included.
    fn changes_to_exit(session: &TerminalSession) -> Vec<TerminalChange> {
        let watch = session.watch();
        let started = Instant::now();
        let mut changes = Vec::new();
        loop {
            let change = watch.next_change();
            let end = matches!(change, TerminalChange::Exited(_));
            changes.push(change);
            if end {
                return changes;
            }
            assert!(started.elapsed() < PATIENCE, "{changes:?}");
        }
    }

    fn contents(session: &TerminalSession) -> String {
        session.parser().screen().contents()
    }

    #[test]
    fn environment_variables_reach_the_program() {
        let session = sh("printf '%s %s' \"$QDESK\" \"$TERM\"").env("QDESK", "1").spawn().expect("pty");
        changes_to_exit(&session);
        assert_eq!(contents(&session), "1 xterm-256color");
        let session = sh("printf '%s' \"$TERM\"").env("TERM", "vt100").spawn().expect("pty");
        changes_to_exit(&session);
        assert_eq!(contents(&session), "vt100", "a variable given replaces the default");
    }

    #[test]
    fn the_program_starts_on_the_given_size() {
        let session = sh("stty size").size(100, 30).spawn().expect("pty");
        changes_to_exit(&session);
        assert_eq!(contents(&session).trim(), "30 100");
        assert_eq!(session.parser().screen().size(), (30, 100));
    }

    #[test]
    fn scrollback_keeps_the_given_number_of_lines() {
        let script = "i=0; while [ $i -lt 100 ]; do echo line $i; i=$((i+1)); done";
        let session = sh(script).scrollback(10).spawn().expect("pty");
        changes_to_exit(&session);
        let mut parser = session.parser();
        parser.screen_mut().set_scrollback(usize::MAX);
        assert_eq!(parser.screen().scrollback(), 10);
        let session = sh(script).spawn().expect("pty");
        changes_to_exit(&session);
        let mut parser = session.parser();
        parser.screen_mut().set_scrollback(usize::MAX);
        assert_eq!(parser.screen().scrollback(), 100 - 23, "the default keeps everything here");
    }

    #[test]
    fn titles_folders_bells_and_notifications_are_reported() {
        let script = "printf '\\033]0;title\\007'; sleep 0.2; \\
                      printf '\\033]7;file://host/tmp/x%%20y\\033\\\\'; sleep 0.2; \\
                      printf '\\a'; sleep 0.2; \\
                      printf '\\033]9;hi\\007'; sleep 0.2; \\
                      printf '\\033]777;notify;T;B\\007'; sleep 0.2; \\
                      printf '\\033]2;hal'; sleep 0.2; printf 'f\\007'; exit 3";
        let session = sh(script).spawn().expect("pty");
        let changes = changes_to_exit(&session);
        let notices: Vec<&TerminalChange> =
            changes.iter().filter(|change| !matches!(change, TerminalChange::Output)).collect();
        assert_eq!(
            notices,
            [
                &TerminalChange::Title("title".into()),
                &TerminalChange::WorkingFolder("/tmp/x y".into()),
                &TerminalChange::Bell,
                &TerminalChange::Notify { title: None, body: "hi".into() },
                &TerminalChange::Notify { title: Some("T".into()), body: "B".into() },
                &TerminalChange::Title("half".into()),
                &TerminalChange::Exited(Some(3)),
            ],
            "{changes:?}"
        );
    }

    #[test]
    fn next_leaves_the_notices_out() {
        let session = sh("printf '\\033]0;title\\007\\a'; exit 4").spawn().expect("pty");
        let watch = session.watch();
        let started = Instant::now();
        loop {
            match watch.next() {
                TerminalEvent::Output => assert!(started.elapsed() < PATIENCE),
                TerminalEvent::Exited(code) => break assert_eq!(code, Some(4)),
            }
        }
    }

    #[test]
    fn coalescing_bounds_the_reports_of_a_fast_stream() {
        let interval = Duration::from_millis(50);
        let session = sh("yes | head -n 200000; sleep 0.3; printf end").coalesce(interval).spawn().expect("pty");
        let watch = session.watch();
        let started = Instant::now();
        let mut outputs = 0u32;
        loop {
            match watch.next_change() {
                TerminalChange::Output => outputs += 1,
                TerminalChange::Exited(_) => break,
                _ => {}
            }
            assert!(started.elapsed() < PATIENCE);
        }
        let elapsed = started.elapsed();
        let bound = u32::try_from(elapsed.as_millis() / interval.as_millis()).unwrap_or(u32::MAX) + 2;
        assert!(outputs <= bound, "{outputs} reports in {elapsed:?}");
        assert!(outputs >= 2, "the stream and the last word were both reported");
        assert!(contents(&session).ends_with("end"), "the last bytes were not held back");
    }

    #[test]
    fn terminate_hangs_up_and_kills_what_ignores_the_hangup() {
        let session = sh("echo ready; sleep 100").spawn().expect("pty");
        assert!(session.pid().is_some());
        wait_for(&session, "ready");
        let started = Instant::now();
        session.terminate(PATIENCE);
        assert!(started.elapsed() < Duration::from_secs(1), "terminate does not block");
        assert!(matches!(changes_to_exit(&session).last(), Some(TerminalChange::Exited(_))));
        assert!(started.elapsed() < PATIENCE, "the hangup ended it before the grace");

        let session = sh("trap '' HUP; echo ready; sleep 100").spawn().expect("pty");
        wait_for(&session, "ready");
        let grace = Duration::from_millis(300);
        let started = Instant::now();
        session.terminate(grace);
        assert!(started.elapsed() < grace, "terminate does not block");
        changes_to_exit(&session);
        assert!(started.elapsed() >= grace, "it outlived the hangup until the grace ran out");
        assert!(session.exit().is_some());
    }

    /// A bound short enough to keep the test quick, long enough not to matter.
    const BOUND: Duration = Duration::from_millis(300);

    /// How much longer than the bound a busy machine may take to come back.
    const SLACK: Duration = Duration::from_secs(5);

    #[test]
    fn a_bounded_wait_gives_up_on_a_live_but_silent_program_and_leaves_it_usable() {
        // Live and silent: it waits for a line of input and says nothing until it gets one.
        let session = sh("printf ready; read line; printf 'got %s' \"$line\"; exit 6").spawn().expect("pty");
        wait_for(&session, "ready");
        let watch = session.watch();
        // Three times over, so giving up is not a one-off and nothing is left behind.
        for round in 1..=3 {
            let started = Instant::now();
            let change = watch.next_change_within(BOUND);
            let elapsed = started.elapsed();
            assert_eq!(change, None, "round {round}");
            assert!(elapsed >= BOUND, "round {round} came back early: {elapsed:?}");
            assert!(elapsed < BOUND + SLACK, "round {round} overshot the bound: {elapsed:?}");
        }
        assert!(session.exit().is_none(), "the program is still running");

        session.write(b"now\n").expect("write");
        let started = Instant::now();
        let mut seen = Vec::new();
        loop {
            let change = watch.next_change_within(PATIENCE).expect("the program answered");
            let end = matches!(change, TerminalChange::Exited(_));
            seen.push(change);
            if end {
                break;
            }
            assert!(started.elapsed() < PATIENCE, "{seen:?}");
        }
        assert!(seen.contains(&TerminalChange::Output), "{seen:?}");
        assert_eq!(seen.last(), Some(&TerminalChange::Exited(Some(6))), "{seen:?}");
        assert!(contents(&session).contains("got now"), "{}", contents(&session));
    }

    #[test]
    fn a_bounded_wait_returns_output_well_inside_the_bound() {
        let session = sh("sleep 0.2; printf hello; sleep 100").spawn().expect("pty");
        let watch = session.watch();
        let started = Instant::now();
        assert_eq!(watch.next_change_within(PATIENCE), Some(TerminalChange::Output));
        assert!(started.elapsed() < PATIENCE / 4, "it waited out the bound: {:?}", started.elapsed());
        session.kill();
    }

    #[test]
    fn a_bounded_wait_still_reports_the_end_of_the_program() {
        let session = sh("printf bye; exit 9").spawn().expect("pty");
        let watch = session.watch();
        let started = Instant::now();
        loop {
            match watch.next_change_within(PATIENCE) {
                Some(TerminalChange::Exited(code)) => break assert_eq!(code, Some(9)),
                Some(_) => assert!(started.elapsed() < PATIENCE),
                None => panic!("the end was not reported within the bound"),
            }
        }
    }

    /// A program that shows what it is given as it arrives, with escape sequences visible: the
    /// terminal echoes nothing and hands over every byte instead of whole lines, so a paste shows
    /// up whether or not it ends with a line break. `ready` marks the setup as done.
    const SHOWS_INPUT: &str = "stty -echo -icanon min 1 time 0; printf ready; cat -v";

    /// The same, after asking for bracketed paste, so the marks are expected around a paste.
    const SHOWS_INPUT_BRACKETED: &str = "stty -echo -icanon min 1 time 0; printf '\\033[?2004hready'; cat -v";

    /// Waits until the screen holds `text`, without hanging on a program that never writes it.
    fn expect_screen(session: &TerminalSession, text: &str) {
        let watch = session.watch();
        let started = Instant::now();
        while !contents(session).contains(text) {
            assert!(started.elapsed() < PATIENCE, "{text:?} never arrived: {:?}", contents(session));
            let _ = watch.next_change_within(BOUND);
        }
    }

    /// Waits until the program has said nothing for a while, so a later "it stayed the same"
    /// reads a real silence and not a report that has not arrived yet.
    fn expect_silence(session: &TerminalSession) {
        let watch = session.watch();
        let started = Instant::now();
        while watch.next_change_within(BOUND).is_some() {
            assert!(started.elapsed() < PATIENCE, "the program never fell silent: {:?}", contents(session));
        }
    }

    #[test]
    fn a_paste_reaches_a_program_that_asked_for_the_marks_in_one_piece() {
        let session = sh(SHOWS_INPUT_BRACKETED).spawn().expect("pty");
        expect_screen(&session, "ready");
        assert!(session.parser().screen().bracketed_paste(), "the program turned the mode on");
        session.paste("first line\nsecond line").expect("paste");
        expect_screen(&session, "^[[201~");
        let shown = contents(&session);
        assert_eq!(shown.matches("^[[200~").count(), 1, "one paste, one start mark: {shown:?}");
        assert_eq!(shown.matches("^[[201~").count(), 1, "one paste, one end mark: {shown:?}");
        assert!(shown.contains("^[[200~first line"), "{shown:?}");
        assert!(shown.contains("second line^[[201~"), "{shown:?}");
        session.kill();
    }

    #[test]
    fn a_paste_goes_plain_to_a_program_that_did_not_ask_for_the_marks() {
        let session = sh(SHOWS_INPUT).spawn().expect("pty");
        expect_screen(&session, "ready");
        assert!(!session.parser().screen().bracketed_paste());
        session.paste("bare text").expect("paste");
        expect_screen(&session, "bare text");
        let shown = contents(&session);
        assert!(!shown.contains("^["), "no marks were sent: {shown:?}");
        session.kill();
    }

    #[test]
    fn the_paste_marks_inside_the_text_are_left_out() {
        let session = sh(SHOWS_INPUT_BRACKETED).spawn().expect("pty");
        expect_screen(&session, "ready");
        // An end mark would leave the paste early and have "b" read as keys; a start mark would
        // open a second paste inside the first.
        session.paste("a\x1b[201~b\x1b[200~c").expect("paste");
        expect_screen(&session, "^[[201~");
        let shown = contents(&session);
        assert!(shown.contains("^[[200~abc^[[201~"), "{shown:?}");
        assert_eq!(shown.matches("^[[201~").count(), 1, "{shown:?}");
        assert_eq!(shown.matches("^[[200~").count(), 1, "{shown:?}");
        session.kill();
    }

    #[test]
    fn last_output_follows_the_program_and_stands_still_while_it_is_quiet() {
        let before = Instant::now();
        let session = sh("printf ready; read line; printf 'got it'").spawn().expect("pty");
        assert!(session.last_output() >= before, "it starts at the session's start");
        expect_screen(&session, "ready");
        expect_silence(&session);
        let wrote = session.last_output();
        assert!(wrote > before, "the program's first words moved it");
        expect_silence(&session);
        assert_eq!(session.last_output(), wrote, "a program waiting for a line does not move it");
        session.write(b"now\n").expect("write");
        expect_screen(&session, "got it");
        assert!(session.last_output() > wrote, "the answer moved it");
    }

    #[test]
    fn last_input_hears_the_keys_and_not_the_applications_own_paste() {
        let before = Instant::now();
        let session = sh(SHOWS_INPUT).spawn().expect("pty");
        let start = session.last_input();
        assert!(start >= before, "it starts at the session's start");
        expect_screen(&session, "ready");
        session.paste("pasted").expect("paste");
        expect_screen(&session, "pasted");
        assert_eq!(session.last_input(), start, "the application pasting is not the person typing");
        session.write(b"typed").expect("write");
        expect_screen(&session, "typed");
        let typed = session.last_input();
        assert!(typed > start, "a key written to the program moved it");
        session.paste(" and more").expect("paste");
        expect_screen(&session, "and more");
        assert_eq!(session.last_input(), typed, "a paste after a key still does not count");
        session.kill();
    }

    #[test]
    fn a_paste_into_a_program_that_ended_is_an_error() {
        let session = sh("printf bye").spawn().expect("pty");
        changes_to_exit(&session);
        assert!(session.paste("too late").is_err(), "the program no longer reads its input");
        assert!(session.write(b"too late").is_err(), "and so is writing to it");
    }

    fn wait_for(session: &TerminalSession, text: &str) {
        let watch = session.watch();
        let started = Instant::now();
        while !contents(session).contains(text) {
            let _ = watch.next();
            assert!(started.elapsed() < PATIENCE, "{}", contents(session));
        }
    }
}
