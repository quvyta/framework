## Methods

- `Settings::load(app)` — reads `<config dir>/<app>/settings.toml`; missing file means empty settings.
- `Settings::open(path)` — reads a specific file.
- `Settings::in_memory()` — never saved; for tests.
- `Settings::parse_str(file, text)` — reads TOML text, reporting against `file`; saving does nothing.
- `.schema(schema)` — checks every loaded key against `schema`: unknown keys and invalid values become located warnings.
- `.self_heal(bool)` — off by default; with a schema, removes unknown keys, replaces invalid values with their default, removes invalid optional values, leaves open prefixes and missing keys alone, backs the file up and saves it once.
- `.get::<T>(key)`, `.get_or(key, default)`, `.set(key, value) -> bool` (changed), `.remove(key) -> bool`, `.value(key)`, `.keys()`.
- `.theme()`, `.language()`, `.icon_mode()`, `.reduced_motion()`, `.pillar_style()`, `.slide()` — the well-known keys `Settings::THEME`, `LANGUAGE`, `ICONS`, `REDUCED_MOTION`, `PILLAR`, `SLIDE`.
- `.apply::<Msg>()` — commands that switch to the saved theme, language, icons, reduced motion, pillar and slide.
- `.save()` — writes atomically; `.save_command(|Result<(), String>| msg)` — saves a copy on a background thread.
- `.to_toml()`, `.path()`, `.diagnostics()`.
- `Runtime::settings(&settings)` — applies the saved appearance before the first frame.
- `Setting` — implemented for `bool`, `String`, integer types, `f32`, `f64`, `Vec<String>`; `SettingValue` is the stored form.

## Schema

- `Schema::builtin()` — `theme` and `language` (any string; `monochrome`, `en`), `icons` (`auto`, `nerd`, `unicode`, `ascii`; `auto`), `reduced-motion` (`false`), `pillar` (`thick`, `thin`; `thick`), `slide` (`true`).
- `Schema::default()` — no keys.
- `.flag(key, default)` — `true` or `false`.
- `.text(key, default)` — any string.
- `.choice(key, choices, default)` — one string of `choices`, e.g. the installed theme ids.
- `.check(key, default, |value: &T| bool)` — reads as `T` and passes the closure, e.g. a number in a range.
- `.optional(key, kind)` — a key without a default; `kind` is `SettingKind::flag()`, `SettingKind::text()`, `SettingKind::choice(choices)` or `SettingKind::check(|value: &T| bool)`.
- `.open(prefix)` — keeps every key under the dotted table `prefix` (e.g. `plugins`: `[plugins]` and deeper) as it is; a trailing dot is ignored, `""` opens nothing, opening a prefix twice is the same as once.
- Declaring a key again replaces its rule, including between a key with a default and an optional one.

## Folders

- `config_dir(app)` — where the settings of `app` live: `$XDG_CONFIG_HOME/<app>` when absolute, else `$HOME/.config/<app>` when `HOME` is absolute on Linux and other Unix; `$HOME/Library/Application Support/<app>` on macOS; `%APPDATA%\<app>`, the roaming folder, on Windows.
- `data_dir(app)` — where `app` keeps its own records: `$XDG_DATA_HOME/<app>` when absolute, else `$HOME/.local/share/<app>` on Linux and other Unix; `$HOME/Library/Application Support/<app>` on macOS, the same folder as the settings, because macOS has no separate data folder for a command line application; `%LOCALAPPDATA%\<app>`, the local folder, on Windows, so records are not copied between machines.
- `state_dir(app)` — where `app` keeps what it remembers between runs, such as its last background check: `$XDG_STATE_HOME/<app>` when absolute, else `$HOME/.local/state/<app>` on Linux and other Unix; `$HOME/Library/Application Support/<app>` on macOS, the same folder as the data; `%LOCALAPPDATA%\<app>` on Windows.
- `cache_dir(app)` — where `app` keeps files it can rebuild at any time: `$XDG_CACHE_HOME/<app>` when absolute, else `$HOME/.cache/<app>` on Linux and other Unix; `$HOME/Library/Caches/<app>` on macOS; `%LOCALAPPDATA%\<app>` on Windows.
- A `HOME` that is not an absolute path counts as missing in all four, as a relative XDG variable does, so nothing lands under the working directory.
- An empty variable counts as unset; a relative `XDG_*` path is invalid and ignored.
- `None` when the platform's variables say nothing. `Settings::load` then keeps everything in memory with a warning diagnostic.
- Asking does not create the folder, and it does not have to exist.

## Ecosystem

