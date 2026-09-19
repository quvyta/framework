## Methods

- `Family::preferences(app, &i18n)` — resolves language, theme and icons of `app`; creates the shared file with the detected values when it is missing.
- `Family::preferences_in(folder, app, &i18n)` — the same in `folder` instead of the platform's family folder.
- `Family::set(app, Shared::Theme, "nordic", Scope::Family)` — writes one shared key; `Scope::App` writes the application's file only. `set_in(folder, …)` in another folder.
- `Preferences::language()`, `theme()`, `icons()` — a `Resolved { value, source }` each; `source(key)` for one key.
- `Preferences::apply()` — the commands that switch a running application to the resolved values.
- `Preferences::diagnostics()` — located problems of the shared file.
- `Runtime::preferences(&prefs)` — start with the resolved values; they win over the same keys of `Runtime::settings`.
- `Settings::member_of(&family)` — the family's id is a valid value of the shared keys and reads as "not set here".
- `Appearance::new(family, app, prefs)` — the rows' state; `.in_folder(folder)` saves elsewhere.
- `Appearance::section(list, msg)` — a heading and the rows; `rows(list, msg)` without the heading.
- `Appearance::update(change, &mut settings)` — saves an `AppearanceChange` and returns the command that shows it.
- `SettingRow::nested(true)` — a row that belongs to the row above: its text starts two cells further in.

## Files

- Box under the row checked: `quvyta.conf` takes `theme = "nordic"`, `code.conf` takes `theme = "quvyta"`.
- Box cleared: `quvyta.conf` is unchanged, `code.conf` takes `theme = "nordic"`.

## Behaviour

- Order per key: the application's value (anything but the family's id), the shared file, detection (the language by `I18n::detect`, the theme always `monochrome`, the icons by `detect_glyph_mode`).
- A key missing from the application's file follows the family.
- The shared file cannot follow itself; `"quvyta"` in it is reported and the detected value is used.
- A change that cannot be saved is still applied, and its row says why until the next change.
- The reduced motion row is disabled while `QUVYTA_REDUCED_MOTION` decides.
