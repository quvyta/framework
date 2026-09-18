## When to use

Use settings storage for choices the user expects to find again next time: theme, language, icons, the pillar, the selection slide, reduced motion, and your own preferences such as a default deploy region. It is not a database: keep documents, history and caches in their own files.

## Step by step

1. Load at start: `let settings = Settings::load("my-app");`. On Linux this reads `~/.config/my-app/settings.toml` (or `$XDG_CONFIG_HOME`).
2. Describe your keys with a schema: `Schema::builtin().choice("deploy.region", ["eu-west", "us-east"], "eu-west").flag("deploy.confirm", true)`. Declare `theme` and `language` again with the installed ids, which you only know while running. A key with no sensible default is optional: `.optional("deploy.note", SettingKind::text())`. A table other code fills, such as plugins' settings, is open: `.open("plugins")`.
3. Check the file against it, and let it repair itself if you want: `Settings::load("my-app").schema(schema).self_heal(true)`.
4. Show the saved appearance on the very first frame: `Runtime::new(App::new(settings.clone())).settings(&settings).run()`.
5. Read with defaults in `view` or `update`: `settings.get_or("deploy.region", "eu-west".to_owned())`.
6. Change and save in `update`: `if settings.set("deploy.region", region) { return settings.save_command(Msg::Saved); }`. The file is written on a background thread.
7. Show `settings.diagnostics()` somewhere quiet (a settings screen, the log) so the user learns why a hand-edited value was ignored or repaired.
8. In tests use `Settings::in_memory()` or `Settings::parse_str` so nothing touches the real config directory.

## How it works

- **Typed values.** Values are booleans, integers, floats, strings and lists. `get::<T>` returns `None` when the stored type differs or the number does not fit, so a wrong value falls back to your default.
- **Dotted keys are tables.** `deploy.region` is written as `region` under `[deploy]`; plain keys come first.
- **A schema says what is valid.** Each key has a kind (`flag`, `text`, `choice` from a list, or `check` with your own closure) and a default. An optional key has a kind (`SettingKind::flag()`, `text()`, `choice(..)`, `check(..)`) and no default. Without a schema only the built-in keys are checked: `theme`, `language`, `icons`, `reduced-motion`, `pillar` and `slide`.
- **Checking only warns.** With a schema, a key it does not know and a value it does not accept each become a warning with file, line and column. The file is not touched.
- **Self-healing repairs, key by key.** With `self_heal(true)` every key is judged on its own: valid keys stay, unknown keys are removed, invalid values are replaced by their default. The order of the keys is never a problem and is left as it is. If anything changed, the file as it was is kept as `settings.toml.bak` and the repaired file is saved once. Every repair is a warning in the diagnostics, so the user can see what happened. Healing is off by default and needs your schema: only you know all the keys of your application.
- **Missing keys use their default and are not written.** A key that is not in the file stays out of it, even while healing: `get_or` gives your default, and the file only holds what the user or your code stored.
- **Optional keys are kept only when valid.** A valid value stays; an invalid one is removed with a warning, since there is no default to put in its place; a missing one is not added and reads as `None`.
- **Open prefixes are kept as they are.** Every key under `.open("plugins")`, in `[plugins]` and every table below it, is never checked, reported or removed. A key you declare under the prefix still follows its own rule. `plugins = …` on its own is not under the table and is checked like any other key.
- **Atomic saves.** Saving goes through `atomic_write`: the new file is written next to the old one, flushed, renamed over it and then the folder itself is flushed, so a crash or a power cut leaves either the old or the new file, never half of one.
- **Broken files never stop the application.** Syntax errors and unusable entries become diagnostics; everything readable is still used. Before a broken file is first overwritten, it is copied next to itself with `.bak` added to its name: `settings.toml.bak`, or `code.conf.bak` for a family file.
- **No extra dependency.** The folders come from the platform's own variables, and the file is written by the framework's own small TOML writer. Only the lock needs a crate, for the one call the standard library does not have.

On this page, "Show a broken file" loads a hand-broken file with the showcase's schema, which declares `deploy.note` and `deploy.retries` as optional and opens `plugins`. Turn "Self-heal" on and off to compare: off lists warnings and keeps the file; on lists the repairs, shows the repaired file and writes each repair to the event log. In the repaired file the note stays, `retries = "twice"` is gone, `[plugins]` is untouched, and `deploy.confirm`, which the file never had, is not added. The showcase heals its own settings file this way when it starts.

## The rest of an application's files

Settings are one file; an application has others. Four things belong to all of them, and they live here so every application in the family does them the same way.

1. **Two folders, not one.** `config_dir("qfocus")` is for settings, `data_dir("qfocus")` for the application's own records. A recorded session is not a setting, and on Linux it does not belong in `.config`. Both give `None` when the platform has no home to write in, and neither creates the folder: `fs::create_dir_all` before the first write.
2. **Write every file with `atomic_write(path, contents)`.** It writes a temporary file in the same folder, flushes it, renames it over the real name and then flushes the folder. The last step is the one that is usually forgotten: without it a power cut can lose the rename, and then both names are gone, the old file replaced and the new one never on the disk. `atomic_write_reporting` is the same write with each finished step handed to a closure, for a log or a screen like the one on this page.
3. **Keep a second instance out with `AppLock::acquire(path)`.** `Ok(Some(lock))` means this instance may write, and it may write for as long as the value lives. `Ok(None)` means another process holds the lock, so show that and stay read-only. The lock is the operating system's, not the file's existence: when a process dies the kernel releases it, so a crash never leaves a lock nobody holds — and the recovery that has to write is never locked out by the crash it is recovering from. `holder_pid(path)` reads the process id out of the file for a message that names the holder, and for nothing else: a process id is reused, so it may never decide anything.
4. **One file per machine with `machine_name()`.** When the data folder is synced between machines, a file both of them write is a file one of them overwrites. Put the machine in the name instead: `format!("running-{machine}.toml")`. The name is `uname -n` on Unix and `COMPUTERNAME` on Windows, and it comes back ready for a file name: surrounding whitespace removed, any character other than a letter, a digit, `-`, `_` or `.` turned into `-`, dots at either end dropped. `None` means the system gave nothing usable; pick a fallback name of your own then.

