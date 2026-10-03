## Folders

- `XdgDirs::from_env(lookup) -> XdgDirs` — `XDG_DATA_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_HOME`, `XDG_CONFIG_DIRS` with their standard fallbacks, and `XDG_CURRENT_DESKTOP`. `lookup` returns a variable, such as `|name| std::env::var(name).ok()`. An empty value counts as unset and a relative folder is ignored.
- `XdgDirs { data_home, data_dirs, config_home, config_dirs, desktops }` — public fields, so a test builds one over a temporary folder.

## Kinds

- `MimeDb::load(&XdgDirs) -> MimeDb` — `globs2`, `aliases` and `subclasses` from every data folder's `mime`, the person's own first. `__NOGLOBS__` drops a kind's patterns from the folders after it.
- `db.guess(name) -> Option<String>` — from the name alone: weight, then a case-sensitive pattern over a case-blind one, then the longest.
- `db.sniff(path) -> String` — a folder is `inode/directory`, a pipe, socket or device its own `inode/*`; otherwise the name, and when no pattern fits the first 4 KiB: `text/plain` or `application/octet-stream`.
- `db.canonical(mime) -> String` — the kind's own name when `mime` is an alias.
- `db.ancestors(mime) -> Vec<String>` — `mime` first, then every kind it is a case of, breadth first; every `text/*` is `text/plain`, and all but `inode/*` end in `application/octet-stream`.
- `db.comment(mime, lang) -> Option<String>` — the kind in words ("Rust source code"), from the `<comment>` lines of `mime/<kind>.xml` in the data folders, the person's own first. An alias is followed first; the comment in `lang` (`pt-BR`, `tr_TR.UTF-8`) wins, then its base language (`pt`), then the one with no language. XML's character references are decoded. `None` when no folder describes the kind.
- `db.comment_with_diagnostics(mime, lang, &mut Vec<Diagnostic>) -> Option<String>` — the same, with a diagnostic for each file that is not UTF-8, has a `<comment>` never closed, or is not a regular file; such a file is skipped and a less important folder may still answer.
- `db.diagnostics() -> &[Diagnostic]`.

## Programs

- `Apps::load(&XdgDirs, lang, path_var) -> Apps` — the desktop entries below every `applications` folder and every `mimeapps.list`; names, comments and keywords in `lang` (`tr_TR.UTF-8`); `TryExec` looked for on `path_var` (a `PATH` value).
- `Apps::load_including(&XdgDirs, lang, path_var, include) -> Apps` — the same read, keeping what `include` asks for: `Include::MISSING` a program whose `TryExec` program is not installed, `Include::HIDDEN` an entry marked `Hidden`, `Include::ALL` both, `Include::NONE` neither. `Openers::load` has no keeping form; build `Openers { mime: MimeDb::load(&dirs), apps }` instead.
- `EntryDetails { comment, generic_name, categories, keywords, folder, no_display, hidden, try_exec, installed }` — public fields, one for each program of `all()` in the same order. `comment` and `generic_name` are in the person's language and `None` when the key is missing or empty; `keywords` are the ones in that language and then the plain ones, each written once and in the order the entry writes them; `try_exec` is the `TryExec` program as written and `installed` says whether it was found.
- `apps.details(id) -> Option<&EntryDetails>` — what the entry of that desktop file id says, when it is installed.
- `shell_words(line) -> Option<Vec<String>>` — a command line such as `$EDITOR` split the way a POSIX shell splits it, quotes and backslashes followed, nothing expanded; `None` for an unclosed quote. A path with spaces in quotes stays one word.
- `apps.for_mime(&db, mime) -> Vec<&DesktopApp>` — for the kind and then each kind it is a case of: every file's default, the added programs, then the programs that declare it. A removed one never, and neither does one that cannot start: a program the person deleted, or one whose own program is not installed.
- `apps.default_for(&db, mime) -> Option<&DesktopApp>` — the nearest kind's default, else the first program of `for_mime`.
- `apps.all()`, `apps.get(id)`, `apps.diagnostics()`.
- `DesktopApp { id, name, exec, terminal, mime_types, path, icon }` — public fields.
- `app.launch_command() -> Option<Vec<OsString>>` — the same line with no file to open: `%f %F %u %U` stand for nothing and the argument they stood alone in goes with them, `%c` the name, `%k` the entry and `%i` `--icon <icon>` as in `command`, `%%` a percent, and nothing is added at the end. `None` for a line that gives no command.
- `app.command(&Path) -> Option<Vec<OsString>>` — the `Exec` line filled in: `%f %F %u %U` the file, `%c` the name, `%k` the entry, `%i` `--icon <icon>`, `%%` a percent; retired codes dropped; with no file code the file goes last. `None` for a line that gives no command.

