//! One instance at a time: an advisory lock the operating system holds for the running process.
//!
//! A lock made of "the file exists" sets a trap. The counter is running, the power goes out, the
//! file stays on the disk, and the next start refuses to write — exactly when the recovery that
//! has to write is the only thing left to do. An advisory lock has no such state: the kernel
//! holds it for a process, and when the process dies, whatever kills it, the lock is gone.

use std::fs;
use std::io;
use std::path::Path;

/// An advisory lock on a file, held by the running process for as long as this value lives.
///
/// The lock is the operating system's, not the file's existence: the kernel releases it when the
/// process ends, so a crash or a power cut never leaves a lock behind. Dropping the value
/// releases it too.
///
/// The file itself holds the process id of the holder as text, and that is only ever material
/// for a message to the user: a process id is reused, so nothing may be decided from it. The
/// decision is [`AppLock::acquire`]'s answer and nothing else.
///
/// ```no_run
/// # use qframe::storage::{AppLock, data_dir};
/// let dir = data_dir("qfocus").expect("a home directory");
/// std::fs::create_dir_all(&dir)?;
/// match AppLock::acquire(&dir.join("lock"))? {
///     Some(_lock) => { /* this instance may write; the lock lives as long as `_lock` */ }
///     None => { /* another instance is running */ }
/// }
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct AppLock {
    /// Holding the open file is the lock; the kernel releases it when this file is closed,
    /// which is what dropping the lock does. Nothing ever reads the field, and that is the
    /// point: it exists to be held. Unix only, because only there does this framework have a
    /// lock to hold.
    #[cfg(unix)]
    #[expect(dead_code, reason = "the open file is the lock; closing it releases it")]
    file: fs::File,
}

impl AppLock {
    /// Takes the lock on `path`, creating the file when it is not there, or answers `None`
    /// because another process holds it.
    ///
    /// The parent directory has to exist. On success the process id of this process is written
    /// into the file, for a message that names the holder; see [`holder_pid`].
    ///
    /// On Unix systems this is `flock(LOCK_EX | LOCK_NB)`, which the kernel releases when the
    /// process dies. Locks of this kind are per open file, not per process, so a second
    /// `acquire` on the same path inside one process answers `None` as well.
    ///
    /// A child process started between the moment the lock is taken and the moment it is released
    /// keeps it for a few milliseconds longer: between `fork` and `exec` the child holds a copy of
    /// every open file, this file among them, and the kernel counts the lock as held until that copy
    /// is closed. So a released lock is free in a moment, not in the same instant, and a program
    /// that starts children may need to wait briefly before it can take its own lock again.
    ///
    /// On every other platform, Windows among them, this framework has no advisory lock yet:
    /// `acquire` returns an error of kind [`io::ErrorKind::Unsupported`] and never `Ok`, so an
    /// application is told it has no lock instead of quietly running without one. Windows would
    /// need `LockFileEx`, which is not reachable without `unsafe`, and this framework forbids
    /// `unsafe`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file cannot be opened or written, and
    /// [`io::ErrorKind::Unsupported`] on a platform without an advisory lock. A lock another
    /// process holds is `Ok(None)`, not an error.
    pub fn acquire(path: &Path) -> io::Result<Option<Self>> {
        acquire(path)
    }
}

/// The process id the lock file names, for a message such as "another instance (12345) is
/// running". `None` when the file is missing, empty or holds anything but a number.
///
/// This is diagnostic text and nothing more. The process id may belong to a process that died
/// long ago and to something else entirely by now, so no decision may rest on it; only
/// [`AppLock::acquire`] answers whether the lock is free.
#[must_use]
pub fn holder_pid(path: &Path) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Takes the lock with `flock`, which the kernel drops when the process dies.
#[cfg(unix)]
fn acquire(path: &Path) -> io::Result<Option<AppLock>> {
    use rustix::fs::{FlockOperation, flock};

    let file = fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => {}
        Err(errno) => {
            let error = io::Error::from(errno);
            // The one error that is an answer rather than a failure: somebody else has it.
            if error.kind() == io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(error);
        }
    }
    // The lock is ours from here on; the process id is written for diagnostics only. A failure to
    // write it is still reported, as `acquire` promises, and returning drops the file, which
    // releases the lock again: the caller never holds a lock it was told it did not get.
    let pid = format!("{}\n", std::process::id());
    file.set_len(0)?;
    io::Write::write_all(&mut &file, pid.as_bytes())?;
    Ok(Some(AppLock { file }))
}