## A family of applications

Applications made to be used together keep their settings in one folder, so the user finds all of them in one place. A `Family` names that folder; `Family::QUVYTA` is the Quvyta family.

1. **Load from the family's folder.** `Settings::load_member(&Family::QUVYTA, "code")` reads `~/.config/quvyta/code.conf` on Linux: the same TOML, the same schema and healing, only the file name says which application it belongs to. `family.shared_file()` is `quvyta.conf`, the settings every member shares, and `family.app_dir("code")` is the folder beside the file for everything else the application configures. On macOS the folder is `~/Library/Application Support/Quvyta`, on Windows `%APPDATA%\Quvyta`: there users read folder names as names, so the title is used. File names are always lowercase.
2. **Move the old settings in once, at start.** An application that kept its settings in a folder of its own calls `Family::QUVYTA.adopt("packages", &old)` before loading, with `old` the folder it used, for example `config_dir("quvyta-packages")`. `settings.toml` becomes `packages.conf`, every other file keeps its place under `packages/`, and emptied old folders are removed. When the old folder already is the application's folder, as `~/.config/quvyta/focus` is for focus, only `settings.toml` moves.
3. **Show what stayed behind.** `adopt` returns a `Migration`: `moved()` lists each file as `(from, to)`, `diagnostics()` says what was left and why, and `is_clean()` is true when nothing was. `Settings::load_member(..).with_diagnostics(migration.diagnostics().iter().cloned())` puts the report in front of what loading found, so a settings screen shows both. The showcase itself starts this way.
4. **Put the user's own work in the Documents folder.** `documents_dir()` is the folder a file manager calls Documents, under the name the desktop gave it: `~/Belgeler` on a Turkish Linux desktop, read from `user-dirs.dirs`. `Family::QUVYTA.workspace_dir("Code")` is `~/Belgeler/Quvyta/Code`. Neither is created by asking.

Moving is careful because a lost settings file cannot be fixed afterwards. Each file is copied with its permissions, read back and compared, and only then is the old one removed. A file whose new place is already taken stays where it is and is reported; nothing is overwritten and nothing is merged. Symbolic links are never followed or moved. A second start finds nothing to move and changes nothing. On this page, "Move the old folder in" runs the move in a folder of this run: two files move, one theme file stays because the family already has one, and pressing again shows what a second start finds.

## Common mistakes

- **Healing with an incomplete schema.** A key you forgot to declare is unknown, and self-healing removes it. Declare every key you store before turning healing on; the backup keeps the old file, but the user's choice is gone from the app.
- **Giving a default to a key that has none.** A made-up default such as `""` replaces an invalid value with something the user never chose; declare the key with `optional` instead.
- **Opening a prefix to silence warnings.** An open table is never repaired, so a typo inside it stays. Open only tables other code owns, and declare the keys you read yourself.
- **Hard-coding installed themes or languages.** Build the `choice` for `theme` and `language` from what the environment loaded, or a user's own theme is "repaired" away.
- **Saving in `view`.** `view` must not touch the disk; save from `update` with `save_command`.
- **Using the real config directory in tests.** Tests that call `Settings::load` read and write the developer's own settings.
- **Putting dots inside a key segment.** Dots always separate tables; use `-` inside names.
- **Keeping records next to the settings.** Sessions, history and logs belong in `data_dir`, not in `config_dir`; a user who copies their settings between machines does not want to carry the records along.
- **Deciding on the lock file instead of the lock.** "The file exists, so somebody is running" refuses to start after a power cut, exactly when the recovery has to write. Ask `AppLock::acquire`; a leftover file is not a lock, and the process id in it may be a process that died last week.
- **Letting the lock be dropped.** `AppLock::acquire(path)?;` takes the lock and releases it on the same line. Keep the value for as long as the instance runs.
- **Writing the temporary file somewhere else.** A temporary file in `/tmp` cannot be renamed onto another file system; `atomic_write` keeps it in the target folder for that reason.
- **Reading the machine name from `HOSTNAME`.** Most shells set it without exporting it, so a program started from them never sees it. `machine_name()` asks the kernel.
- **Building a file name from the raw host name.** A name can hold a `/`, a space or a leading dot; `machine_name()` has already made it safe, so use it as it comes.
- **Renaming over a link yourself.** A rename replaces a symbolic link with a plain file and cuts a settings file off from the dotfiles repository it was linked from. `atomic_write` follows the link and replaces the file it points at, in that file's folder.
- **Loading before adopting.** `load_member` before `adopt` starts from an empty file, and the first save takes the place the old settings were meant to move to; from then on they stay behind as a conflict.
- **Hard-coding `~/Documents`.** On a desktop in another language the folder has another name; ask `documents_dir()`.
- **Treating settings as the source of truth while running.** Keep the live value in your state and store it on change, as the showcase header does.
