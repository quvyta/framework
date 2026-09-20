//! A program that keeps running after a detached handoff and talks to the application through
//! its standard input and output.
//!
//! One thread reads the program's output from the moment it starts: the handoff waits on it for
//! the first line, and once the application has the screen back the same thread hands every
//! later line to the application's messages. Reading never stops in between, so no line written
//! right after the first one is lost, and a program that writes faster than the application reads
//! waits on its full pipe instead of piling lines up in memory.

use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use super::process::{CHUNK, Lines};

/// What a [`LiveChild`] says, delivered through
/// [`DetachedHandoff::on_line`](super::DetachedHandoff::on_line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildLine {
    /// One line the program wrote on its standard output after its first one, without the
    /// newline. Text that is not UTF-8 is replaced rather than dropped.
    Line(String),
    /// The program closed its standard output and ended. Nothing follows.
    Ended {
        /// The exit code, or `None` when a signal ended it.
        code: Option<i32>,
    },
}

/// Where the lines of a live child go once the application has the screen back.
pub(crate) type Sink = Box<dyn FnMut(ChildLine) + Send>;

/// A program a [`DetachedHandoff`](super::DetachedHandoff) left running: its standard input is
/// the application's to write, its later output arrives as messages through
/// [`DetachedHandoff::on_line`](super::DetachedHandoff::on_line).
///
/// Clones share the one program, so the application can keep one in its state and move others
/// into background work. When the last clone is dropped — at the latest when the application's
/// state is dropped as the run ends — the program's standard input is closed and it reads the
/// end of its input; ending it before that is the application's decision
/// ([`LiveChild::close_stdin`], [`LiveChild::kill`]).
///
/// After its first line the program runs in the background of the terminal, which the
/// application draws on again. Its standard error is still the terminal, so it should keep quiet
/// there from then on: anything it writes lands on the application's screen until the next full
/// redraw.
#[derive(Clone)]
pub struct LiveChild {
    inner: Arc<Inner>,
}

enum Inner {
    Real(Real),
    Double(Arc<Double>),
}

/// A real program and the ends of its pipes the application holds.
struct Real {
    id: u32,
    stdin: Mutex<Option<ChildStdin>>,
    process: Arc<Mutex<Child>>,
    /// Hands the reading thread where to send the lines; taken by the first attach.
    attach: Mutex<Option<SyncSender<Attach>>>,
}