/// Reports that this platform has no advisory lock in this framework.
#[cfg(not(unix))]
fn acquire(_path: &Path) -> io::Result<Option<AppLock>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "this platform has no advisory lock in this framework; see AppLock::acquire",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// An empty directory of this test's own.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-lock-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("test directory");
        dir
    }

    #[test]
    fn the_second_attempt_is_told_the_lock_is_taken_and_the_drop_frees_it() {
        if !cfg!(unix) {
            return;
        }
        let dir = temp_dir("busy");
        let path = dir.join("lock");

        let held = AppLock::acquire(&path).expect("first attempt").expect("the lock is free");
        assert_eq!(holder_pid(&path), Some(std::process::id()), "the file names the holder");
        // A `flock` belongs to an open file, not to a process, so this is the same answer
        // another process would get.
        assert!(AppLock::acquire(&path).expect("second attempt").is_none(), "the lock is taken");

        drop(held);
        // Free in a moment, not in the same instant: another test running beside this one starts
        // a child process, and between `fork` and `exec` that child holds a copy of this open
        // file, so the kernel counts the lock as held until the copy is closed. The deadline is
        // what is asserted, and it is generous on purpose; the usual answer is the first one.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut again = AppLock::acquire(&path).expect("third attempt");
        while again.is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
            again = AppLock::acquire(&path).expect("another attempt");
        }
        assert!(again.is_some(), "dropping the holder released the lock");
        drop(again);
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_lock_file_left_behind_is_not_a_lock() {
        if !cfg!(unix) {
            return;
        }
        let dir = temp_dir("stale");
        let path = dir.join("lock");
        // What a power cut leaves: the file, with a process id in it, and no lock.
        fs::write(&path, "4242\n").expect("write a stale file");
        assert_eq!(holder_pid(&path), Some(4242));
        let lock = AppLock::acquire(&path).expect("attempt").expect("a leftover file holds nothing");
        assert_eq!(holder_pid(&path), Some(std::process::id()), "the holder is now this process");
        drop(lock);
        fs::remove_dir_all(&dir).expect("clean");
    }

    /// A real second process, so the answer is not only this process talking to itself.
    /// `flock(1)` exits with the code given to `-E` when the lock is taken.
    #[test]
    fn another_process_is_kept_out_and_let_in_again() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let dir = temp_dir("process");
        let path = dir.join("lock");
        let attempt = |path: &Path| {
            std::process::Command::new("flock")
                .args(["--nonblock", "--conflict-exit-code", "9"])
                .arg(path)
                .args(["--command", "true"])
                .status()
        };
        let held = AppLock::acquire(&path).expect("attempt").expect("the lock is free");
        let Ok(busy) = attempt(&path) else {
            // No `flock` command on this machine; the in-process test covers the same call.
            fs::remove_dir_all(&dir).expect("clean");
            return;
        };
        assert_eq!(busy.code(), Some(9), "the other process was told the lock is taken");
        drop(held);
        // Free in a moment, not in the same instant: a child another test starts between `fork`
        // and `exec` holds a copy of the open file for that long (see `AppLock::acquire`).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut code = attempt(&path).expect("run").code();
        while code != Some(0) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
            code = attempt(&path).expect("run").code();
        }
        assert_eq!(code, Some(0), "with the holder gone the lock is free");
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_file_that_cannot_be_opened_is_an_error() {
        let dir = temp_dir("missing");
        let error = AppLock::acquire(&dir.join("absent").join("lock")).expect_err("no directory");
        let expected = if cfg!(unix) { io::ErrorKind::NotFound } else { io::ErrorKind::Unsupported };
        assert_eq!(error.kind(), expected);
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn a_file_without_a_number_names_nobody() {
        let dir = temp_dir("pid");
        let path = dir.join("lock");
        assert_eq!(holder_pid(&path), None, "a missing file names nobody");
        fs::write(&path, "").expect("empty");
        assert_eq!(holder_pid(&path), None);
        fs::write(&path, "qfocus\n").expect("text");
        assert_eq!(holder_pid(&path), None);
        fs::remove_dir_all(&dir).expect("clean");
    }
}
