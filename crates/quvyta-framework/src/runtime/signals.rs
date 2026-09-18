//! Catching the signals that end a run, on Unix: `SIGTERM`, `SIGINT` and `SIGHUP`.
//!
//! A signal handler can do almost nothing safely, so `signal-hook` turns each signal into a byte
//! on a pipe, and one thread, `quvyta-signals`, reads them in order. For each one it applies the
//! same rule the engine follows ([`termination::receive`]), records the cause for the terminal
//! loop and wakes it: the loop waits on the keyboard and on a socket this thread writes to, so a
//! signal is heard at once, not at the next key or the next deadline.
//!
//! The thread also keeps the promise that a run ends in bounded time. The loop quits by itself
//! when the application's grace is over, but the loop may be the thing that is stuck: an
//! `update` that never returns, a handed-off program that ignores the signal. So a second after
//! the grace, or a second after a second `SIGTERM`, the thread ends the process itself: it kills
//! a handed-off program, restores the terminal if it still exists, and lets the signal do what
//! it does without the framework.
//!
//! While a [`Handoff`](super::Handoff) program owns the terminal, a caught signal is passed on to
//! the program's process group, which is the terminal's foreground: it ends the way it would
//! have ended as a job of the shell, and the application takes the terminal back afterwards.
//!
//! Handlers are installed on the first run and stay. Outside a run a signal ends the process the
//! way it would without them, except a `SIGHUP` after a run whose terminal hung up: that is the
//! same hangup arriving late.

use std::io::{self, Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError, mpsc};
use std::time::{Duration, Instant};

use crossterm::event as ct;
use nix::sys::signal::{SigSet, Signal as NixSignal};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{Mode, OFlags};
use rustix::process::{Signal, getpgrp, kill_process_group};
use rustix::termios::{OptionalActions, Termios, isatty, tcgetattr, tcgetpgrp, tcsetattr};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM, SIGWINCH};
use signal_hook::iterator::Signals as Caught;
use signal_hook::low_level::emulate_default_handler;

use super::termination::{self, Ending, Step, Termination};

/// How long the loop has to end the run after the grace or a second signal before the process
/// is ended without it.
const FORCE_MARGIN: Duration = Duration::from_secs(1);

/// The run being watched, if any. One terminal runtime runs at a time.
static RUN: Mutex<Option<Run>> = Mutex::new(None);

/// Tells runs apart, so a late timer of one run never ends the next.
static SERIAL: AtomicU64 = AtomicU64::new(0);

/// Set once a run's terminal hung up. There is no terminal to hang up again, so a later SIGHUP
/// is that hangup arriving late.
static HUNG_UP: AtomicBool = AtomicBool::new(false);

/// What the signal thread knows about the run.
struct Run {
    serial: u64,
    epoch: Instant,
    ending: Option<Ending>,
    /// The signal the process ends by if it has to be ended by force.
    signal: i32,
    /// Causes the loop has not taken yet, oldest first.
    heard: Vec<Termination>,
    /// Whether the terminal was resized since the loop last looked.
    resized: bool,
    /// Written to wake the loop.
    wake: UnixStream,
    /// The terminal, for its foreground group and its modes.
    tty: OwnedFd,
    /// The terminal's modes before the run, restored when the process is ended by force.
    original: Option<Termios>,
    /// While a handoff runs: whether the application owned the terminal's foreground when it
    /// began, so that another foreground group is the program's.
    handoff: Option<bool>,
    /// When the process is ended if the run has not ended by then.
    force_at: Option<Instant>,
    /// Whether the terminal is standard input, which is what crossterm reads.
    reads_stdin: bool,
    /// Our end of the socket put in place of standard input once the terminal hung up; see
    /// [`release_input`]. Dropped with the run, so reads there then see the end of input.
    stand_in: Option<UnixStream>,
}

/// What the loop takes from the signal thread.
#[derive(Debug, Default)]
pub(crate) struct Heard {
    /// Causes to tell the engine, oldest first.
    pub(crate) causes: Vec<Termination>,
    /// Whether the terminal was resized.
    pub(crate) resized: bool,
}

