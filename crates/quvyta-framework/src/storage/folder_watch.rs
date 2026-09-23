//! Folder changes from the operating system's own events, without polling.
//!
//! Rereading a folder on a timer costs work every tick and still shows a change late. The kernel
//! already knows the moment an entry is created, removed or renamed; asking it to say so costs
//! nothing while nothing happens. On Linux that is inotify, reached through `rustix` without
//! `unsafe`.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long a batch keeps gathering after its first event. A `git checkout` touches thousands of
/// files in a burst; answering each would redraw a tree thousands of times, while a tenth of a
/// second is still quicker than anyone notices.
const GATHER: Duration = Duration::from_millis(100);

/// What happened in a watched folder; see [`FolderChange`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FolderChangeKind {
    /// An entry appeared: created here, or moved in from a folder this watch does not see.
    Created,
    /// An entry disappeared: removed, or moved out to a folder this watch does not see.
    Removed,
    /// An entry was renamed inside the folder; [`FolderChange::name`] is its new name.
    Renamed {
        /// The name the entry had before.
        from: OsString,
    },
    /// An entry's content or attributes (permissions, times) changed.
    Modified,
    /// The watched folder itself was removed, moved away or unmounted. It is reported once and no
    /// longer watched; watch it again under its new path if it still matters.
    Gone,
    /// The system dropped events because they came faster than they were read. Anything may have
    /// changed in this folder, so read it again.
    Overflow,
}

/// One change in a watched folder, as [`FolderChanges::next`] reports it.
///
/// A change carries the name of the entry it concerns, so an application that shows a single
/// file can tell whether that file changed without rereading anything. Most applications need
/// less: they read [`folder`](Self::folder) again, once per batch, whatever the names are.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FolderChange {
    /// The watched folder, as it was given to [`FolderWatch::watch`].
    pub folder: PathBuf,
    /// The entry inside it that changed; `None` when the change concerns the folder as a whole
    /// ([`FolderChangeKind::Gone`] and [`FolderChangeKind::Overflow`]).
    pub name: Option<OsString>,
    /// What happened.
    pub kind: FolderChangeKind,
}

/// Watches folders for changes the operating system reports, and never polls.
///
/// The watch is not recursive: a folder's own entries are watched, not what happens deeper
/// down. That matches a file tree, which only has to know about the folders it shows open.
///
/// The watch is owned by the application, which adds and removes folders as the user opens and
/// closes them. The waiting happens elsewhere: [`changes`](Self::changes) gives a handle whose
/// [`FolderChanges::next`] blocks, sleeping in the kernel, until something changes. Run it in a
/// [`Command::perform`](crate::runtime::Command::perform) and start it again when its message
/// arrives. Dropping the watch wakes a waiting `next`, which then answers an empty list.
///
/// ```no_run
/// # use qframe::storage::FolderWatch;
/// let mut watch = FolderWatch::new()?;
/// watch.watch(std::path::Path::new("/home/me/project"))?;
/// let changes = watch.changes();
/// // Inside a `Command::perform`:
/// for change in changes.next() {
///     println!("{} changed: {:?}", change.folder.display(), change.kind);
/// }
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// Only Linux has this watch so far, through inotify. On every other platform
/// [`FolderWatch::new`] returns an error of kind [`io::ErrorKind::Unsupported`], and the
/// application keeps rereading at the moments it chooses (after its own changes, on return to a
/// screen, on a "refresh" key). macOS and Windows have their own event sources, but this framework
/// reaches none of them without `unsafe` or a large dependency.
#[derive(Debug)]
pub struct FolderWatch {
    source: Arc<platform::Source>,
}

/// The waiting side of a [`FolderWatch`]; see [`FolderChanges::next`].
///
/// Cheap to clone and free to move to another thread, so each
/// [`Command::perform`](crate::runtime::Command::perform) can take its own.
#[derive(Debug, Clone)]
pub struct FolderChanges {
    source: Arc<platform::Source>,
}

impl FolderWatch {
    /// A watch with no folders yet.
    ///
    /// # Errors
    ///
    /// Returns the system's error when no watch can be made (too many open watches for this user,
    /// for example), and [`io::ErrorKind::Unsupported`] on a platform other than Linux.
    pub fn new() -> io::Result<Self> {
        Ok(Self { source: Arc::new(platform::Source::new()?) })
    }

    /// Starts watching the entries of `folder`. Watching a folder that is already watched does
    /// nothing.
    ///
    /// # Errors
    ///
    /// Returns the error when `folder` is missing or is not a folder, and an error of kind
    /// [`io::ErrorKind::QuotaExceeded`] when the system's limit on watches
    /// (`fs.inotify.max_user_watches` on Linux) is reached. Either way the application can go on
    /// without live changes for that folder.
    pub fn watch(&mut self, folder: &Path) -> io::Result<()> {
        self.source.watch(folder)
    }

