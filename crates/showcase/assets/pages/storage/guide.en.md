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
- **Atomic saves.** The new file is written next to the old one, flushed to disk and renamed over it, so a crash leaves either the old or the new file, never half of one.
- **Broken files never stop the application.** Syntax errors and unusable entries become diagnostics; everything readable is still used. Before a broken file is first overwritten, it is copied to `settings.toml.bak`.
- **No extra dependency.** The config directory comes from `XDG_CONFIG_HOME`, `HOME` or `APPDATA` directly, and the file is written by the framework's own small TOML writer.

On this page, "Show a broken file" loads a hand-broken file with the showcase's schema, which declares `deploy.note` and `deploy.retries` as optional and opens `plugins`. Turn "Self-heal" on and off to compare: off lists warnings and keeps the file; on lists the repairs, shows the repaired file and writes each repair to the event log. In the repaired file the note stays, `retries = "twice"` is gone, `[plugins]` is untouched, and `deploy.confirm`, which the file never had, is not added. The showcase heals its own settings file this way when it starts.

## Common mistakes

- **Healing with an incomplete schema.** A key you forgot to declare is unknown, and self-healing removes it. Declare every key you store before turning healing on; the backup keeps the old file, but the user's choice is gone from the app.
- **Giving a default to a key that has none.** A made-up default such as `""` replaces an invalid value with something the user never chose; declare the key with `optional` instead.
- **Opening a prefix to silence warnings.** An open table is never repaired, so a typo inside it stays. Open only tables other code owns, and declare the keys you read yourself.
- **Hard-coding installed themes or languages.** Build the `choice` for `theme` and `language` from what the environment loaded, or a user's own theme is "repaired" away.
- **Saving in `view`.** `view` must not touch the disk; save from `update` with `save_command`.
- **Using the real config directory in tests.** Tests that call `Settings::load` read and write the developer's own settings.
- **Putting dots inside a key segment.** Dots always separate tables; use `-` inside names.
- **Treating settings as the source of truth while running.** Keep the live value in your state and store it on change, as the showcase header does.