/// Why a wait ended.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Woken {
    /// The keyboard has something to read.
    pub(crate) keyboard: bool,
    /// The terminal hung up.
    pub(crate) hung_up: bool,
}

/// The terminal loop's side of the signals: its run is watched while this lives.
pub(crate) struct Signals {
    wake: UnixStream,
    tty: OwnedFd,
}

impl Signals {
    /// Starts watching the run: installs the handlers the first time, and records the
    /// terminal's modes before the runtime changes them.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when there is no terminal, when the handlers cannot be installed or
    /// when another terminal runtime is already running.
    pub(crate) fn catch() -> io::Result<Self> {
        start_catching()?;
        let reads_stdin = isatty(io::stdin());
        let tty = terminal()?;
        let (wake, wake_write) = UnixStream::pair()?;
        wake.set_nonblocking(true)?;
        wake_write.set_nonblocking(true)?;
        let run = Run {
            serial: SERIAL.fetch_add(1, Ordering::Relaxed),
            epoch: Instant::now(),
            ending: None,
            signal: SIGTERM,
            heard: Vec::new(),
            resized: false,
            wake: wake_write,
            tty: tty.try_clone()?,
            original: tcgetattr(&tty).ok(),
            handoff: None,
            force_at: None,
            reads_stdin,
            stand_in: None,
        };
        let mut slot = lock();
        if slot.is_some() {
            return Err(io::Error::other("a terminal runtime is already running"));
        }
        *slot = Some(run);
        Ok(Self { wake, tty })
    }

    /// The causes heard since the last call, and whether the terminal was resized.
    pub(crate) fn take(&self) -> Heard {
        let mut slot = lock();
        let Some(run) = slot.as_mut() else {
            return Heard::default();
        };
        Heard { causes: std::mem::take(&mut run.heard), resized: std::mem::take(&mut run.resized) }
    }

    /// Whether a cause waits for the loop.
    pub(crate) fn pending(&self) -> bool {
        lock().as_ref().is_some_and(|run| !run.heard.is_empty())
    }

    /// Reports a hangup the loop found itself, in the terminal it reads or writes, as if its
    /// `SIGHUP` had arrived: the signal may come later, or not at all when the application is
    /// not the one the system tells.
    pub(crate) fn hung_up(&self) {
        if let Some(run) = lock().as_mut() {
            note(run, Termination::Hangup, SIGHUP);
            release_input(run);
        }
    }

    /// Whether the terminal is gone: after a hangup it answers nothing, not even its modes.
    pub(crate) fn terminal_gone(&self) -> bool {
        tcgetattr(&self.tty).is_err()
    }

    /// Whether the terminal has hung up, asked without waiting. The loop asks before every
    /// question to crossterm: on a terminal that hung up every read finds nothing, and
    /// crossterm's reader keeps reading forever.
    pub(crate) fn hung_up_now(&self) -> bool {
        let mut fds = [PollFd::new(&self.tty, PollFlags::empty())];
        let now = Timespec { tv_sec: 0, tv_nsec: 0 };
        poll(&mut fds, Some(&now)).is_ok() && fds[0].revents().contains(PollFlags::HUP)
    }

    /// Marks a handoff as running or over, so the signals that arrive meanwhile reach its
    /// program.
    pub(crate) fn handoff(&self, running: bool) {
        let owned = running && tcgetpgrp(&self.tty).is_ok_and(|group| group == getpgrp());
        if let Some(run) = lock().as_mut() {
            run.handoff = running.then_some(owned);
        }
    }