- `Ecosystem::QUVYTA`, `Ecosystem::new(id, title)` — an ecosystem of applications: a lowercase `id` for folder and file names, a `title` for the places a user reads it as a name. `.id()`, `.title()`.
- `.config_dir()` — the ecosystem's folder: `$XDG_CONFIG_HOME/<id>` when absolute, else `$HOME/.config/<id>` on Linux and other Unix; `$HOME/Library/Application Support/<title>` on macOS; `%APPDATA%\<title>` on Windows. `None` without a home folder.
- `.shared_file()` — `<config_dir>/<id>.conf`, the settings every member shares.
- `.app_file(app)` — `<config_dir>/<app>.conf`. An application whose id is the ecosystem's own would get the shared file.
- `.app_dir(app)` — `<config_dir>/<app>`, for the application's other configuration files.
- `.state_dir(app)` — `<state folder>/<id>/<app>`: `$XDG_STATE_HOME/<id>/<app>` when absolute, else `$HOME/.local/state/<id>/<app>` on Linux and other Unix; `$HOME/Library/Application Support/<title>/<app>` on macOS; `%LOCALAPPDATA%\<title>\<app>` on Windows.
- `.cache_dir(app)` — `<cache folder>/<id>/<app>`: `$XDG_CACHE_HOME/<id>/<app>` when absolute, else `$HOME/.cache/<id>/<app>` on Linux and other Unix; `$HOME/Library/Caches/<title>/<app>` on macOS; `%LOCALAPPDATA%\<title>\<app>` on Windows.
- `.workspace_dir(app_title)` — `<documents_dir>/<title>/<app_title>`, where the user's work with the application goes.
- `Settings::load_member(&ecosystem, app)` — `Settings::open(ecosystem.app_file(app))`; without a home folder, settings in memory with the same warning as `Settings::load`.
- `Settings::with_diagnostics(diagnostics)` — puts diagnostics found around loading in front of the file's own; they stay through later schema checks.
- Nothing is created by asking.

## Adopting old settings

