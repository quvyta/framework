## Methods

- `FileManager::new(&state, wrap)` — `wrap` turns a `FileManagerMsg` into your message: a function such as `Msg::Files`, or a closure that captures what it needs. Any `Fn(FileManagerMsg) -> Msg + 'static`.
- `.root_label(text)` — what the top row says; the root folder's own name by default.
- `.on_open(|path| Msg)` — a click or Enter on a file. Without it a click on a file only selects it.
- `.on_open_terminal(|path| Msg)` — adds "Open a terminal here" to a folder's menu, with that folder's path.
- `.menu_items(|key, targets| Vec<ContextItem<Msg>>)` — your own items, in a group of their own; `targets` is what an action on that row acts on.
- `.row_mark(|key| RowMark)` — what the application says about a row's look; `RowMark::new()` for a row with nothing to say.
- `.view(FileView::Tree | List | Icons)` — the shape the folder is drawn in; the tree by default.
- `.kind_icons(bool)` — each row's icon by the kind of its entry, in the row's own colour; off by default. A mark's sign wins over it.
- `.kind_tones(bool)` — those icons in their family's colour; nothing without `kind_icons`, and not drawn in sixteen colours or in ASCII.
- `.user_folders(&UserFolders)` — the home whose folders `kind_icons` knows by name, in place of the person's own.
- `.disabled(bool)` — every row faint, no click, key, drag or menu.
- `.show(ui)` — adds the manager and returns the tree's node, for `.fill()` and `.id(name)`. The naming dialog is added too while one is open; it is a layer and takes no room.
- `FileManagerState::new(root)`, `.confined()`, `.following(bool)`, `.trashing()`, `.trashing_in(folder)`, `.showing_hidden(bool)`; `set_following(bool)`, `set_showing_hidden(bool)`, `is_confined()`, `follows_changes()`, `is_trashing()`, `shows_hidden()`.
- `.load(wrap)` — reads the root the first time and every open folder again after that. `.update(message, wrap)` — applies a message and answers with the work it asks for. `wrap` is moved to background threads: any `Fn(FileManagerMsg) -> Msg + Send + Sync + 'static`.
- Reading the state: `root()`, `path(key)`, `children(key)`, `work()`, `shown_children(key)`, `copied()`, `pending()`, `is_copying()`, `is_open(key)`, `is_loading(key)`, `is_folder(key)`, `folder_keys()`, `visible_folders()`, `selected()`, `chosen()`, `targets(key)`, `cut()`, `is_cut(key)`, `error()`, `naming()`, `naming_problem()`, `select(key)`.
- `FileManagerState::ROOT` — the root's key, the empty string.
- Details: `details(key) -> Option<Option<&FileDetails>>` (nothing asked for / asked for and nothing there), `has_details(key)`, `detail(keys, wrap)` for an exact range, `detail_page(folder, wrap)` for a page around the cursor, `detail_gaps(folder)` for what a page still lacks. `FileDetails { size, modified, mode, readonly }` with `size_text(folder)`, `modified_text()`, `permissions_text(folder)` and `FileDetails::read(path)`.
- Flat views: `folder()` — the folder the list and the icons show; `FileManagerMsg::Enter(key)` steps into a folder, `FileManagerMsg::Leave` steps out of it.
- `FolderEntry { name, folder, executable }` with `.is_hidden()`; `executable` is only read for a file whose name says nothing of its kind, and `FolderEntry::read_folder(path) -> Result<Vec<FolderEntry>, String>`, for an application that reads a folder its own way.
- `FileManagerMsg::Select | Choose | Expand | Read | Listed | NewFile | NewFolder | Rename | Cut | Copy | Paste | DropCut | Drop | Delete | DeleteConfirmed | Trash | ShowHidden | Refresh | Name | Submit | CloseNaming | Done | Changed | Detail | Detailed | Enter | Leave`. `Read` is what an application's own read answers with, in the system's words; `Listed` is the manager's own and keeps a `FileError`, so the words are chosen where the language is known.
- `copy_into(root, key, into, confined) -> Result<FileChange, FileError>`, for an application that copies a path its own way.
- `FileWork` — the long operation running now: `id()` (its `TaskId`, to show it in a `Tasks` model or stop it yourself), `done()`, `note()`, `entries()`.
- `FileChange::Created(key) | Moved(from, to) | Deleted(key) | Copied(key) | Trashed(key)`; `FileError::Name | Outside | IntoItself | Taken(name) | CrossDevice | Denied | NotReadable | Missing | NoTrash | NoRoom | Stopped | System(text)`, each with `.message()`.
- `RowMark::new()`, `.sign(icon, tone)` (the two always together), `.faint(bool)`; reading it: `icon()`, `tone()`, `is_faint()`, `is_empty()`.
- `qframe::icons::file_kind(name, folder, executable) -> FileKind` with `.icon()` (the icon key, such as `file-rust`) and `.family()`; `KindFamily::Folder | Text | Document | Sheet | Code | Data | Image | Audio | Video | Archive | Package | Executable | Key | Font | File` with `.tone()`. `UserFolders::english(home)`, `::parse(home, text)`, `::read(home, config)`, `::current()`, `.home()`, `.kind(path)`.
- `NameProblem::Empty | Slash | Nul | Dots | Taken` with `.message()`; `Naming { purpose, folder, value, tried }`; `NameFor::File | Folder | Rename(key)`.
- Keys as identities: `child_key(parent, name)`, `parent_key(key)`, `name_of(key)`, `is_within(key, folder)`, `is_inside(key)`.

