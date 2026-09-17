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

## Config directory

- Linux and other Unix: `$XDG_CONFIG_HOME` when absolute, else `$HOME/.config`.
- macOS: `$HOME/Library/Application Support`.
- Windows: `%APPDATA%`.
- None found: settings stay in memory with a warning diagnostic.

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
- Saving creates the directory, writes `settings.toml.tmp-<pid>`, syncs and renames; a file loaded with problems is copied to `settings.toml.bak` first.
- A float read with `get::<f64>` also accepts a whole number.
