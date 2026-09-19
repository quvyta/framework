//! Many instances at once, and something that waits for the last of them to close.
//!
//! [`AppLock`](super::AppLock) answers "is another instance running?". A background service asks
//! the other question: "tell me when none is". Every instance holds a shared lock on one file; the
//! service asks for the exclusive lock on the same file and the kernel puts it to sleep until the
//! last shared holder is gone. There is no polling and no count to keep right: a crashed instance
//! holds nothing, because the kernel releases a lock when the process dies.

#[cfg(unix)]
use std::fs;
use std::io;
use std::path::Path;

/// A shared or exclusive advisory lock on a file, held for as long as this value lives.
///
/// Every running instance of an application holds a [shared](Self::shared) lock; any number of
/// them can at once. Something that has to act once they are all gone, such as a service that
/// cleans up after the last window closes, [waits for the exclusive lock](Self::wait_exclusive)
/// on the same file, and wakes when the last shared lock is released, however that happens: a
/// normal exit, a crash or a `kill -9`.
///
/// ```no_run
/// # use qframe::storage::{InstanceLock, data_dir};
/// # let path = data_dir("qcode").expect("a home directory").join("instances");
/// // In every instance, for as long as it runs:
/// let _instance = InstanceLock::shared(&path)?;
///
/// // In the service, on a thread of its own:
/// let last_closed = InstanceLock::wait_exclusive(&path)?;
/// // ... clean up after the instances ...
/// drop(last_closed); // let a new instance start
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// These locks are `flock` locks, which belong to an open file rather than to a process: two
/// locks taken on one path inside one process meet each other exactly as two processes would. A
/// shared lock and an [`AppLock`](super::AppLock) are two different questions, so keep them on
/// two different files.
///
/// On every platform other than Unix this framework has no advisory lock, so each call returns an
/// error of kind [`io::ErrorKind::Unsupported`] and never `Ok`, as
/// [`AppLock::acquire`](super::AppLock::acquire) does.
#[derive(Debug)]
pub struct InstanceLock {
    /// Holding the open file is the lock; closing it, which dropping the value does, releases
    /// it. Nothing reads the field: it exists to be held. Unix only, as for `AppLock`.
    #[cfg(unix)]
    #[expect(dead_code, reason = "the open file is the lock; closing it releases it")]
    file: fs::File,
}

impl InstanceLock {
    /// Takes a shared lock on `path`, creating the file when it is not there. Any number of
    /// shared locks can be held at once, so this does not wait for other instances.
    ///
    /// It waits only while an exclusive lock is held: a service that is cleaning up after the
    /// last instance holds that lock until it is done, and an instance starting meanwhile waits
    /// for the cleanup instead of running into it. The parent directory has to exist.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file cannot be opened or locked, and
    /// [`io::ErrorKind::Unsupported`] on a platform without an advisory lock.
    pub fn shared(path: &Path) -> io::Result<Self> {
        shared(path)
    }

    /// Takes the exclusive lock on `path` when nobody holds a lock on it, or answers `None` at
    /// once because somebody does. The file is created when it is not there; the parent
    /// directory has to exist.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file cannot be opened or locked, and
    /// [`io::ErrorKind::Unsupported`] on a platform without an advisory lock. A lock somebody
    /// else holds is `Ok(None)`, not an error.
    pub fn try_exclusive(path: &Path) -> io::Result<Option<Self>> {
        try_exclusive(path)
    }

    /// Waits until nobody holds a lock on `path`, then takes the exclusive lock and returns it.
    /// The file is created when it is not there; the parent directory has to exist.
    ///
    /// The thread sleeps in the kernel while it waits and costs no processor time. It wakes when
    /// the last holder's lock is released, which the kernel also does when a holder's process
    /// dies. The wait cannot be cancelled, so give it a thread of its own, and hold the lock only
    /// as long as the work that needs it: every [`shared`](Self::shared) call waits meanwhile.
    ///
    /// When nobody holds the lock this returns at once. A service that should wait again for the
    /// next group of instances therefore has to learn in some other way that one has started;
    /// calling this again in a loop with nobody running would spin.
    ///
    /// A released lock is free in a moment rather than in the same instant: a child process
    /// started between `fork` and `exec` holds a copy of every open file for those few
    /// milliseconds (see [`AppLock::acquire`](super::AppLock::acquire)).
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file cannot be opened or locked, and
    /// [`io::ErrorKind::Unsupported`] on a platform without an advisory lock.
    pub fn wait_exclusive(path: &Path) -> io::Result<Self> {
        wait_exclusive(path)
    }
}

/// Opens `path`, creating it when needed, and locks it with `operation`.
#[cfg(unix)]
fn open_and_lock(path: &Path, operation: rustix::fs::FlockOperation) -> io::Result<InstanceLock> {
    // The standard library opens files close-on-exec, so a program this process starts never
    // keeps the lock past its own `exec`.
    let file = fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
    loop {
        match rustix::fs::flock(&file, operation) {
            Ok(()) => return Ok(InstanceLock { file }),
            // A signal handled on this thread interrupts a wait; the wait goes on.
            Err(rustix::io::Errno::INTR) => {}
            Err(errno) => return Err(errno.into()),
        }
    }
}

#[cfg(unix)]
fn shared(path: &Path) -> io::Result<InstanceLock> {
    open_and_lock(path, rustix::fs::FlockOperation::LockShared)
}

#[cfg(unix)]
fn try_exclusive(path: &Path) -> io::Result<Option<InstanceLock>> {
    match open_and_lock(path, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
        Ok(lock) => Ok(Some(lock)),
        // The one error that is an answer rather than a failure: somebody holds a lock.
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn wait_exclusive(path: &Path) -> io::Result<InstanceLock> {
    open_and_lock(path, rustix::fs::FlockOperation::LockExclusive)
}

/// Reports that this platform has no advisory lock in this framework.
#[cfg(not(unix))]
fn unsupported() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "this platform has no advisory lock in this framework; see InstanceLock")
}

#[cfg(not(unix))]
fn shared(_path: &Path) -> io::Result<InstanceLock> {
    Err(unsupported())
}

#[cfg(not(unix))]
fn try_exclusive(_path: &Path) -> io::Result<Option<InstanceLock>> {
    Err(unsupported())
}

#[cfg(not(unix))]
fn wait_exclusive(_path: &Path) -> io::Result<InstanceLock> {
    Err(unsupported())
}

#[cfg(test)]
#[path = "instance_lock_tests.rs"]
mod tests;