/// What the reading thread needs once the handoff is over.
pub(crate) struct Attach {
    sink: Sink,
    process: Arc<Mutex<Child>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl LiveChild {
    /// The program `process`, whose output the thread behind `attach` reads.
    pub(crate) fn running(process: Child, stdin: ChildStdin, attach: SyncSender<Attach>) -> Self {
        Self {
            inner: Arc::new(Inner::Real(Real {
                id: process.id(),
                stdin: Mutex::new(Some(stdin)),
                process: Arc::new(Mutex::new(process)),
                attach: Mutex::new(Some(attach)),
            })),
        }
    }

    /// A stand-in for tests, with the handle a test drives it through: what the application
    /// writes is recorded there, and the test says the program's lines and ends it. Answer a
    /// detached handoff with it through
    /// [`Harness::set_detached_outcome`](super::Harness::set_detached_outcome).
    ///
    /// ```
    /// use qframe::runtime::{ChildLine, LiveChild};
    ///
    /// let (child, program) = LiveChild::for_tests();
    /// child.write_line("status")?;
    /// assert_eq!(program.written(), ["status"]);
    /// assert_eq!(child.try_wait()?, None, "it runs until the test ends it");
    /// program.exit(Some(0));
    /// assert_eq!(child.try_wait()?, Some(Some(0)));
    /// # let _ = ChildLine::Line(String::new());
    /// # Ok::<(), std::io::Error>(())
    /// ```
    #[must_use]
    pub fn for_tests() -> (Self, TestChild) {
        let double = Arc::new(Double::default());
        (Self { inner: Arc::new(Inner::Double(Arc::clone(&double))) }, TestChild { double })
    }

    /// The program's process id, or `None` for the stand-in of [`LiveChild::for_tests`].
    #[must_use]
    pub fn id(&self) -> Option<u32> {
        match &*self.inner {
            Inner::Real(real) => Some(real.id),
            Inner::Double(_) => None,
        }
    }

    /// Writes `line` and a newline to the program's standard input. A `line` holding newlines
    /// reaches the program as several lines.
    ///
    /// The write blocks while the pipe is full, which only happens when the program stops
    /// reading; a program that answers each line keeps it empty.
    ///
    /// # Errors
    ///
    /// Returns an error once the standard input is closed ([`io::ErrorKind::BrokenPipe`]) or
    /// the program no longer reads it.
    pub fn write_line(&self, line: &str) -> io::Result<()> {
        match &*self.inner {
            Inner::Real(real) => {
                let mut stdin = lock(&real.stdin);
                let pipe = stdin.as_mut().ok_or_else(closed)?;
                let mut bytes = Vec::with_capacity(line.len() + 1);
                bytes.extend_from_slice(line.as_bytes());
                bytes.push(b'\n');
                pipe.write_all(&bytes)?;
                pipe.flush()
            }
            Inner::Double(double) => {
                let mut state = lock(&double.state);
                if !state.stdin_open || state.code.is_some() {
                    return Err(closed());
                }
                state.written.push(line.to_owned());
                Ok(())
            }
        }
    }

    /// Closes the program's standard input, for every clone: the program reads the end of its
    /// input, which is how a helper that serves one line at a time is asked to finish. Closing
    /// it again does nothing.
    pub fn close_stdin(&self) {
        match &*self.inner {
            Inner::Real(real) => drop(lock(&real.stdin).take()),
            Inner::Double(double) => lock(&double.state).stdin_open = false,
        }
    }

    /// Ends the program at once (`SIGKILL` on Unix); its [`ChildLine::Ended`] follows. Killing
    /// one that already ended does nothing.
    ///
    /// # Errors
    ///
    /// Returns the system's error, such as when the program runs as another user and may not
    /// be signalled, which is the case for one started through `pkexec` or `sudo`: close its
    /// standard input instead.
    pub fn kill(&self) -> io::Result<()> {
        match &*self.inner {
            Inner::Real(real) => lock(&real.process).kill(),
            Inner::Double(double) => {
                let mut state = lock(&double.state);
                state.killed = true;
                state.end(None);
                Ok(())
            }
        }
    }

    /// How the program ended, without waiting: `None` while it runs, then `Some(code)`, where
    /// `code` is `None` when a signal ended it, as in
    /// [`HandoffOutcome::Finished`](super::HandoffOutcome::Finished).
    ///
    /// # Errors
    ///
    /// Returns the system's error when the program's state cannot be read.
    pub fn try_wait(&self) -> io::Result<Option<Option<i32>>> {
        match &*self.inner {
            Inner::Real(real) => Ok(lock(&real.process).try_wait()?.map(|status| status.code())),
            Inner::Double(double) => Ok(lock(&double.state).code),
        }
    }

    /// Sends the program's lines to `sink` from now on. Only the first call counts: the lines
    /// have one reader.
    pub(crate) fn attach(&self, sink: Sink) {
        match &*self.inner {
            Inner::Real(real) => {
                if let Some(attach) = lock(&real.attach).take() {
                    // The reading thread takes this once and then ends with the program. A send
                    // that fails means the program is already over, so there are no more lines
                    // for the sink to be given.
                    let _ = attach.send(Attach { sink, process: Arc::clone(&real.process) });
                }
            }
            Inner::Double(double) => {
                let mut state = lock(&double.state);
                if state.sink.is_none() {
                    let mut sink = sink;
                    for line in state.waiting.drain(..) {
                        sink(line);
                    }
                    state.sink = Some(sink);
                }
            }
        }
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "the program's standard input is closed")
}

impl Drop for Inner {
    fn drop(&mut self) {
        // A real program's input closes as its pipe is dropped with it; the stand-in records it.
        if let Self::Double(double) = self {
            lock(&double.state).stdin_open = false;
        }
    }
}

impl fmt::Debug for LiveChild {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.inner {
            Inner::Real(real) => f.debug_struct("LiveChild").field("id", &real.id).finish_non_exhaustive(),
            Inner::Double(_) => f.write_str("LiveChild(test)"),
        }
    }
}

/// Two handles are equal when they are clones of each other: the same program.
impl PartialEq for LiveChild {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for LiveChild {}

/// The test's side of the stand-in from [`LiveChild::for_tests`]: it plays the program.
///
/// Lines it says reach the application through
/// [`DetachedHandoff::on_line`](super::DetachedHandoff::on_line) at the harness's next step, such
/// as [`Harness::render`](super::Harness::render); lines said before the handoff was answered
/// wait for it, as they would in a pipe.
#[derive(Clone)]
pub struct TestChild {
    double: Arc<Double>,
}

#[derive(Default)]
struct Double {
    state: Mutex<DoubleState>,
}

struct DoubleState {
    written: Vec<String>,
    stdin_open: bool,
    killed: bool,
    code: Option<Option<i32>>,
    sink: Option<Sink>,
    /// What was said before a sink was attached, oldest first.
    waiting: Vec<ChildLine>,
}

impl Default for DoubleState {
    fn default() -> Self {
        Self { written: Vec::new(), stdin_open: true, killed: false, code: None, sink: None, waiting: Vec::new() }
    }
}

impl DoubleState {
    fn say(&mut self, line: ChildLine) {
        match &mut self.sink {
            Some(sink) => sink(line),
            None => self.waiting.push(line),
        }
    }