## Behaviour

- The list has four columns — name, size, changed, permissions — and scrolls them sideways when they do not fit. Both flat views put a check beside each row for choosing several, and a row under them says how many entries the folder holds, or spins while a read takes longer than about 300 ms.

- Rows: the root first, then folders and files, each group in name order. An entry whose name the platform does not spell as text is shown lossily rather than left out.
- A folder is read once when it opens; opening it again shows what is known. Closing it while it is read throws the answer away, so a new folder of the same name starts closed and unread.
- `up` `down` `home` `end` move the cursor, `left` `right` and `enter` open and close a folder, `enter` on a file opens it, `space` and `ctrl`+click select several, `shift+f10` opens the row's menu. Rows dragged onto a folder move there; dropped on the free space below the rows they move to the root.
- A right click keeps the selection when it is on one of the selected rows and makes the row the selection otherwise, so the menu acts on what was clicked.
- A folder's menu: New file, New folder, then Rename and Cut (not at the root), then Paste here and Cancel the move while something is cut, then your items, then Refresh at the root or Delete below it. A file's menu leaves out what only a folder can do. A row of several selected offers Cut and Delete for all of them and no Rename, since a name is given to one entry at a time.
- Pasting into the folder that was cut, or into one inside it, is shown and cannot be chosen.
- Renaming opens with the name before its extension selected, so typing keeps the kind of file; a folder and a dotfile are selected whole.
- The name is checked as it is typed: empty (only after the first try), `/`, NUL, `.` and `..`, and a name the folder already has, which an entry's own name is not.
- Deleting asks first, in the danger colour, and says a folder takes everything in it along. Several entries are asked about once, by name up to five and counted after that.
- Operations run one after the other in the order given, each on its own. A refusal names the entry and the reason; what could be done was done. After a change the folders touched are read again and the cursor goes to what was made or moved.
- `confined()` refuses a key that climbs out of the root and a folder on the way that is a symbolic link. Without it those parts are still refused; only the link is followed.
- A move is a rename: a target on another file system is refused and said so, nothing is copied, and nothing already there is ever overwritten.
- Copying runs as a `Task`, not as a held thread: it says how far it has come, and a row above the rows names it, draws a progress bar and offers Stop. Stopping takes the half-written entry away again and reads the folders once more, because what had already been copied is on disk. One copy runs at a time; a second asked for while one runs does nothing.
- Copying is the other operation: `Copy` puts entries aside the way `Cut` does and `Paste` copies them instead of moving them, a folder with everything in it. What was copied stays where it is, so nothing about it is faint. A link is copied as a link. A name the target folder has is refused; nothing is overwritten.
- `trashing()` and `trashing_in(folder)` make deleting put an entry in a trash: the folder takes the place of Delete on every menu and nothing is asked, because the trash can be looked in again. The trash is written the way the freedesktop specification says: `files/` holds the entry, `info/<name>.trashinfo` says where it came from and when, the note is written first with `create_new` so it reserves the name, and a name already there takes the next number. An entry the trash cannot take — on another file system, or no trash at all — is never deleted quietly: the manager asks whether to delete it for good, in the danger colour, as its own question.
- `showing_hidden(bool)` shows the entries whose name starts with a dot. They are read either way, so turning it on goes nowhere near the disk, and a new name is checked against a hidden one that is already there whether it is shown or not.
- A refusal is said in the person's own words, never in the system's: a folder nobody may look into, something that is not there any more, a full disk and a copy that was stopped each have their own sentence.
- With `kind_icons(true)` a row's icon is its kind's: the whole name, then an ending of several parts, then the extension, then a folder's telling name or, for a hidden folder that says nothing more, `folder-hidden`, then a file that may be run, and otherwise `file` or `folder`. Letter case never matters. The root row takes the kind of the root's own name, whatever `root_label` says. The home and its folders (`folder-home`, `folder-downloads`…) are known by where they are, while the home is inside the root. The icon has no colour of its own: quiet in the list, the row's colour when selected, as the plain icons are.
- `kind_tones(true)` gives folders the accent and files four series tones: code and writing `series-2`; pictures, sound, video and fonts `series-3`; data and keys `series-4`; archives, packages and programs `series-5`. A file of no kind keeps the row's colour.
- A mark replaces the row's folder or file icon with its sign, in its tone, and can draw the row faint. It can never make a row louder: a cut entry and a disabled manager stay faint whatever the mark says. A tone cannot be given without a sign, so a marked row is still told apart in the sixteen-colour and the ASCII mode.
- `following(true)` watches the folders on screen. Created, removed and renamed reread that folder; modified rereads nothing; a folder that went away or an overflow rereads everything on screen. A batch of a watch that was let go is ignored.
- `.following_within(bound)` follows like `following(true)` with each wait lasting at most `bound`, then waiting again (`FileManagerMsg::Quiet`), so a screen test sees another program's change arrive by stepping the harness.

## Theme keys

- The manager uses the tree's styles: `list-item` (with `faint`), `tree-chevron`, `tree-drop`, `list-detail`, `spinner`, and the scrollbar's.
- The menu and the dialog use `context-menu`, `context-item` (with `danger`, `disabled`), `modal`, `modal-title`, `text-input`, `field-error`, `button.primary`.
- Icons: `folder`, `file`, `tree-expanded`, `tree-collapsed`; with `kind_icons`, the `file-*` and `folder-*` keys of the icon set, such as `file-rust`, `file-archive`, `folder-git`, `folder-downloads`.
- Strings: `quvyta.file-manager.*`.