## One file

- `Openers::load(&XdgDirs, lang, path_var) -> Openers` — `mime` and `apps` read together; `openers.diagnostics()`.
- `openers.for_file(&Path) -> Choices` — `Choices { mime, apps, default }`, where `default` is an index into `apps`, `None` only when there is no program.

## Opening

- `graphical_session(lookup) -> bool` — `DISPLAY` or `WAYLAND_DISPLAY` is set and not empty.
- `app.can_start(graphical) -> bool` — a terminal program always can, a graphical one only in a graphical session.
- `app.launch(&Path, graphical, on_done) -> Result<Command<Msg>, LaunchError>` — a `Handoff` for a terminal program, an `Open::program` for a graphical one; either runs in the file's folder, which the harness records as `dir`.
- `Launched::Returned { code }`, `Launched::Started`, `Launched::Failed(reason)`.
- `LaunchError::NoGraphicalSession`, `LaunchError::NoCommand`.

## The default program

- `set_default(&XdgDirs, mime, app_id) -> Result<Change, SetDefaultError>` — writes `<mime>=<app_id>;` into the kind's line inside `[Default Applications]` of `$XDG_CONFIG_HOME/mimeapps.list`, and says whether that line is new. The file and its folder are made when the person has none; a file that is there is read, changed in one line and replaced whole. Reading the file again with `Apps::load` from the same `XdgDirs` gives `app_id` as that kind's default.
- `Change::Added`, `Change::Replaced`; the enum is `#[non_exhaustive]`.
- `SetDefaultError::Io(io::Error)`, `SetDefaultError::NoConfigHome`, `SetDefaultError::NotText`, `SetDefaultError::ReadOnly`, `SetDefaultError::InTheWay { line }`, `SetDefaultError::Unstorable`; the enum is `#[non_exhaustive]`. On every one of them the file is left as it was, and a line already written for `mime` keeps the program it names.

## Behaviour

- Missing files are normal. A line that cannot be read is skipped with a warning at its file, its line and, where the fault is a key rather than the whole line, the column the key is written at; an application entry without `Name`, without `Exec` or with an `Exec` that gives no command is skipped with a warning. Its id stays taken, so a broken copy of the person's never brings back the system's.
- Entries that are not `Type=Application` and `Hidden=true` entries take their id without offering a program; `Include::HIDDEN` keeps the hidden ones with `hidden == true`, so an application can see what the person deleted. `NoDisplay` programs are kept: they still open files, and `details` says so.
- An entry whose `TryExec` program is not found is dropped; `Include::MISSING` keeps it with `installed == false`, so a launcher can list it, say so and offer its package. Neither a deleted program nor one that is not installed is ever offered by `for_mime` or `default_for`: it can start nothing.
- A file's own `[Removed Associations]` do not undo its own additions; they remove what less important files add and what entries declare.
- Only regular files up to 16 MiB are read, so a pipe with a database's name never blocks.
- The binary `magic` rules are not read, and of the XML descriptions only the `<comment>` lines, line by line, with no XML parser.
- The only file ever written is the person's own `$XDG_CONFIG_HOME/mimeapps.list`, and only by `set_default`. A file under `$XDG_CONFIG_DIRS`, a running desktop's `<desktop>-mimeapps.list` and every `.desktop` entry are read and never written.
- `set_default` puts the rest of the file back byte for byte: other sections, comments, blank lines and lines it does not understand all stay, as does a line for the same kind under `[Removed Associations]`, and a second line for the same kind inside `[Default Applications]` where the first one is the one a reader takes. A kind with no line of its own gets a section of its own at the end of the file when the file has none.