- `ecosystem.adopt(app, legacy_dir) -> Migration` — `legacy_dir/settings.toml` to `app_file(app)`; every other file under `legacy_dir`, at any depth, to the same relative path under `app_dir(app)`.
- `ecosystem.adopt_in(config_dir, app, legacy_dir)` — the same move with `config_dir` as the ecosystem's folder, for tests and demos that must not touch the user's own settings.
- When `legacy_dir` is `app_dir(app)`, only `settings.toml` moves and the folder is kept.
- Each move: the new name is claimed (created empty, failing if anything is there; on Unix with the old file's permissions), filled with `atomic_write`, given the old file's permissions, read back and compared; only then is the old file removed. A failed step removes the new file and keeps the old one.
- A taken new place, a symbolic link (never followed, never moved), anything that is not a plain file, and a file that cannot be read stay where they are, each with a diagnostic naming the path. A symbolic link or a file as `legacy_dir`, and a `legacy_dir` inside `app_dir` or around it, adopt nothing.
- Emptied old folders are removed from the deepest up; a folder with anything left is kept; `legacy_dir` is never removed when it is `app_dir`.
- A missing `legacy_dir` is nothing to do; a second run changes nothing. Without a home folder nothing happens and the report says so.
- `Migration` — `.moved()` as `(from, to)` pairs, `.diagnostics()` (warnings for what was left on purpose, errors for what failed), `.is_clean()`.
- A crash in the middle of a move can leave an empty new file next to the whole old one; the next run reports the pair and keeps both.

## Documents folder

- `documents_dir()` — Linux and other Unix: the `XDG_DOCUMENTS_DIR` line of `user-dirs.dirs` in the config root (`$XDG_CONFIG_HOME` when absolute, else `$HOME/.config`); the value is `"$HOME/…"` or an absolute path in double quotes, `#` starts a comment line, a backslash escapes the next character, nothing else is expanded, and the last valid line wins. A missing, unreadable or broken file, and a value that is the home folder itself (how the file turns a folder off), give `$HOME/Documents`.
- `user_dir(UserDir::Desktop)` — any of the person's folders by the same rules, under the name the desktop gave it: `Desktop`, `Documents`, `Downloads`, `Music`, `Pictures`, `Videos`, `Public`, `Templates` (`UserDir::ALL`); `~/Masaüstü` for the desktop on a Turkish Linux desktop. The English name in the home when the file does not name it. `user_dir_in(which, home, config)` reads only the given folders, for a test. `documents_dir()` is `user_dir(UserDir::Documents)`.
- macOS: `$HOME/Documents`. Windows: the Documents Known Folder through the `dirs` crate, else `%USERPROFILE%\Documents`.
- `None` without an absolute home folder. Not created.

## Machine name

- `machine_name() -> Option<String>` — this machine's name, ready for a file name: the node name `uname -n` prints on Unix (read with `rustix`, no shell and no file), `COMPUTERNAME` on Windows, `None` elsewhere.
- Made safe in three steps: whitespace around it removed; every character that is not a letter or digit (of any script), `-`, `_` or `.` replaced by `-`, bytes that are not UTF-8 included; dots at either end removed.
- `None` when the platform gives no name or nothing is left after those steps.
- Read again on every call, not cached. The unchanged system name is not offered.

## Atomic write

- `atomic_write(path, contents)` — leaves either the file that was there or the new one, never half of either. The parent folder has to exist. A `path` that is a symbolic link is written through: the file it points at is replaced, keeps its permissions, and the link stays a link, so a settings file linked from a dotfiles repository stays that repository's file. Links in a loop are an error.
- `atomic_write_reporting(path, contents, |step| …)` — the same write, reporting each finished step.
- `WriteStep` — `Wrote(PathBuf)` (the temporary file, next to the real one), `SyncedFile`, `Renamed`, `SyncedDirectory`, in that order.
- The temporary file is `<name>.tmp-<pid>-<n>` in the same folder, because a rename cannot cross a file system; the counter keeps two writers of one path apart.
- A failed step removes the temporary file, so a failed write leaves nothing behind, and the error is the first step's error.
- On Unix the folder is flushed after the rename, which is what makes the new name survive a power cut. On other platforms, Windows among them, the standard library cannot open a folder to flush it, so `SyncedDirectory` is not reported: no half-written file can appear, but a power cut just after the rename can leave the old file. Doing better needs calls this framework cannot make without `unsafe`.

## One instance

- `AppLock::acquire(path)` — `Ok(Some(lock))` took it, `Ok(None)` another process holds it, `Err` could not try. The parent folder has to exist.
- The lock lives as long as the value: dropping it releases it, and so does the process ending, however it ends. A leftover lock file is not a lock.
- On Unix this is `flock(LOCK_EX | LOCK_NB)`. Such a lock belongs to an open file, not to a process, so a second `acquire` on the same path inside one process answers `None` as well.
- On every other platform, Windows among them, there is no advisory lock here: `acquire` returns `io::ErrorKind::Unsupported` and never `Ok`, so an application is told it has no lock instead of quietly running without one.
- `holder_pid(path)` — the process id written in the lock file, for a message that names the holder. Diagnostic text only: a process id is reused, so no decision may rest on it.
- `InstanceLock::shared(path) -> io::Result<InstanceLock>` — a shared lock; any number are held at once. Waits only while an exclusive lock is held.
- `InstanceLock::try_exclusive(path) -> io::Result<Option<InstanceLock>>` — the exclusive lock when nobody holds a lock, `Ok(None)` at once otherwise.
- `InstanceLock::wait_exclusive(path) -> io::Result<InstanceLock>` — sleeps in the kernel until nobody holds a lock, then takes the exclusive one. Returns at once when nobody does; it cannot be cancelled, so give it a thread of its own.
- An `InstanceLock` is released when the value is dropped or its process ends, however it ends. On Unix these are `flock(LOCK_SH)`, `flock(LOCK_EX | LOCK_NB)` and `flock(LOCK_EX)` on a file opened close-on-exec; like `AppLock` they belong to the open file, so two locks on one path in one process meet each other. Elsewhere every call returns `io::ErrorKind::Unsupported`.

## Behaviour

- Syntax errors are located; the readable rest of the file is kept.
- Datetimes and arrays of tables are skipped with a warning.
- Diagnostics point at the key itself, never at the same word elsewhere in the file.
- Without a schema, the built-in keys are checked: `theme` and `language` must be strings, `reduced-motion` and `slide` booleans, `icons` one of `auto`, `nerd`, `unicode`, `ascii`, `pillar` one of `thick`, `thin`. Otherwise a located warning and the typed helpers ignore the value. Other keys are not reported.
- With a schema and self-healing off, unknown keys and invalid values are warnings and the file is untouched.
- With a schema and self-healing on, each key is judged alone; key order is never changed or reported; a known key whose value settings cannot store gets its default; the file as it was is copied to `settings.toml.bak` and the repaired file is saved once; each repair is a warning. A save that fails is an error diagnostic, and the repaired values are still used.
- A key missing from the file is never added, with or without healing: typed reads give `None` and `get_or` gives your default.
- An optional key: valid, kept; invalid, a warning (`it is ignored` without healing, `removed` with it); holding a value settings cannot store, removed while healing; missing, nothing happens.
- Keys under an open prefix are never reported or removed; a rule declared under the prefix still checks and heals its key. The key named by the prefix itself (`plugins = 1`) is not under it.
- While healing, keys one file cannot hold together (`git = 1` next to `"git.sign" = true` in the same table, both read as dotted keys) are separated: a declared key wins over an open one, otherwise the key written first stays; the other is removed with a warning.
- Datetimes and arrays of tables under an open prefix are skipped like everywhere else, so they are not in a healed file; the backup keeps them.
- `.schema` and `.self_heal` can be called in either order; repairs stay in the diagnostics when the check runs again.
- Saving creates the folder and goes through `atomic_write`; a file loaded with problems is copied first under its own name with `.bak` added: `settings.toml.bak`, `code.conf.bak`.
- A float read with `get::<f64>` also accepts a whole number.