    /// Stops watching `folder`. A folder that is not watched, or that is [`FolderChangeKind::Gone`],
    /// is left alone.
    pub fn unwatch(&mut self, folder: &Path) {
        self.source.unwatch(folder);
    }

    /// The handle that waits for this watch's changes.
    #[must_use]
    pub fn changes(&self) -> FolderChanges {
        FolderChanges { source: Arc::clone(&self.source) }
    }
}

impl Drop for FolderWatch {
    fn drop(&mut self) {
        // A waiter must not sleep forever on a watch nobody can add to or read from any more.
        self.source.close();
    }
}

impl FolderChanges {
    /// Blocks until something changes in a watched folder, then returns everything that changed
    /// within the next tenth of a second, oldest first and each change once. A burst of
    /// thousands of changes therefore arrives as a few batches, not as thousands of answers.
    ///
    /// While nothing changes the thread sleeps in the kernel and costs no processor time. When the
    /// [`FolderWatch`] is dropped, a waiting `next` wakes and returns an empty list, and so does
    /// every later call: an empty answer means "stop waiting". Call it inside
    /// [`Command::perform`](crate::runtime::Command::perform), never in `update` or `view`.
    ///
    /// A rename inside one folder is one [`FolderChangeKind::Renamed`]. An entry moved from one
    /// watched folder to another is [`FolderChangeKind::Removed`] in the first and
    /// [`FolderChangeKind::Created`] in the second, as it is when only one side is watched.
    #[must_use]
    pub fn next(&self) -> Vec<FolderChange> {
        self.source.next(GATHER, None).unwrap_or_default()
    }

    /// Waits like [`next`](Self::next), but at most `bound` for the first change; `None` when
    /// nothing changed in that time. The watch goes on: ask again.
    ///
    /// Made for screen tests: a [`Harness`](crate::runtime::Harness) runs the work of a
    /// [`Command::perform`](crate::runtime::Command::perform) on the spot, so a wait with no end
    /// would hold the test for good. A running application has a thread for its watch and keeps
    /// using `next`. Once a change has come, the tenth of a second that gathers its batch is
    /// waited in full, so a batch is never cut short by the bound.
    #[must_use]
    pub fn next_within(&self, bound: Duration) -> Option<Vec<FolderChange>> {
        // A bound so far off that the clock cannot name it is the unbounded wait.
        match Instant::now().checked_add(bound) {
            Some(limit) => self.source.next(GATHER, Some(limit)),
            None => Some(self.next()),
        }
    }
}

/// Collects one batch of changes: renames paired, repeats dropped, order kept.
#[derive(Debug, Default)]
struct Batch {
    changes: Vec<FolderChange>,
    /// Where each half-seen rename stands in `changes`, by the cookie that pairs its two halves.
    moved_from: Vec<(u32, usize)>,
}

impl Batch {
    fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    fn push(&mut self, folder: &Path, name: Option<OsString>, kind: FolderChangeKind) {
        self.changes.push(FolderChange { folder: folder.to_path_buf(), name, kind });
    }

    /// The first half of a move: the entry left `folder`. It stays a removal unless the second
    /// half arrives in the same folder.
    fn moved_from(&mut self, cookie: u32, folder: &Path, name: OsString) {
        self.moved_from.push((cookie, self.changes.len()));
        self.push(folder, Some(name), FolderChangeKind::Removed);
    }

    /// The second half of a move: the entry arrived in `folder` under `name`.
    fn moved_to(&mut self, cookie: u32, folder: &Path, name: OsString) {
        let first = self.moved_from.iter().position(|(seen, _)| *seen == cookie);
        if let Some(position) = first {
            let (_, index) = self.moved_from.remove(position);
            let earlier = &mut self.changes[index];
            if earlier.folder == folder {
                let from = earlier.name.take().unwrap_or_default();
                earlier.name = Some(name);
                earlier.kind = FolderChangeKind::Renamed { from };
                return;
            }
        }
        self.push(folder, Some(name), FolderChangeKind::Created);
    }

    /// The batch in the order it happened, each change once. A repeated change keeps its last
    /// place, so "created, removed, created again" ends as the entry being there.
    fn finish(self) -> Vec<FolderChange> {
        let mut seen = std::collections::HashSet::new();
        let mut kept: Vec<FolderChange> =
            self.changes.into_iter().rev().filter(|change| seen.insert(change.clone())).collect();
        kept.reverse();
        kept
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::collections::HashMap;
    use std::ffi::{CStr, OsStr, OsString};
    use std::io;
    use std::mem::MaybeUninit;
    use std::os::fd::OwnedFd;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::time::{Duration, Instant};

    use rustix::event::{EventfdFlags, PollFd, PollFlags, Timespec, eventfd, poll};
    use rustix::fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags};
    use rustix::io::Errno;

