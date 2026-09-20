## When to use

Every application of the Quvyta family speaks the same language, draws with the same theme and the same icons unless the user chose otherwise for one of them. Use `Family::preferences` at start to resolve those three, and put the ready-made `Appearance` rows on your settings page instead of building language, theme, icon, reduced motion and pillar rows yourself.

## Step by step

1. At start, after `Family::adopt` and `Settings::load_member`, resolve the shared preferences: `let prefs = Family::QUVYTA.preferences("code", &i18n);`. The first application that starts creates `quvyta.conf` with the values detected on the machine.
2. Start the runtime with them, after your own settings: `Runtime::new(app).settings(&settings).preferences(&prefs)`.
3. Keep an `Appearance` in your state: `Appearance::new(Family::QUVYTA, "code", prefs)`.
4. Place it in a settings list: `SettingsList::show(ui, |list| self.appearance.section(list, Msg::Appearance))`. A setup wizard's first step uses `rows` instead, without the heading.
5. Hand every change back: `Msg::Appearance(change) => self.appearance.update(change, &mut self.settings)`. It saves the change and returns the command that shows it at once.

## How it works

- **Each key on its own.** For language, theme and icons: the application's own value when its file names one, else `quvyta.conf`, else what the machine suggests. A missing key and the value `"quvyta"` both mean "follow the family".
- **Back to the family, on its own.** `Family::follow(app, key)` writes the family's id in that application's file and nothing else: the shared file is neither read nor written, so one member goes back to following without changing what the whole family draws with. `Family::set(.., Scope::Family)` also writes the shared value, which is what you want on the application's own settings page and not what you want on a row for another member.
- **The box under a shared row is the scope.** Checked, a change goes to `quvyta.conf` and the application's file says `"quvyta"`, so every application that follows the family changes with it. Cleared, the change stays in the application's own file. An application that chose its own value is never changed from another one.
- **Two writers keep both changes.** Each file is read right before it is written and only the changed key is written, with the folder held by an advisory lock on Unix.
- **Nothing stops on a broken line.** That key falls back to the detected value and the reason, with file, line and column, is in `Preferences::diagnostics`.
- **Reduced motion and the pillar are the application's own.** They sit in the same section and are saved in the application's file. When `QUVYTA_REDUCED_MOTION` decides, the row is disabled and says so.
- **Where a value came from.** `Resolved::source` is `App`, `Family` or `Detected`.

## Common mistakes

- **Declaring `language` with a fixed list and self-healing.** Call `Settings::member_of(&Family::QUVYTA)` before `self_heal`, or load with `load_member`, so `"quvyta"` is kept.
- **Saving the whole settings file from an old copy.** Pass your in-memory settings to `Appearance::update`; they take the change too.
- **Using `Scope::Family` to make another application follow.** It rewrites `quvyta.conf` with the value you pass, so a settings table that lists every member would change the family's theme from one member's row. Use `follow` there.
- **Writing the user's real files in tests.** Use `preferences_in`, `set_in` and `Appearance::in_folder` with a temporary folder.
