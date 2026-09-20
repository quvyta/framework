//! Resolution order, scopes and file handling of the family's shared preferences.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::storage::dirs::config_root;

/// A folder of its own for each test, empty at the start.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quvyta-preferences-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("test folder");
    dir
}

/// A machine that speaks Turkish in a terminal known for Nerd Fonts.
fn turkish_kitty(name: &str) -> Option<String> {
    match name {
        "LANG" => Some("tr_TR.UTF-8".to_owned()),
        "TERM_PROGRAM" => Some("kitty".to_owned()),
        _ => None,
    }
}

fn resolve(dir: &Path, app: &str) -> Preferences {
    Family::QUVYTA.preferences_detecting(dir, app, &I18n::builtin(), turkish_kitty)
}

fn write(dir: &Path, file: &str, text: &str) {
    fs::write(dir.join(file), text).expect("write test file");
}

fn read(dir: &Path, file: &str) -> String {
    fs::read_to_string(dir.join(file)).expect("read test file")
}

fn text(value: &str, source: Source) -> Resolved<String> {
    Resolved { value: value.to_owned(), source }
}

#[test]
fn the_applications_own_value_wins() {
    let dir = folder("own");
    write(&dir, "quvyta.conf", "language = \"de\"\ntheme = \"monochrome\"\nicons = \"ascii\"\n");
    write(&dir, "code.conf", "language = \"fr\"\ntheme = \"nordic\"\nicons = \"unicode\"\n");
    let prefs = resolve(&dir, "code");
    assert_eq!(prefs.language(), &text("fr", Source::App));
    assert_eq!(prefs.theme(), &text("nordic", Source::App));
    assert_eq!(prefs.icons(), &Resolved { value: IconMode::Unicode, source: Source::App });
    assert!(prefs.diagnostics().is_empty(), "{:?}", prefs.diagnostics());
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_family_id_and_a_missing_key_both_follow_the_shared_file() {
    let dir = folder("follow");
    write(&dir, "quvyta.conf", "language = \"de\"\ntheme = \"amber\"\nicons = \"ascii\"\n");
    write(&dir, "code.conf", "language = \"quvyta\"\ntheme = \"quvyta\"\n");
    let prefs = resolve(&dir, "code");
    assert_eq!(prefs.language(), &text("de", Source::Family));
    assert_eq!(prefs.theme(), &text("amber", Source::Family));
    assert_eq!(prefs.icons(), &Resolved { value: IconMode::Ascii, source: Source::Family }, "absent counts as quvyta");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_key_neither_file_holds_is_detected() {
    let dir = folder("detected");
    write(&dir, "quvyta.conf", "theme = \"amber\"\n");
    write(&dir, "code.conf", "language = \"quvyta\"\n");
    let prefs = resolve(&dir, "code");
    assert_eq!(prefs.language(), &text("tr", Source::Detected));
    assert_eq!(prefs.theme(), &text("amber", Source::Family));
    assert_eq!(prefs.icons(), &Resolved { value: IconMode::Nerd, source: Source::Detected });
    assert_eq!(read(&dir, "quvyta.conf"), "theme = \"amber\"\n", "an existing shared file is not filled in");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_missing_shared_file_is_created_with_the_detected_values() {
    let dir = folder("created").join("quvyta");
    let prefs = resolve(&dir, "code");
    assert_eq!(read(&dir, "quvyta.conf"), "language = \"tr\"\ntheme = \"monochrome\"\nicons = \"nerd\"\n");
    assert_eq!(prefs.language(), &text("tr", Source::Detected));
    assert_eq!(prefs.theme(), &text("monochrome", Source::Detected));
    assert!(!dir.join("code.conf").exists(), "the application's own file is not written");
    assert_eq!(resolve(&dir, "code").language(), &text("tr", Source::Family), "the next start reads it");
    fs::remove_dir_all(dir.parent().expect("parent")).expect("clean");
}

#[test]
fn a_broken_line_falls_back_to_the_detected_value_with_a_located_diagnostic() {
    let dir = folder("broken");
    write(&dir, "quvyta.conf", "language = \"de\"\ntheme = \nicons = \"sparkly\"\n");
    let prefs = resolve(&dir, "code");
    assert_eq!(prefs.language(), &text("de", Source::Family), "the good line is kept");
    assert_eq!(prefs.theme(), &text("monochrome", Source::Detected));
    assert_eq!(prefs.icons(), &Resolved { value: IconMode::Nerd, source: Source::Detected });
    let located: Vec<String> = prefs.diagnostics().iter().map(ToString::to_string).collect();
    assert!(located.iter().any(|d| d.starts_with("quvyta.conf:2:")), "{located:?}");
    assert!(located.iter().any(|d| d.starts_with("quvyta.conf:3:1:") && d.contains("icons")), "{located:?}");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_shared_file_cannot_follow_itself() {
    let dir = folder("itself");
    write(&dir, "quvyta.conf", "theme = \"quvyta\"\n");
    let prefs = resolve(&dir, "code");
    assert_eq!(prefs.theme(), &text("monochrome", Source::Detected));
    assert_eq!(
        prefs.diagnostics()[0].to_string(),
        "quvyta.conf:1:1: warning: `theme` cannot follow the family in the family's own file; the detected value is used"
    );
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn scope_family_writes_the_shared_file_and_makes_the_app_follow() {
    let dir = folder("scope-family");
    write(&dir, "quvyta.conf", "# written by hand\nlanguage = \"tr\"\ntheme = \"monochrome\"\nicons = \"nerd\"\n");
    write(&dir, "code.conf", "theme = \"amber\"\nengine = \"podman\"\n");
    write(&dir, "focus.conf", "theme = \"quvyta\"\n");
    write(&dir, "tools.conf", "theme = \"iris\"\n");
    Family::QUVYTA.set_in(&dir, "code", Shared::Theme, "nordic", Scope::Family).expect("set");
    assert_eq!(read(&dir, "quvyta.conf"), "language = \"tr\"\ntheme = \"nordic\"\nicons = \"nerd\"\n");
    assert_eq!(read(&dir, "code.conf"), "theme = \"quvyta\"\nengine = \"podman\"\n", "other keys stay");
    assert_eq!(resolve(&dir, "code").theme(), &text("nordic", Source::Family));
    assert_eq!(resolve(&dir, "focus").theme(), &text("nordic", Source::Family), "a follower changes too");
    assert_eq!(resolve(&dir, "tools").theme(), &text("iris", Source::App), "an own value stays");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn scope_app_writes_only_the_applications_file() {
    let dir = folder("scope-app");
    let shared = "language = \"tr\"\ntheme = \"monochrome\"\nicons = \"nerd\"\n";
    write(&dir, "quvyta.conf", shared);
    Family::QUVYTA.set_in(&dir, "code", Shared::Theme, "nordic", Scope::App).expect("set");
    assert_eq!(read(&dir, "quvyta.conf"), shared, "unchanged");
    assert_eq!(read(&dir, "code.conf"), "theme = \"nordic\"\n");
    assert_eq!(resolve(&dir, "code").theme(), &text("nordic", Source::App));
    assert_eq!(resolve(&dir, "focus").theme(), &text("monochrome", Source::Family));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn follow_puts_the_application_back_on_the_family_without_touching_the_shared_file() {
    let dir = folder("follow-back");
    let shared = "# written by hand\nlanguage = \"tr\"\ntheme = \"nordic\"\nicons = \"nerd\"\n";
    write(&dir, "quvyta.conf", shared);
    write(&dir, "code.conf", "theme = \"amber\"\nengine = \"podman\"\n");
    Family::QUVYTA.follow_in(&dir, "code", Shared::Theme).expect("follow");
    assert_eq!(read(&dir, "quvyta.conf"), shared, "the shared file is neither read nor written");
    assert_eq!(read(&dir, "code.conf"), "theme = \"quvyta\"\nengine = \"podman\"\n", "other keys stay");
    assert_eq!(resolve(&dir, "code").theme(), &text("nordic", Source::Family));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn follow_creates_a_missing_file_with_only_that_key() {
    let dir = folder("follow-missing").join("quvyta");
    Family::QUVYTA.follow_in(&dir, "tools", Shared::Language).expect("follow");
    assert_eq!(read(&dir, "tools.conf"), "language = \"quvyta\"\n");
    assert!(!dir.join("quvyta.conf").exists(), "the shared file is not created either");
    fs::remove_dir_all(dir.parent().expect("parent")).expect("clean");
}

#[test]
fn following_twice_changes_nothing() {
    let dir = folder("follow-twice");
    write(&dir, "code.conf", "icons = \"ascii\"\n");
    let family = Family::QUVYTA;
    family.follow_in(&dir, "code", Shared::Icons).expect("first");
    let after_first = read(&dir, "code.conf");
    family.follow_in(&dir, "code", Shared::Icons).expect("second, a key that already follows");
    assert_eq!(read(&dir, "code.conf"), after_first);
    assert_eq!(after_first, "icons = \"quvyta\"\n");
    assert!(!dir.join("code.conf.bak").exists(), "nothing was rewritten");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn follow_refuses_a_broken_file_and_leaves_it_as_it_was() {
    let dir = folder("follow-broken");
    let broken = "theme = \nengine = \"podman\"\n";
    write(&dir, "code.conf", broken);
    let error = Family::QUVYTA.follow_in(&dir, "code", Shared::Theme).expect_err("broken");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("code.conf:1:"), "{error}");
    assert_eq!(read(&dir, "code.conf"), broken, "untouched");
    assert!(!dir.join("code.conf.bak").exists(), "no backup either");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn two_writers_keep_both_changes() {
    let dir = folder("two-writers");
    let first = resolve(&dir, "code");
    let second = resolve(&dir, "focus");
    Family::QUVYTA.set_in(&dir, "code", Shared::Theme, "nordic", Scope::Family).expect("code writes");
    Family::QUVYTA.set_in(&dir, "focus", Shared::Language, "de", Scope::Family).expect("focus writes");
    Family::QUVYTA.set_in(&dir, "focus", Shared::Icons, "ascii", Scope::App).expect("focus writes its own");
    assert_eq!(first.theme().value, "monochrome", "what was resolved before stays as it was");
    assert_eq!(second.language().value, "tr");
    assert_eq!(read(&dir, "quvyta.conf"), "language = \"de\"\ntheme = \"nordic\"\nicons = \"nerd\"\n");
    let code = resolve(&dir, "code");
    assert_eq!((code.theme().value.as_str(), code.language().value.as_str()), ("nordic", "de"));
    let focus = resolve(&dir, "focus");
    assert_eq!(focus.icons(), &Resolved { value: IconMode::Ascii, source: Source::App });
    assert_eq!(focus.theme(), &text("nordic", Source::Family));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn writers_on_threads_keep_every_change() {
    let dir = folder("threads");
    write(&dir, "quvyta.conf", "language = \"tr\"\n");
    let handles: Vec<_> = ["code", "focus", "tools"]
        .into_iter()
        .zip([Shared::Theme, Shared::Icons, Shared::Language])
        .zip(["nordic", "unicode", "de"])
        .map(|((app, key), value)| {
            let dir = dir.clone();
            std::thread::spawn(move || Family::QUVYTA.set_in(&dir, app, key, value, Scope::App))
        })
        .collect();
    for handle in handles {
        handle.join().expect("thread").expect("set");
    }
    assert_eq!(resolve(&dir, "code").theme(), &text("nordic", Source::App));
    assert_eq!(resolve(&dir, "focus").icons(), &Resolved { value: IconMode::Unicode, source: Source::App });
    assert_eq!(resolve(&dir, "tools").language(), &text("de", Source::App));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn family_writers_in_the_same_instant_keep_every_change() {
    let dir = folder("same-instant");
    let keys =
        [(Shared::Theme, ["amber", "nordic"]), (Shared::Icons, ["ascii", "unicode"]), (Shared::Language, ["fr", "de"])];
    let handles: Vec<_> = keys
        .into_iter()
        .enumerate()
        .map(|(index, (key, values))| {
            let dir = dir.clone();
            std::thread::spawn(move || {
                for round in 0..40 {
                    let app = ["code", "focus", "tools"][index];
                    let value = values[usize::from(round == 39)];
                    Family::QUVYTA.set_in(&dir, app, key, value, Scope::Family).expect("set");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("thread");
    }
    assert_eq!(read(&dir, "quvyta.conf").lines().count(), 3);
    let prefs = resolve(&dir, "code");
    assert_eq!(prefs.theme().value, "nordic");
    assert_eq!(prefs.icons().value, IconMode::Unicode);
    assert_eq!(prefs.language().value, "de");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn invalid_values_are_refused_without_writing() {
    let dir = folder("invalid");
    let family = Family::QUVYTA;
    for (key, value) in [(Shared::Icons, "sparkly"), (Shared::Theme, "quvyta"), (Shared::Language, " ")] {
        let error = family.set_in(&dir, "code", key, value, Scope::Family).expect_err(value);
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
    assert!(fs::read_dir(&dir).expect("list").next().is_none(), "nothing written");
    family.set_in(&dir, "code", Shared::Icons, "NERD", Scope::App).expect("any case");
    assert_eq!(read(&dir, "code.conf"), "icons = \"nerd\"\n");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_follow_value_survives_the_applications_self_healing() {
    use crate::storage::{Schema, SettingKind};
    let dir = folder("healing");
    write(&dir, "code.conf", "language = \"quvyta\"\ntheme = \"quvyta\"\nicons = \"quvyta\"\n");
    let schema = Schema::builtin().optional(Settings::LANGUAGE, SettingKind::choice(["en", "tr"]));
    let settings = Settings::open(dir.join("code.conf")).member_of(&Family::QUVYTA).schema(schema).self_heal(true);
    assert!(settings.diagnostics().is_empty(), "{:?}", settings.diagnostics());
    assert_eq!(read(&dir, "code.conf"), "language = \"quvyta\"\ntheme = \"quvyta\"\nicons = \"quvyta\"\n");
    assert_eq!((settings.theme(), settings.language(), settings.icon_mode()), (None, None, None));
    let command: Command<()> = settings.apply();
    assert!(command.actions.is_empty(), "nothing to switch: the preferences decide");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_files_sit_in_the_xdg_config_folder_on_linux() {
    if !cfg!(all(unix, not(target_os = "macos"))) {
        return;
    }
    let root = folder("xdg");
    let xdg = root.clone();
    let lookup = move |name: &str| match name {
        "XDG_CONFIG_HOME" => Some(xdg.clone()),
        "HOME" => Some(PathBuf::from("/home/ada")),
        _ => None,
    };
    let family_dir = config_root(lookup).map(|base| base.join(Family::QUVYTA.id())).expect("a folder");
    assert_eq!(family_dir, root.join("quvyta"));
    resolve(&family_dir, "code");
    Family::QUVYTA.set_in(&family_dir, "code", Shared::Theme, "nordic", Scope::App).expect("set");
    assert!(root.join("quvyta").join("quvyta.conf").is_file());
    assert_eq!(read(&root.join("quvyta"), "code.conf"), "theme = \"nordic\"\n");
    fs::remove_dir_all(&root).expect("clean");
}

#[test]
fn apply_switches_the_three_keys() {
    let dir = folder("apply");
    write(&dir, "quvyta.conf", "language = \"de\"\ntheme = \"amber\"\nicons = \"ascii\"\n");
    let command: Command<()> = resolve(&dir, "code").apply();
    assert_eq!(command.actions.len(), 3);
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_runtime_starts_with_the_resolved_values() {
    let dir = folder("runtime");
    write(&dir, "quvyta.conf", "language = \"de\"\ntheme = \"amber\"\nicons = \"ascii\"\n");
    write(&dir, "code.conf", "theme = \"quvyta\"\nicons = \"quvyta\"\nreduced-motion = true\n");
    let settings = Settings::open(dir.join("code.conf")).member_of(&Family::QUVYTA);
    let mut env = crate::env::Env::builtin();
    env.apply_settings(&settings);
    assert!(env.diagnostics().is_empty(), "the family's id is not taken for a theme: {:?}", env.diagnostics());
    env.apply_preferences(&resolve(&dir, "code"));
    assert_eq!(env.theme().id(), "amber");
    assert_eq!(env.i18n().active(), "de");
    assert_eq!(env.icon_mode(), IconMode::Ascii);
    assert!(env.reduced_motion(), "the rest of the settings stay");
    fs::remove_dir_all(&dir).expect("clean");
}