    use super::{Batch, FolderChange, FolderChangeKind};

    /// The inotify instance, the folders it watches, and the bell that ends a wait.
    #[derive(Debug)]
    pub(super) struct Source {
        inotify: OwnedFd,
        /// Written once when the watch is dropped, and never read: staying readable is what
        /// wakes every waiter, the present one and any later one.
        bell: OwnedFd,
        closed: AtomicBool,
        folders: Mutex<Folders>,
        /// Held by the one thread reading at a time, so two waiters never split a batch.
        buffer: Mutex<Vec<MaybeUninit<u8>>>,
    }

    /// Watched folders by the descriptor inotify gave them, and the way back.
    #[derive(Debug, Default)]
    struct Folders {
        by_descriptor: HashMap<i32, PathBuf>,
        by_path: HashMap<PathBuf, i32>,
    }

    impl Folders {
        fn forget(&mut self, descriptor: i32) -> Option<PathBuf> {
            let path = self.by_descriptor.remove(&descriptor)?;
            self.by_path.remove(&path);
            Some(path)
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        // A panic on another thread leaves the map and the buffer usable; the watch goes on.
        mutex.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Room for a few hundred events per read; the kernel queues the rest.
    const BUFFER: usize = 64 * 1024;

    impl Source {
        pub(super) fn new() -> io::Result<Self> {
            // Close-on-exec, so a child process never inherits the watch; non-blocking, because
            // the waiting is done by `poll`, which can also hear the bell.
            let inotify = inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK)?;
            let bell = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)?;
            Ok(Self {
                inotify,
                bell,
                closed: AtomicBool::new(false),
                folders: Mutex::new(Folders::default()),
                buffer: Mutex::new(vec![MaybeUninit::uninit(); BUFFER]),
            })
        }

        pub(super) fn watch(&self, folder: &Path) -> io::Result<()> {
            let mut folders = lock(&self.folders);
            if folders.by_path.contains_key(folder) {
                return Ok(());
            }
            let flags = WatchFlags::CREATE
                | WatchFlags::DELETE
                | WatchFlags::MOVED_FROM
                | WatchFlags::MOVED_TO
                | WatchFlags::MODIFY
                | WatchFlags::ATTRIB
                | WatchFlags::DELETE_SELF
                | WatchFlags::MOVE_SELF
                | WatchFlags::ONLYDIR;
            let descriptor = inotify::add_watch(&self.inotify, folder, flags).map_err(|errno| {
                if errno == Errno::NOSPC {
                    // The kernel says "no space on device", which reads like a full disk.
                    io::Error::new(
                        io::ErrorKind::QuotaExceeded,
                        "the limit on folder watches (fs.inotify.max_user_watches) is reached",
                    )
                } else {
                    io::Error::from(errno)
                }
            })?;
            // Two paths to one folder share a descriptor; the later path is the one reported.
            if let Some(earlier) = folders.by_descriptor.insert(descriptor, folder.to_path_buf()) {
                folders.by_path.remove(&earlier);
            }
            folders.by_path.insert(folder.to_path_buf(), descriptor);
            Ok(())
        }

        pub(super) fn unwatch(&self, folder: &Path) {
            let mut folders = lock(&self.folders);
            if let Some(descriptor) = folders.by_path.get(folder).copied() {
                folders.forget(descriptor);
                // Events already queued for it are dropped when read: nobody knows the descriptor.
                let _ = inotify::remove_watch(&self.inotify, descriptor);
            }
        }

        pub(super) fn close(&self) {
            self.closed.store(true, Ordering::SeqCst);
            // The counter only grows, so the write cannot fail for want of room in practice; the
            // flag above answers every later call even if it did.
            let _ = rustix::io::write(&self.bell, &1u64.to_ne_bytes());
        }

