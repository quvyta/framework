## Methods

- `Family::preferences(app, &i18n)` — resolves language, theme and icons of `app`; creates the shared file with the detected values when it is missing.
- `Family::preferences_in(folder, app, &i18n)` — the same in `folder` instead of the platform's family folder.
- `Family::set(app, Shared::Theme, "nordic", Scope::Family)` — writes one shared key; `Scope::App` writes the application's file only. `set_in(folder, …)` in another folder.
- `Family::follow(app, Shared::Theme)` — puts `app` back on the family's value: `<app>.conf` alone takes `theme = "quvyta"`, the shared file is untouched. `follow_in(folder, …)` in another folder.
- `Preferences::language()`, `theme()`, `icons()` — a `Resolved { value, source }` each; `source(key)` for one key.
- `Preferences::apply()` — the commands that switch a running application to the resolved values.
- `Preferences::diagnostics()` — located problems of the shared file.
- `Runtime::preferences(&prefs)` — start with the resolved values; they win over the same keys of `Runtime::settings`.
- `Settings::member_of(&family)` — the family's id is a valid value of the shared keys and reads as "not set here".
- `Appearance::new(family, app, prefs)` — the rows' state; `.in_folder(folder)` saves elsewhere.
- `Appearance::section(list, msg)` — a heading and the rows; `rows(list, msg)` without the heading.
- `Appearance::update(change, &mut settings)` — saves an `AppearanceChange` and returns the command that shows it.
- `SettingRow::nested(true)` — a row that belongs to the row above: its text starts two cells further in.
- `Family::update_notice()`, `update_notice_in(folder)` — whether the family says when an update is out; on unless `quvyta.conf` says `update-notice = false`. `set_update_notice(on)`, `set_update_notice_in(folder, on)` write it; `Preferences::update_notice()` reads it with the rest. `Appearance::updates(list, msg)` is its row, right after `section`.
- `UpdateCheck::new(family, app, package, current, on_newer)` — the question to crates.io, with the `updates` feature; `.in_folders(config, state)` for a test, `.registry(address)` for a mirror or a test server. `Command::check_for_update(check)` asks it.
- `Update::latest()`, `current()`, `package()`, `toast()` — the answer, and the notice every member shows the same way; `Update::new(family, package, current, latest)` makes one by hand.
- `Harness::update_checks()`, `set_latest_version(Some("0.2.0"))` — a test's view of the question: recorded, answered with the version given, never asked over the network.

## Files

- Box under the row checked: `quvyta.conf` takes `theme = "nordic"`, `code.conf` takes `theme = "quvyta"`.
- Box cleared: `quvyta.conf` is unchanged, `code.conf` takes `theme = "nordic"`.
- `follow`: `quvyta.conf` is unchanged and not even read, `code.conf` takes `theme = "quvyta"`; a missing file is created holding only that key.

## Behaviour

- Order per key: the application's value (anything but the family's id), the shared file, detection (the language by `I18n::detect`, the theme always `monochrome`, the icons by `detect_glyph_mode`).
- A key missing from the application's file follows the family.
- The shared file cannot follow itself; `"quvyta"` in it is reported and the detected value is used.
- `follow` on a key that already follows the family changes nothing and is not an error; on a file that cannot be read it fails with `InvalidData` and leaves the file as it was.
- A change that cannot be saved is still applied, and its row says why until the next change.
- The reduced motion row is disabled while `QUVYTA_REDUCED_MOTION` decides.
