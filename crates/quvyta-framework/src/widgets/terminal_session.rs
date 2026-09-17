//! A process running in a pseudo-terminal, its output parsed into a screen of cells.

use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};

/// Lines kept above the screen for scrolling back.
const SCROLLBACK: usize = 5000;

/// Something that changed in a [`TerminalSession`], see [`TerminalWatch::next`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalEvent {
    /// New output was parsed; draw again.
    Output,
    /// The process ended with this exit code, or the session was dropped (`None`).
    Exited(Option<u32>),
}

/// What the reader thread, the watch and the widget share.
struct Shared {
    parser: Mutex<vt100::Parser>,
    signal: Mutex<Signal>,
    changed: Condvar,
}

#[derive(Default)]
struct Signal {
    /// Bumped for every chunk of output.
    generation: u64,
    /// The generation the watch reported last.
    seen: u64,
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

/// A program (usually a shell) running in a pseudo-terminal.
///
/// Output is read on a background thread and parsed into a screen with scrollback; a
/// [`Terminal`](super::Terminal) widget draws that screen and sends keys to the program.
/// Handles are cheap to clone; when the last one is dropped the program is ended.
///
/// The runtime learns about new output through a [`TerminalWatch`]: run
/// [`TerminalWatch::next`] in a [`Command::perform`](crate::runtime::Command::perform), and
/// start it again when its message arrives, until it reports [`TerminalEvent::Exited`].
#[derive(Clone)]
pub struct TerminalSession {
    shared: Arc<Shared>,
    process: Arc<Mutex<Process>>,
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
    ///
    /// # Errors
    ///
    /// Fails when no pseudo-terminal can be opened or the program cannot start.
    pub fn spawn(program: &OsStr, args: &[impl AsRef<OsStr>], folder: &Path) -> io::Result<Self> {
        let size = (24, 80);
        let pair = native_pty_system().openpty(pty_size(size)).map_err(io::Error::other)?;
        let mut command = CommandBuilder::new(program);
        command.args(args);
        command.cwd(folder);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        let mut child = pair.slave.spawn_command(command).map_err(io::Error::other)?;
        // Without the slave end here, the reader sees end-of-file once the child exits.
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let writer = pair.master.take_writer().map_err(io::Error::other)?;
        let killer = child.clone_killer();

        let shared = Arc::new(Shared {
            parser: Mutex::new(vt100::Parser::new(size.0, size.1, SCROLLBACK)),
            signal: Mutex::new(Signal { size, ..Signal::default() }),
            changed: Condvar::new(),
        });
        let thread_shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        lock(&thread_shared.parser).process(&buffer[..count]);
                        lock(&thread_shared.signal).generation += 1;
                        thread_shared.changed.notify_all();
                    }
                }
            }
            let code = child.wait().ok().map(|status| status.exit_code());
            lock(&thread_shared.signal).exited = Some(code);
            thread_shared.changed.notify_all();
        });
        Ok(Self { shared, process: Arc::new(Mutex::new(Process { master: pair.master, writer, killer })) })
    }

    /// A watch that reports output and the end of the program; see [`TerminalWatch::next`].
    #[must_use]
    pub fn watch(&self) -> TerminalWatch {
        TerminalWatch { shared: Arc::clone(&self.shared), process: Arc::downgrade(&self.process) }
    }

    /// Sends bytes to the program as if typed.
    ///
    /// # Errors
    ///
    /// Fails when the program no longer reads its input.
    pub fn write(&self, bytes: &[u8]) -> io::Result<()> {
        let mut process = lock(&self.process);
        process.writer.write_all(bytes)?;
        process.writer.flush()
    }

    /// Ends the program.
    pub fn kill(&self) {
        let _ = lock(&self.process).killer.kill();
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
    pub(crate) fn parser(&self) -> MutexGuard<'_, vt100::Parser> {
        lock(&self.shared.parser)
    }
}

fn pty_size((rows, cols): (u16, u16)) -> PtySize {
    PtySize { rows: rows.max(2), cols: cols.max(2), pixel_width: 0, pixel_height: 0 }
}

/// Waits for changes of a [`TerminalSession`] on a background thread.
///
/// It holds no strong handle to the program, so dropping the session still ends it.
pub struct TerminalWatch {
    shared: Arc<Shared>,
    process: Weak<Mutex<Process>>,
}

impl TerminalWatch {
    /// Blocks until new output arrives or the program ends. Pending size changes are applied
    /// here. Call it inside [`Command::perform`](crate::runtime::Command::perform), never in
    /// `update` or `view`.
    #[must_use]
    pub fn next(&self) -> TerminalEvent {
        let mut signal = lock(&self.shared.signal);
        loop {
            if let Some(size) = signal.wanted.take() {
                drop(signal);
                self.resize(size);
                signal = lock(&self.shared.signal);
                signal.size = size;
                continue;
            }
            if signal.generation != signal.seen {
                signal.seen = signal.generation;
                return TerminalEvent::Output;
            }
            if let Some(code) = signal.exited {
                return TerminalEvent::Exited(code);
            }
            if self.process.strong_count() == 0 {
                return TerminalEvent::Exited(None);
            }
            signal = self.shared.changed.wait(signal).unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn resize(&self, size: (u16, u16)) {
        if let Some(process) = self.process.upgrade() {
            let _ = lock(&process).master.resize(pty_size(size));
        }
        lock(&self.shared.parser).screen_mut().set_size(size.0.max(2), size.1.max(2));
    }
}
