## When to use

Use it whenever your application lets the person open a file in another program: a file explorer's Enter, an "Open with" menu, a file tree in an editor. The desktop already knows the answer. The shared MIME database names the file's type, every installed program says in its `.desktop` file which types it opens, and `mimeapps.list` holds the person's own defaults. Reading those gives the same answer the person's file manager gives. A desktop or a launcher goes further and lists every program on the machine; the same read answers that with the rest of every entry, so it never parses a `.desktop` file a second time.

- **Do not keep a table of your own.** An extension list in your code disagrees with the desktop the day a program is installed.
- **Do not open everything with `$EDITOR`.** A picture belongs in an image viewer; a Rust file in the editor the person chose for text.
- **Do not build a shell line.** `DesktopApp::command` returns a list of arguments, so a file called `my "notes"; rm -rf ~.txt` is one argument and nothing else.

## Step by step

1. Read the folders once at start: `let dirs = XdgDirs::from_env(|name| std::env::var(name).ok());`.
2. Load the kinds and the programs: `let openers = Openers::load(&dirs, &lang, std::env::var_os("PATH").as_deref());`, where `lang` is the person's language as in `LANG`.
3. Ask about one file: `let choices = openers.for_file(&path);` gives its kind, its programs and which of them is the default.
4. Ask once whether windows can open: `let graphical = graphical_session(|name| std::env::var(name).ok());`.
5. Open: `app.launch(&path, graphical, Msg::Opened)` gives the command to return from `update`, or `NoGraphicalSession`.
6. In a menu, dim what cannot start: `app.can_start(graphical)`.
7. In a launcher, read with `Include::MISSING` and ask every program what its entry says: `openers.apps.details(&app.id)`.

## How it works

- **The type comes from the name first.** `globs2` patterns carry weights: the heaviest matching one wins, then a case-sensitive one over a case-blind one, then the longest, so `backup.tar.gz` is a compressed tar and not just gzip.
- **A name nobody knows is read.** The first 4 KiB decide: empty or NUL-free UTF-8 is `text/plain`, anything else `application/octet-stream`. A folder is `inode/directory`.
- **Kinds have parents.** `subclasses` says `text/x-rust` is a `text/plain`, and every `text/*` type is one anyway, so the editor the person chose for text opens Rust source too.
- **Kinds have names in words.** `openers.mime.comment(&mime, &lang)` gives "Rust source code" for `text/x-rust`, in the person's language when the database has it, for the header of an "Open with" dialog or a properties window.
- **Programs are found in order.** The user's `applications` folder comes before the system's; for the same id the first wins, and a `Hidden=true` file hides the id from the folders after it. Subfolders join the id with `-`. A `TryExec` not found on `PATH` drops the program.
- **An entry says more than a command.** `openers.apps.details(&app.id)` gives what the entry says beside its `Exec` line: `Comment` and `GenericName` in the person's language, `Keywords` for a search, `Categories` for a menu, `Path`, and the `NoDisplay`, `Hidden` and `TryExec` keys with whether the program `TryExec` names is installed. A launcher lists, searches and groups programs with exactly these, so it reads every `.desktop` file once and keeps no parser of its own.
- **Starting it with nothing to open.** `app.launch_command()` is the same `Exec` line with no file: `%f %F %u %U` are gone and nothing is left in their place, so `foo %U --x` is `foo --x` rather than a command with an empty argument. A launcher's own list, a favourite and a panel item start a program this way; opening a file still goes through `command`.
- **A program that is not installed is still a program.** A plain read drops an entry whose `TryExec` names a program this machine has not got, since it can open nothing. A launcher that lists everything reads `Apps::load_including(&dirs, lang, path, Include::MISSING)`, builds `Openers` from it, and names the program with the reason, where it can offer the package too. It is in no file's list of programs, because it can open nothing. `Include::HIDDEN` brings back the entries marked `Hidden`: they say a program was deleted, so an application can see what is gone, and they are never offered as a program that opens a file.
- **The person's choices count.** `mimeapps.list` files are read most preferred first: `[Default Applications]`, `[Added Associations]`, and `[Removed Associations]`, which takes a program away from a kind in every less important file and from the programs that declare the kind, but never undoes its own file's additions. A desktop's own `<desktop>-mimeapps.list` comes before the shared one of the same folder.
- **The default can be written back.** `set_default(&dirs, mime, id)` changes the kind's one line inside `[Default Applications]` of `$XDG_CONFIG_HOME/mimeapps.list` and nothing else. Every other section, comment, blank line and line nobody understands is put back byte for byte, and a line for the same kind under `[Removed Associations]` is left alone, since that is the person saying which program they do *not* want. `Change::Added` or `Change::Replaced` says whether the line is new, and reading the file again with `Apps::load` gives the new program as that kind's default. One file is written and no other: not a file under `$XDG_CONFIG_DIRS`, not a running desktop's `<desktop>-mimeapps.list`, not a `.desktop` entry.
- **Opening goes the right way.** A terminal program (`Terminal=true`) is handed the terminal and the screen comes back when it ends. A graphical one starts beside the application. Without `DISPLAY` or `WAYLAND_DISPLAY` a graphical program cannot start, and `launch` says so instead of trying. Either kind runs in the file's folder, as desktop file managers start it, so relative paths and "Save as" begin beside the file.
- **In tests nothing runs.** The harness records the handoff or the opening; read it with `handoffs()` or `opens()`.

On this page every file, program and choice lives in a folder of this run. Pick a file to see its kind and its programs. The pager is the default for plain text and so for Rust, the editor was added for Rust and comes first, and the image viewer was removed from PNG, so the diagram has no program. Under the default program, what its entry says besides the command is written out key by key; turn the language to Turkish and it is read in Turkish. The photo program at the bottom is in no file's list: its own program is not on this machine, and a launcher names it so its package can be offered. Turn off the graphical session to see a window program become one that cannot start. In the playground, choose a program for the type on screen and make it the default: one line of the demo's own `mimeapps.list` changes and the badge moves to it. The first press on a type with no line of its own adds one and says so; press again and the line stood there before and is replaced.

## Common mistakes

- **Reading the owner's folders in a test.** Build an `XdgDirs` over a temporary folder and pass your own `PATH` for `TryExec`.
- **Assuming the first program is the default.** A program for the exact kind comes before the default of its parent kind; `Choices::default` says which one Enter opens.
- **Opening several files with one command.** `command` takes one file; ask once per file.
- **Hiding `NoDisplay` programs from "Open with".** They are only kept out of launchers; they still open files.
- **Parsing every `.desktop` file a second time.** The reader has the rest of each entry: `Apps::details` is what a launcher lists, searches and groups programs with. A parser of your own is a second set of rules for the same files, and it drifts from them.
- **Rewriting `mimeapps.list` as a whole.** The file is the person's and their desktop's, and it holds comments and sections nobody has explained. `set_default` changes the one line asked for and puts the rest back byte for byte; a file you rebuild is a file that loses something.
- **Putting the person's choice in a `.desktop` entry.** An entry says what a program opens. What the person chose is `mimeapps.list`, and only in their own copy of it.
