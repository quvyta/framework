## Methods

- `FolderWatch::new() -> io::Result<FolderWatch>` — a watch with no folders; `Unsupported` on platforms other than Linux.
- `watch.watch(&Path) -> io::Result<()>` — watches the entries of one folder, not recursively; watching it again does nothing. `NotFound` or `NotADirectory` for a bad path, `QuotaExceeded` when the system's limit on watches is reached.
- `watch.unwatch(&Path)` — stops watching a folder; a folder that is not watched is left alone.
- `watch.changes() -> FolderChanges` — the waiting side; cheap to clone, `Send`, meant for `Command::perform`.
- `changes.next() -> Vec<FolderChange>` — blocks until something changes, gathers for a tenth of a second and returns the batch; empty once the watch is dropped.
- `changes.next_within(bound) -> Option<Vec<FolderChange>>` — the same wait, at most `bound` for the first change; `None` when nothing changed, and the watch goes on. For screen tests, where a harness runs the wait on the spot.
- `FolderChange { folder: PathBuf, name: Option<OsString>, kind: FolderChangeKind }` — `folder` as given to `watch`; `name` is `None` for `Gone` and `Overflow`.
- `FolderChangeKind::Created`, `Removed`, `Renamed { from: OsString }`, `Modified`, `Gone`, `Overflow`.

## Behaviour

- Linux uses inotify with close-on-exec, so a child process never inherits the watch.
- A batch keeps the order the changes happened in; a change repeated within it keeps its last place, so "created, removed, created" ends with the entry there.
- `Modified` covers content and attributes (permissions, times).
- A rename is paired within one batch; its two halves arrive together from the kernel.
- A move between two watched folders is `Removed` in the first and `Created` in the second; a move to or from an unwatched folder is the one half that is seen.
- `Overflow` comes once for every watched folder, sorted by path.
- `Gone` comes once, and the folder is no longer watched: a new folder at the same path is not followed until it is watched again.
- Events still queued for a folder that was just unwatched are dropped.
- Two threads waiting on clones of one `FolderChanges` take turns; a batch is never split between them.