    /// Waits up to `timeout` for a signal and, with `keyboard`, for the terminal to have input
    /// or to hang up.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when waiting fails.
    pub(crate) fn wait(&self, timeout: Duration, keyboard: bool) -> io::Result<Woken> {
        let limit = Timespec::try_from(timeout).map_err(|_| io::Error::other("wait too long"))?;
        let mut fds = [PollFd::new(&self.wake, PollFlags::IN), PollFd::new(&self.tty, PollFlags::IN)];
        let count = if keyboard { 2 } else { 1 };
        match poll(&mut fds[..count], Some(&limit)) {
            // A signal arrived on this thread; the loop looks at what it was.
            Ok(_) | Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
        let mut drained = [0_u8; 64];
        while matches!((&self.wake).read(&mut drained), Ok(1..)) {}
        if !keyboard {
            return Ok(Woken::default());
        }
        let terminal = fds[1].revents();
        if terminal.contains(PollFlags::NVAL) {
            // Some systems cannot poll a terminal device. Waiting the way crossterm does still
            // hears the keyboard; a signal is then heard within the wait, at most half a second.
            return Ok(Woken { keyboard: ct::poll(timeout)?, hung_up: false });
        }
        Ok(Woken {
            keyboard: terminal.intersects(PollFlags::IN | PollFlags::ERR),
            hung_up: terminal.contains(PollFlags::HUP),
        })
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        *lock() = None;
    }
}

fn lock() -> MutexGuard<'static, Option<Run>> {
    RUN.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The terminal the loop reads: standard input when it is one, as crossterm reads it, and the
/// controlling terminal otherwise.
fn terminal() -> io::Result<OwnedFd> {
    let stdin = io::stdin();
    if isatty(&stdin) {
        return stdin.as_fd().try_clone_to_owned();
    }
    Ok(rustix::fs::open("/dev/tty", OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC, Mode::empty())?)
}

/// Installs the handlers and starts the thread that reads them, once per process.
///
/// The handlers are installed on that thread, after it started: were they installed first and
/// the thread failed to start, every signal would be caught and nobody would read it.
fn start_catching() -> io::Result<()> {
    static STARTED: OnceLock<Result<(), String>> = OnceLock::new();
    STARTED
        .get_or_init(|| {
            let (started, result) = mpsc::channel();
            std::thread::Builder::new()
                .name("quvyta-signals".to_owned())
                .spawn(move || match Caught::new([SIGTERM, SIGINT, SIGHUP, SIGWINCH]) {
                    Ok(mut caught) => {
                        let _ = started.send(Ok(()));
                        for signal in caught.forever() {
                            hear(signal);
                        }
                    }
                    Err(error) => {
                        let _ = started.send(Err(error.to_string()));
                    }
                })
                .map_err(|error| error.to_string())?;
            result.recv().map_err(|error| error.to_string())?
        })
        .clone()
        .map_err(io::Error::other)
}

/// Handles one caught signal.
fn hear(signal: i32) {
    let mut slot = lock();
    let Some(run) = slot.as_mut() else {
        drop(slot);
        // No run to end gracefully: the signal does what it would without the framework. A
        // SIGHUP after a run whose terminal hung up is that same hangup arriving late, often
        // after the loop saw it first and the application saved and quit; it must not end the
        // work the application does after `run` returns.
        let echo = signal == SIGHUP && HUNG_UP.load(Ordering::Relaxed);
        if signal != SIGWINCH && !echo {
            let _ = emulate_default_handler(signal);
        }
        return;
    };
    if signal == SIGWINCH {
        run.resized = true;
    } else {
        let cause = if signal == SIGHUP { Termination::Hangup } else { Termination::Terminate };
        if note(run, cause, signal) != Step::Ignore {
            forward(run, signal);
        }
        if signal == SIGHUP && tcgetattr(&run.tty).is_err() {
            release_input(run);
        }
    }
    let _ = (&run.wake).write(&[0]);
}

/// Frees a loop caught reading a terminal that hung up.
///
/// The loop never asks crossterm once it sees the hangup, but the hangup can come in the moment
/// after it looked. Crossterm's reader then reads until the terminal says it has nothing more
/// for now, and a hung-up terminal never says that: every read finds nothing, forever. So the
/// terminal is replaced, on standard input where crossterm reads it, by one end of an empty,
/// non-blocking socket, whose reads say "nothing for now" and let the reader return. The
/// terminal is gone, so nothing is lost; when the run ends our end closes and standard input
/// reads as ended, as the hung-up terminal did.
fn release_input(run: &mut Run) {
    HUNG_UP.store(true, Ordering::Relaxed);
    if !run.reads_stdin || run.stand_in.is_some() {
        return;
    }
    let Ok((ours, theirs)) = UnixStream::pair() else {
        return;
    };
    if theirs.set_nonblocking(true).is_ok() && rustix::stdio::dup2_stdin(&theirs).is_ok() {
        run.stand_in = Some(ours);
    }
}

/// Applies the rule to one more `cause`, records it for the loop and sets when the process is
/// ended if the loop does not end the run first.
fn note(run: &mut Run, cause: Termination, signal: i32) -> Step {
    let step = termination::receive(&mut run.ending, cause, run.epoch.elapsed());
    let force_at = match step {
        Step::Ignore => return step,
        Step::Ask(_) => run.epoch + run.ending.map_or(Duration::ZERO, |ending| ending.deadline) + FORCE_MARGIN,
        Step::End => Instant::now() + FORCE_MARGIN,
    };
    run.signal = signal;
    run.heard.push(cause);
    if run.force_at.is_none_or(|at| force_at < at) {
        run.force_at = Some(force_at);
        schedule(run.serial, force_at);
    }
    step
}

/// Ends the process at `at` unless run `serial` has ended by then.
fn schedule(serial: u64, at: Instant) {
    // Without a thread the loop's own deadline still ends the run, unless the loop is stuck.
    let _ = std::thread::Builder::new().name("quvyta-ending".to_owned()).spawn(move || {
        std::thread::sleep(at.saturating_duration_since(Instant::now()));
        let slot = lock();
        if let Some(run) = slot.as_ref()
            && run.serial == serial
            && run.force_at.is_some_and(|due| due <= Instant::now())
        {
            force(run);
        }
    });
}

/// Passes `signal` on to a handed-off program, which owns the terminal's foreground and is
/// otherwise never told: the signal was sent to the application alone.
fn forward(run: &Run, signal: i32) {
    if run.handoff != Some(true) {
        return;
    }
    if let (Ok(group), Some(signal)) = (tcgetpgrp(&run.tty), Signal::from_named_raw(signal))
        && group != getpgrp()
    {
        let _ = kill_process_group(group, signal);
    }
}

/// Ends the process now: the handed-off program first, then the terminal is put back as it
/// was, then the signal does what it would without the framework.
fn force(run: &Run) -> ! {
    if run.handoff == Some(true)
        && let Ok(group) = tcgetpgrp(&run.tty)
        && group != getpgrp()
    {
        let _ = kill_process_group(group, Signal::KILL);
    }
    restore(run);
    let _ = emulate_default_handler(run.signal);
    std::process::exit(128 + run.signal)
}

/// Leaves application mode on the terminal, when it still exists.
///
/// The screen is written through a description of its own that never blocks: a terminal that
/// stopped reading must not keep the process alive. The application may be in the background
/// of its terminal, during a handoff, and the system answers a background group that changes the
/// terminal with SIGTTOU, which would stop it; blocking the signal on this thread lets the modes
/// be put back, as a shell does. The thread ends with the process, so its mask is not restored.
fn restore(run: &Run) {
    let Some(original) = run.original.as_ref() else {
        return;
    };
    if tcgetattr(&run.tty).is_err() {
        return;
    }
    let mut ttou = SigSet::empty();
    ttou.add(NixSignal::SIGTTOU);
    let _ = ttou.thread_block();
    let raw_off = || tcsetattr(&run.tty, OptionalActions::Now, original).map_err(io::Error::from);
    match rustix::fs::open(
        "/dev/tty",
        OFlags::WRONLY | OFlags::NOCTTY | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    ) {
        Ok(screen) => {
            let _ = super::terminal::give_back(&mut std::fs::File::from(screen), true, raw_off);
        }
        Err(_) => {
            let _ = raw_off();
        }
    }
}