        /// The next batch, `None` when `limit` passes before the first change of one.
        pub(super) fn next(&self, gather: Duration, limit: Option<Instant>) -> Option<Vec<FolderChange>> {
            let mut buffer = lock(&self.buffer);
            let mut batch = Batch::default();
            let mut deadline: Option<Instant> = None;
            loop {
                if self.closed.load(Ordering::SeqCst) {
                    return Some(Vec::new());
                }
                // Until a change arrives the wait runs to the caller's limit, if any; after it,
                // to the end of the gathering.
                let until = match deadline {
                    Some(deadline) => Some((deadline, true)),
                    None => limit.map(|limit| (limit, false)),
                };
                let timeout = match until {
                    None => None,
                    Some((until, gathering)) => {
                        let left = until.saturating_duration_since(Instant::now());
                        if left.is_zero() {
                            return gathering.then(|| batch.finish());
                        }
                        Some(Timespec::try_from(left).unwrap_or(Timespec { tv_sec: 0, tv_nsec: 0 }))
                    }
                };
                let mut fds = [PollFd::new(&self.inotify, PollFlags::IN), PollFd::new(&self.bell, PollFlags::IN)];
                match poll(&mut fds, timeout.as_ref()) {
                    Ok(_) | Err(Errno::INTR) => {}
                    // Polling two descriptors this watch owns does not fail; if it ever does,
                    // the waiter stops rather than spinning, as if the watch had been dropped.
                    Err(_) => return Some(Vec::new()),
                }
                if !fds[1].revents().is_empty() {
                    return Some(Vec::new());
                }
                if !fds[0].revents().is_empty() {
                    self.drain(&mut buffer, &mut batch);
                    if deadline.is_none() && !batch.is_empty() {
                        deadline = Some(Instant::now() + gather);
                    }
                }
            }
        }

        /// Reads every event queued so far into `batch`.
        fn drain(&self, buffer: &mut [MaybeUninit<u8>], batch: &mut Batch) {
            let mut reader = inotify::Reader::new(&self.inotify, buffer);
            loop {
                match reader.next() {
                    Ok(event) => self.record(&event, batch),
                    Err(Errno::INTR) => {}
                    // `AGAIN` is the queue running empty; any other error leaves the rest for the
                    // next wake-up rather than losing the batch gathered so far.
                    Err(_) => return,
                }
            }
        }

        fn record(&self, event: &inotify::Event<'_>, batch: &mut Batch) {
            let flags = event.events();
            let mut folders = lock(&self.folders);
            if flags.contains(ReadFlags::QUEUE_OVERFLOW) {
                let mut all: Vec<&PathBuf> = folders.by_path.keys().collect();
                all.sort();
                for folder in all {
                    batch.push(folder, None, FolderChangeKind::Overflow);
                }
                return;
            }
            let Some(folder) = folders.by_descriptor.get(&event.wd()).cloned() else {
                // Unwatched a moment ago, or never known: nobody asked about it.
                return;
            };
            if flags.intersects(ReadFlags::DELETE_SELF | ReadFlags::MOVE_SELF | ReadFlags::UNMOUNT) {
                // Forget it now, so it is reported once. A moved folder would otherwise go on
                // reporting under its old path; a removed one is already dropped by the kernel,
                // which then answers this call with an error nobody needs to hear.
                folders.forget(event.wd());
                let _ = inotify::remove_watch(&self.inotify, event.wd());
                batch.push(&folder, None, FolderChangeKind::Gone);
                return;
            }
            let Some(name) = event.file_name().map(os_name) else {
                return;
            };
            if flags.contains(ReadFlags::MOVED_FROM) {
                batch.moved_from(event.cookie(), &folder, name);
            } else if flags.contains(ReadFlags::MOVED_TO) {
                batch.moved_to(event.cookie(), &folder, name);
            } else if flags.contains(ReadFlags::CREATE) {
                batch.push(&folder, Some(name), FolderChangeKind::Created);
            } else if flags.contains(ReadFlags::DELETE) {
                batch.push(&folder, Some(name), FolderChangeKind::Removed);
            } else if flags.intersects(ReadFlags::MODIFY | ReadFlags::ATTRIB) {
                batch.push(&folder, Some(name), FolderChangeKind::Modified);
            }
        }
    }

    fn os_name(name: &CStr) -> OsString {
        OsStr::from_bytes(name.to_bytes()).to_os_string()
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use std::io;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::FolderChange;

    /// No event source on this platform, so no value of this type is ever made.
    #[derive(Debug)]
    pub(super) enum Source {}

    impl Source {
        pub(super) fn new() -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "this platform has no folder watch in this framework; see FolderWatch",
            ))
        }

        pub(super) fn watch(&self, _folder: &Path) -> io::Result<()> {
            match *self {}
        }

        pub(super) fn unwatch(&self, _folder: &Path) {
            match *self {}
        }

        pub(super) fn close(&self) {
            match *self {}
        }

        pub(super) fn next(&self, _gather: Duration, _limit: Option<Instant>) -> Option<Vec<FolderChange>> {
            match *self {}
        }
    }
}

#[cfg(test)]
#[path = "folder_watch_tests.rs"]
mod tests;
