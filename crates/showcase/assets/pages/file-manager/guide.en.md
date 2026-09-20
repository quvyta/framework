## When to use

Use a file manager when a folder is part of what your application is about: a project's files beside its editor, a Files window on a desktop, a pane that lets somebody make, rename, move and delete things without leaving your program. For choosing one path and going away again, use the file and folder picker instead — a picker answers a question, a manager is a place to work.

- **One state per manager on screen.** `FileManagerState` lives in your own state, like any other screen state, and you hand it every `FileManagerMsg`.
- **Opening is yours.** The manager says "this path was asked to be opened" and nothing more. A tab, a window, a preview or an answer to a dialog are all your decision.
- **Say what the folder is for.** `confined()` for a folder somebody must not leave; without it the manager shows whatever root it is given, the whole file system included.
- **Choose the shape.** `view(FileView::Tree | List | Icons)`: folders inside folders, one folder as rows with size, date and permissions, or one folder as a grid of icons. The tree is what you get without asking.

## Step by step

1. Keep the state: `FileManagerState::new(root)`, with `.confined()` when operations must not leave the root.
2. Read the root when the manager comes on screen: `self.manager.load(Msg::Files)`, from `init` or when your screen is entered. It reads the root the first time and every open folder again after that.
3. Hand every message over: `Msg::Files(m) => self.manager.update(m, Msg::Files)`. Reads and file operations run on background threads; drawing never waits for them.
4. Draw it: `FileManager::new(&self.manager, Msg::Files).on_open(|path| Msg::Open(path.to_path_buf())).show(ui).fill()`.
5. Add what only you know: `on_open_terminal` for "Open a terminal here", `menu_items(|key, targets| …)` for your own items on a row's menu, and `row_mark(|key| …)` for what an entry means to you.
6. Say where a deleted entry goes: `.trashing()` for the person's own trash, or nothing for deleting for good. `.showing_hidden(true)` for a manager that shows the dotfiles.
7. To follow other programs, `.following(true)` on the state, or `set_following(true)` while the manager is on screen and `false` when it leaves.

## How it works

- **Reading never happens while drawing.** A folder is read once, in one answer, and the view is built from what is already known. Ten thousand entries arrive as one batch and are drawn once; they are never streamed in pieces, because every redraw is a whole screen over a remote connection.
- **A flat view shows one folder.** The list and the icons show the folder `state.folder()` names, with its own row at the top: that row carries the folder's menu and is the way back out of it, and a folder row steps into it. The keys, the menus and every operation are the same in all three shapes.
- **The list reads a page, never a folder.** Size, date and permissions each mean one more call to the system for that one entry, so the list asks for a page of two hundred entries around the cursor and remembers what came back. The tree and the icons ask for nothing at all. An application that knows exactly which rows it draws asks for those with `state.detail(keys, wrap)`.
- **Only the visible rows are painted.** A folder of ten thousand entries costs what a folder of two hundred costs: the rows on screen, and no more. A tree row reads no size, date or permissions at all.
- **The reading indicator has a rule.** A read shorter than about 300 ms shows nothing; a slower one puts a small spinner on the folder's own row, which then stays about 500 ms. Quick reads never flash, slow ones never blink.
- **Keys, not paths.** Every entry has a key: its path relative to the root, written with `/` on every platform. Operations take keys and turn them into paths themselves, so a key that climbs out of the root (`..`, an empty part, a NUL) is refused wherever it came from — a session file included.
- **A confined manager refuses a link.** A folder on the way that is a symbolic link is refused, because a link can point anywhere. The link itself is still an entry: renaming, moving or deleting one acts on the link and leaves what it points at alone.
- **A mark is how you say what the manager cannot know.** `row_mark(|key| RowMark::new().sign("warning", "warning").faint(true))` gives one row a sign in a tone and a faint name: an entry your backup leaves out, one a version control system ignores, one that has not been saved. The sign and the tone come together on purpose, so the row is still told apart where colours are off. A mark never makes a row louder than the manager's own states: a cut entry and a disabled manager stay faint.
- **A trash is a rename, so it has a limit.** An entry goes to the trash by being renamed into it, which only works on the file system the trash is on. When it cannot, the manager does not delete it quietly and does not pretend it worked: it asks whether to delete it for good, in the danger colour, with the name and the reason. That question is the only place a trashing manager ever deletes anything for good.
- **A copy is a task, not a wait.** It runs on the framework's task machinery: the manager shows a row with what it is doing, how far it has come and a Stop button, and the rows stay readable underneath. Stopping takes the half-written entry away again — half a file is worse than no file — and what was already copied stays. Read `work()` to put the same operation in your own `Tasks` list.
- **Copying is not moving.** `Copy` puts entries aside the way `Cut` does, and `Paste` copies them; what was copied stays where it is, so nothing about it is faint, and a link is copied as a link rather than followed.
- **Hidden entries are read, not fetched.** They come with every read; showing them is a decision about the rows, not another look at the disk.
- **What was refused is said.** One entry gives its reason; several give a line each, and what could be done was done. A cut entry that could not be pasted stays cut, to be tried somewhere else.
- **Outside changes are merged.** With `following(true)` the manager watches exactly the folders on screen and rereads the one a change happened in. A change of content alone reads nothing: rows are names. A watch that overflows, or a folder that goes away, rereads everything on screen.

## Pitfalls

- **Do not reread on a timer.** Refresh when your own operation finished, when the manager comes back on screen, and when a watch says something changed.
- **Do not decide what a file is.** Extensions, viewers and programs are your application's business; the manager only hands over the path.
- **Do not tell somebody an entry is in the trash when it is not.** Hand `Trash` over and let the manager's question do the rest; it knows whether the rename worked.
- **Do not keep a key that may be gone.** After a change the manager forgets what a folder no longer has, so a key you kept yourself can name nothing. Ask the state.
- **Following costs a waiting thread.** Turn it off for a manager nobody is looking at, and remember it is a Linux watch: elsewhere your own rereads are what keeps the rows honest.
