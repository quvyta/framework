## When to use

Use it whenever your application lets the person open a file in another program: a file explorer's Enter, an "Open with" menu, a file tree in an editor. The desktop already knows the answer. The shared MIME database names the file's type, every installed program says in its `.desktop` file which types it opens, and `mimeapps.list` holds the person's own defaults. Reading those gives the same answer the person's file manager gives.

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

## How it works

- **The type comes from the name first.** `globs2` patterns carry weights: the heaviest matching one wins, then a case-sensitive one over a case-blind one, then the longest, so `backup.tar.gz` is a compressed tar and not just gzip.
- **A name nobody knows is read.** The first 4 KiB decide: empty or NUL-free UTF-8 is `text/plain`, anything else `application/octet-stream`. A folder is `inode/directory`.
- **Kinds have parents.** `subclasses` says `text/x-rust` is a `text/plain`, and every `text/*` type is one anyway, so the editor the person chose for text opens Rust source too.
- **Kinds have names in words.** `openers.mime.comment(&mime, &lang)` gives "Rust source code" for `text/x-rust`, in the person's language when the database has it, for the header of an "Open with" dialog or a properties window.
- **Programs are found in order.** The user's `applications` folder comes before the system's; for the same id the first wins, and a `Hidden=true` file hides the id from the folders after it. Subfolders join the id with `-`. A `TryExec` not found on `PATH` drops the program.
- **The person's choices count.** `mimeapps.list` files are read most preferred first: `[Default Applications]`, `[Added Associations]`, and `[Removed Associations]`, which takes a program away from a kind in every less important file and from the programs that declare the kind, but never undoes its own file's additions. A desktop's own `<desktop>-mimeapps.list` comes before the shared one of the same folder.
- **Opening goes the right way.** A terminal program (`Terminal=true`) is handed the terminal and the screen comes back when it ends. A graphical one starts beside the application. Without `DISPLAY` or `WAYLAND_DISPLAY` a graphical program cannot start, and `launch` says so instead of trying. Either kind runs in the file's folder, as desktop file managers start it, so relative paths and "Save as" begin beside the file.
- **In tests nothing runs.** The harness records the handoff or the opening; read it with `handoffs()` or `opens()`.

On this page every file, program and choice lives in a folder of this run. Pick a file to see its kind and its programs. The pager is the default for plain text and so for Rust, the editor was added for Rust and comes first, and the image viewer was removed from PNG, so the diagram has no program. Turn off the graphical session to see a window program become one that cannot start.

## Common mistakes

- **Reading the owner's folders in a test.** Build an `XdgDirs` over a temporary folder and pass your own `PATH` for `TryExec`.
- **Assuming the first program is the default.** A program for the exact kind comes before the default of its parent kind; `Choices::default` says which one Enter opens.
- **Opening several files with one command.** `command` takes one file; ask once per file.
- **Hiding `NoDisplay` programs from "Open with".** They are only kept out of launchers; they still open files.
- **Writing `mimeapps.list`.** The defaults are the person's desktop setting. Changing them is a separate decision.