    fn end(&mut self, code: Option<i32>) {
        if self.code.is_none() {
            self.code = Some(code);
            self.say(ChildLine::Ended { code });
        }
    }
}

impl TestChild {
    /// The program writes `line` on its standard output. Nothing is said after it ended.
    pub fn say(&self, line: impl Into<String>) {
        let mut state = lock(&self.double.state);
        if state.code.is_none() {
            state.say(ChildLine::Line(line.into()));
        }
    }

    /// The program ends with `code` (`None` for a signal): [`LiveChild::try_wait`] reports it
    /// and [`ChildLine::Ended`] is delivered. Only the first end counts.
    pub fn exit(&self, code: Option<i32>) {
        lock(&self.double.state).end(code);
    }

    /// Every line the application wrote with [`LiveChild::write_line`], oldest first.
    #[must_use]
    pub fn written(&self) -> Vec<String> {
        lock(&self.double.state).written.clone()
    }

    /// Whether the program's standard input is still open: `false` after
    /// [`LiveChild::close_stdin`] and once every [`LiveChild`] clone was dropped.
    #[must_use]
    pub fn stdin_open(&self) -> bool {
        lock(&self.double.state).stdin_open
    }

    /// Whether the application called [`LiveChild::kill`].
    #[must_use]
    pub fn killed(&self) -> bool {
        lock(&self.double.state).killed
    }
}

impl fmt::Debug for TestChild {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TestChild")
    }
}

/// The longest pause between two looks at a program that closed its output but has not ended.
const LONGEST_LOOK: Duration = Duration::from_millis(500);

/// Reads the program's output on the calling thread: the first line (or `None` at the end of
/// the output) goes to `first`, every later line to the sink `attach` delivers, and the end of
/// the program after them.
///
/// When nobody attaches — the handoff ended without detaching, or the application dropped the
/// child before it had the screen back — the rest of the output is read and dropped, so the
/// program is never stopped by a full pipe or ended by a closed one on the way out.
pub(crate) fn read(mut stdout: ChildStdout, first: &SyncSender<Option<String>>, attach: &Receiver<Attach>) {
    let mut lines = Lines::default();
    let mut ready = VecDeque::new();
    let mut chunk = [0_u8; CHUNK];
    let mut open = true;
    while open && ready.is_empty() {
        open = read_some(&mut stdout, &mut chunk, &mut lines, &mut ready);
    }
    let first_line = ready.pop_front();
    let said = first_line.is_some();
    if first.send(first_line).is_err() || !said {
        drain(open, &mut stdout, &mut chunk);
        return;
    }
    let Ok(Attach { mut sink, process }) = attach.recv() else {
        drain(open, &mut stdout, &mut chunk);
        return;
    };
    loop {
        for line in ready.drain(..) {
            sink(ChildLine::Line(line));
        }
        if !open {
            break;
        }
        open = read_some(&mut stdout, &mut chunk, &mut lines, &mut ready);
    }
    drop(stdout);
    sink(ChildLine::Ended { code: wait_for_end(&process) });
}

/// Reads once, adding finished lines to `ready`. Returns whether the output is still open.
fn read_some(stdout: &mut ChildStdout, chunk: &mut [u8], lines: &mut Lines, ready: &mut VecDeque<String>) -> bool {
    loop {
        match stdout.read(chunk) {
            Ok(0) => break,
            Ok(count) => {
                lines.feed(&chunk[..count], &mut |line| ready.push_back(line));
                return true;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    lines.finish(&mut |line| ready.push_back(line));
    false
}

fn drain(open: bool, stdout: &mut ChildStdout, chunk: &mut [u8]) {
    if open {
        while !matches!(stdout.read(chunk), Ok(0) | Err(_)) {}
    }
}

/// Waits for a program whose output ended to end too, and returns its exit code. It usually
/// ends right away; one that only closed its output is looked at less and less often.
fn wait_for_end(process: &Mutex<Child>) -> Option<i32> {
    let mut pause = Duration::from_millis(5);
    loop {
        match lock(process).try_wait() {
            Ok(Some(status)) => return status.code(),
            Ok(None) => {}
            Err(_) => return None,
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(LONGEST_LOOK);
    }
}
