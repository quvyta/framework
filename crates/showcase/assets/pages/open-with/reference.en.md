## Folders

- `XdgDirs::from_env(lookup) -> XdgDirs` — `XDG_DATA_HOME`, `XDG_DATA_DIRS`, `XDG_CONFIG_HOME`, `XDG_CONFIG_DIRS` with their standard fallbacks, and `XDG_CURRENT_DESKTOP`. `lookup` returns a variable, such as `|name| std::env::var(name).ok()`. An empty value counts as unset and a relative folder is ignored.
- `XdgDirs { data_home, data_dirs, config_home, config_dirs, desktops }` — public fields, so a test builds one over a temporary folder.

## Kinds

- `MimeDb::load(&XdgDirs) -> MimeDb` — `globs2`, `aliases` and `subclasses` from every data folder's `mime`, the person's own first. `__NOGLOBS__` drops a kind's patterns from the folders after it.
- `db.guess(name) -> Option<String>` — from the name alone: weight, then a case-sensitive pattern over a case-blind one, then the longest.
- `db.sniff(path) -> String` — a folder is `inode/directory`, a pipe, socket or device its own `inode/*`; otherwise the name, and when no pattern fits the first 4 KiB: `text/plain` or `application/octet-stream`.
- `db.canonical(mime) -> String` — the kind's own name when `mime` is an alias.
- `db.ancestors(mime) -> Vec<String>` — `mime` first, then every kind it is a case of, breadth first; every `text/*` is `text/plain`, and all but `inode/*` end in `application/octet-stream`.
- `db.diagnostics() -> &[Diagnostic]`.

## Programs

- `Apps::load(&XdgDirs, lang, path_var) -> Apps` — the desktop entries below every `applications` folder and every `mimeapps.list`; names in `lang` (`tr_TR.UTF-8`); `TryExec` looked for on `path_var` (a `PATH` value).
- `apps.for_mime(&db, mime) -> Vec<&DesktopApp>` — for the kind and then each kind it is a case of: every file's default, the added programs, then the programs that declare it. Removed and hidden ones never.
- `apps.default_for(&db, mime) -> Option<&DesktopApp>` — the nearest kind's default, else the first program of `for_mime`.
- `apps.all()`, `apps.get(id)`, `apps.diagnostics()`.
- `DesktopApp { id, name, exec, terminal, mime_types, path, icon }` — public fields.
- `app.command(&Path) -> Option<Vec<OsString>>` — the `Exec` line filled in: `%f %F %u %U` the file, `%c` the name, `%k` the entry, `%i` `--icon <icon>`, `%%` a percent; retired codes dropped; with no file code the file goes last. `None` for a line that gives no command.

## One file

- `Openers::load(&XdgDirs, lang, path_var) -> Openers` — `mime` and `apps` read together; `openers.diagnostics()`.
- `openers.for_file(&Path) -> Choices` — `Choices { mime, apps, default }`, where `default` is an index into `apps`, `None` only when there is no program.

## Opening

- `graphical_session(lookup) -> bool` — `DISPLAY` or `WAYLAND_DISPLAY` is set and not empty.
- `app.can_start(graphical) -> bool` — a terminal program always can, a graphical one only in a graphical session.
- `app.launch(&Path, graphical, on_done) -> Result<Command<Msg>, LaunchError>` — a `Handoff` for a terminal program, an `Open::program` for a graphical one.
- `Launched::Returned { code }`, `Launched::Started`, `Launched::Failed(reason)`.
- `LaunchError::NoGraphicalSession`, `LaunchError::NoCommand`.

## Behaviour

- Missing files are normal. A line that cannot be read is skipped with a warning at its file and line; an application entry without `Name`, without `Exec` or with an `Exec` that gives no command is skipped with a warning. Its id stays taken, so a broken copy of the person's never brings back the system's.
- Entries that are not `Type=Application` and `Hidden=true` entries take their id without offering a program. `NoDisplay` programs are kept: they still open files.
- A file's own `[Removed Associations]` do not undo its own additions; they remove what less important files add and what entries declare.
- Only regular files up to 16 MiB are read, so a pipe with a database's name never blocks.
- The binary `magic` rules and the XML descriptions are not read. Nothing is ever written.
