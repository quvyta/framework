## When to use

Use a folder watch when something on screen shows a folder that other programs change: a file tree while a build writes into it, a list of recordings another process saves, a settings folder the user edits by hand. The operating system already knows the moment an entry appears or disappears; a watch asks it to say so, and costs nothing while nothing happens.

- **Watch** the folders the user sees open, not a whole tree. The watch is not recursive on purpose: a tree only has to know about what it shows.
- **Do not poll.** Reading a folder every second costs work on every tick and still shows the change late.
- **Keep your own rereads.** After your application changes a folder itself, reread it at once; the watch confirms it a moment later, which does no harm.

## Step by step

1. Make one watch at start and keep it in your state: `let watch = FolderWatch::new()?;`. On a platform without one it is an `Unsupported` error: keep rereading at the moments you choose then.
2. Watch each folder as it opens: `watch.watch(&folder)?`, and `watch.unwatch(&folder)` when it closes. An error means that folder has no live changes; the rest still do.
3. Wait in the background: `Command::perform(move || Msg::Changed(changes.next()))`, with `changes = watch.changes()`.
4. In `update`, reread each folder named in the batch, then wait again with a fresh `watch.changes()`.
5. An empty batch means the watch was dropped: stop waiting.

## How it works

- **The thread sleeps in the kernel.** `next()` blocks on the system's event queue and wakes only when something changes or the watch is dropped. An idle watch uses no processor time.
- **Changes arrive in batches.** After the first event `next()` keeps gathering for a tenth of a second, then returns everything at once, oldest first and each change once. A `git checkout` of thousands of files is a handful of batches, so your tree is rebuilt a handful of times.
- **A change names its entry.** `FolderChange { folder, name, kind }`: `Created`, `Removed`, `Modified`, or `Renamed { from }` for a rename inside one folder. A move from one watched folder to another is a removal in one and a creation in the other.
- **Overflow means "read again".** When changes come faster than they are read, the system drops some and says so. Every watched folder then gets one `Overflow`, and rereading them is the whole answer.
- **A folder that goes away is reported once.** Removed, moved elsewhere or unmounted, a watched folder is `Gone` once and no longer watched. Watch it again under its new path if it still matters.
- **Limits are errors, not crashes.** When the system's limit on watches is reached (`fs.inotify.max_user_watches` on Linux), `watch` returns a `QuotaExceeded` error.
- **Dropping the watch wakes the waiter.** The thread in `next()` returns an empty batch, and every later call does too, so a closed screen leaves no thread behind.
- **Linux only, for now.** macOS and Windows have event sources as well, but none this framework reaches without `unsafe` or a large dependency; `new()` says `Unsupported` there.

On this page, start the watch and press the buttons. A created file, a rename and a removal each arrive as a batch of one. "Make and remove" creates two hundred files and removes them again: four hundred changes arrive as one or two batches.

## Common mistakes

- **Waiting in `update`.** `next()` blocks; call it only inside `Command::perform`.
- **Forgetting to wait again.** Each `perform` returns one batch. Start the next wait when its message arrives, or the second change is never heard.
- **Watching every folder of a tree.** Each watch counts against the user's limit. Watch what is open and unwatch what closes.
- **Updating the tree from names alone.** A batch can hold an entry created and removed again. Reread the folder; the names are there for the rare screen that follows a single file.
- **Treating `Gone` as an error.** The user moved or removed the folder; show that, and stop showing its contents.
- **Relying on the watch for your own changes.** Reread right after your own write, so the screen does not wait a tenth of a second for news it already has.
