## When to use

Every application of the Quvyta ecosystem speaks the same language, draws with the same theme and the same icons unless the user chose otherwise for one of them. Use `Ecosystem::preferences` at start to resolve those three, and put the ready-made `Appearance` rows on your settings page instead of building language, theme, icon, reduced motion and pillar rows yourself.

## Step by step

1. At start, after `Ecosystem::adopt` and `Settings::load_member`, resolve the shared preferences: `let prefs = Ecosystem::QUVYTA.preferences("code", &i18n);`. The first application that starts creates `quvyta.conf` with the values detected on the machine.
2. Start the runtime with them, after your own settings: `Runtime::new(app).settings(&settings).preferences(&prefs)`.
3. Keep an `Appearance` in your state: `Appearance::new(Ecosystem::QUVYTA, "code", prefs)`.
4. Place it in a settings list: `SettingsList::show(ui, |list| self.appearance.section(list, Msg::Appearance))`. A setup wizard's first step uses `rows` instead, without the heading.
5. Hand every change back: `Msg::Appearance(change) => self.appearance.update(change, &mut self.settings)`. It saves the change and returns the command that shows it at once.

## How it works

- **Each key on its own.** For language, theme and icons: the application's own value when its file names one, else `quvyta.conf`, else what the machine suggests. A missing key and the value `"quvyta"` both mean "follow the ecosystem".
- **Back to the ecosystem, on its own.** `Ecosystem::follow(app, key)` writes the ecosystem's id in that application's file and nothing else: the shared file is neither read nor written, so one member goes back to following without changing what the whole ecosystem draws with. `Ecosystem::set(.., Scope::Ecosystem)` also writes the shared value, which is what you want on the application's own settings page and not what you want on a row for another member.
- **The box under a shared row is the scope.** Checked, a change goes to `quvyta.conf` and the application's file says `"quvyta"`, so every application that follows the ecosystem changes with it. Cleared, the change stays in the application's own file. An application that chose its own value is never changed from another one.
- **Two writers keep both changes.** Each file is read right before it is written and only the changed key is written, with the folder held by an advisory lock on Unix.
- **Nothing stops on a broken line.** That key falls back to the detected value and the reason, with file, line and column, is in `Preferences::diagnostics`.
- **Reduced motion and the pillar are the application's own.** They sit in the same section and are saved in the application's file. When `QUVYTA_REDUCED_MOTION` decides, the row is disabled and says so.
- **Where a value came from.** `Resolved::source` is `App`, `Ecosystem` or `Detected`.


## Saying when an update is out

The ecosystem's update notice is one switch for every application. An application that asks for its updates shows it right after the section, with `self.appearance.updates(list, Msg::Appearance)`; one that never asks leaves it out. To ask, turn on the framework's `updates` feature (`quvyta-framework = { version = "…", features = ["updates"] }`) and ask at start:

```rust
fn init(&mut self) -> Command<Msg> {
    let check = UpdateCheck::new(Ecosystem::QUVYTA, "code", "quvyta-code", env!("CARGO_PKG_VERSION"), Msg::NewVersion);
    Command::check_for_update(check)
}
// in update:
Msg::NewVersion(update) => Command::toast(update.toast()),
```

- **At most once a day, never in the way.** The question runs on a thread of its own; the first frame never waits. When it was last asked is kept in the application's state folder.
- **Silent when it cannot ask.** No network, no answer within ten seconds, or an answer that cannot be read: nothing is shown and the next day asks again.
- **Only the name goes out.** The request names the package; its `User-Agent` is the package and its version. No identity, no machine detail, no use of the application.
- **Off for the whole ecosystem.** With the switch off (`update-notice = false` in `quvyta.conf`) nothing is asked and nothing is written.
- **Only a real newer version.** Yanked versions are passed over and a build newer than the registry says nothing. Versions are ordered as semver orders them: `0.1.0-alpha.9` < `0.1.0-alpha.10` < `0.1.0-beta` < `0.1.0`. A person on a release never hears of a pre-release; a person on a pre-release hears of the newest version after it, the next alpha or the release, so nobody stays on an old alpha.
- **Tests never reach the network.** A harness records the question (`Harness::update_checks`) and answers it with `Harness::set_latest_version(Some("0.2.0"))`, without touching the check's folders.

## Common mistakes

- **Declaring `language` with a fixed list and self-healing.** Call `Settings::member_of(&Ecosystem::QUVYTA)` before `self_heal`, or load with `load_member`, so `"quvyta"` is kept.
- **Saving the whole settings file from an old copy.** Pass your in-memory settings to `Appearance::update`; they take the change too.
- **Using `Scope::Ecosystem` to make another application follow.** It rewrites `quvyta.conf` with the value you pass, so a settings table that lists every member would change the ecosystem's theme from one member's row. Use `follow` there.
- **Writing the user's real files in tests.** Use `preferences_in`, `set_in` and `Appearance::in_folder` with a temporary folder.
