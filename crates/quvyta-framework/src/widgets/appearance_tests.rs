//! The appearance rows: texts in several languages, changes applied at once and saved where the
//! box under each shared row says.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::env::Env;
use crate::i18n::I18n;
use crate::icons::GlyphMode;
use crate::prelude::*;
use crate::widgets::SettingsList;

struct Code {
    settings: Settings,
    appearance: Appearance,
}

#[derive(Debug, Clone)]
enum Msg {
    Appearance(AppearanceChange),
}

impl App for Code {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Appearance(change) => self.appearance.update(change, &mut self.settings),
        }
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        SettingsList::show(ui, |list| self.appearance.section(list, Msg::Appearance)).id("appearance");
    }
}

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quvyta-appearance-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("test folder");
    dir
}

fn read(dir: &Path, file: &str) -> String {
    fs::read_to_string(dir.join(file)).unwrap_or_default()
}

/// The rows of `code` in `dir`, drawn in `env`, 70 × 24 cells.
fn code_in(dir: &Path, env: Env) -> Harness<Code> {
    let family = Family::QUVYTA;
    let preferences = family.preferences_in(dir, "code", &I18n::builtin());
    let appearance = Appearance::new(family, "code", preferences).in_folder(dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&family);
    Harness::with_env(Code { settings, appearance }, env, 70, 24)
}

fn shared_file(dir: &Path) {
    fs::write(dir.join("quvyta.conf"), "language = \"en\"\ntheme = \"monochrome\"\nicons = \"unicode\"\n")
        .expect("shared file");
}

#[test]
fn the_rows_speak_the_active_language() {
    let dir = folder("languages");
    shared_file(&dir);
    for (code, texts) in [
        ("en", ["Appearance", "Language", "In every Quvyta application", "Reduce motion", "Pillar"]),
        ("tr", ["Görünüm", "Dil", "Tüm Quvyta uygulamalarında", "Hareketi azalt", "Vurgu çubuğu"]),
        ("de", ["Darstellung", "Sprache", "In jeder Quvyta-Anwendung", "Bewegung reduzieren", "Akzentbalken"]),
    ] {
        let mut app = code_in(&dir, Env::builtin());
        app.set_locale(code);
        let screen = app.screen();
        for text in texts {
            assert!(screen.contains(text), "{code}: `{text}` missing in\n{screen}");
        }
        assert_eq!(screen.matches(texts[2]).count(), 3, "one box under each shared row");
    }
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn every_language_has_every_appearance_text() {
    let i18n = I18n::builtin();
    for (code, _) in i18n.list() {
        let missing: Vec<String> =
            i18n.missing_keys(&code, "en").into_iter().filter(|key| key.starts_with("quvyta.appearance")).collect();
        assert!(missing.is_empty(), "{code}: {missing:?}");
    }
}

#[test]
fn a_new_icon_mode_redraws_at_once_with_its_glyphs() {
    let dir = folder("icons");
    shared_file(&dir);
    let mut app = code_in(&dir, Env::builtin());
    assert!(app.screen().contains('▾'), "unicode chevrons first\n{}", app.screen());
    app.send(Msg::Appearance(AppearanceChange::Icons(IconMode::Ascii)));
    assert_eq!(app.env().glyph_mode(), GlyphMode::Ascii);
    let screen = app.screen();
    assert!(!screen.contains('▾'), "{screen}");
    assert!(screen.contains("ASCII"), "the row shows the new mode\n{screen}");
    assert_eq!(read(&dir, "quvyta.conf"), "language = \"en\"\ntheme = \"monochrome\"\nicons = \"ascii\"\n");
    assert_eq!(read(&dir, "code.conf"), "icons = \"quvyta\"\n", "code follows the family");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn forced_reduced_motion_is_disabled_says_why_and_does_not_change() {
    let dir = folder("forced");
    shared_file(&dir);
    let mut env = Env::builtin();
    env.force_reduced_motion(Some(false));
    let mut app = code_in(&dir, env);
    assert!(app.screen().contains("Kept by the shell"), "{}", app.screen());
    let (_, y) = app.find("Reduce motion").expect("the row");
    // Every cell on the right of the row, where the switch sits.
    for x in 35..70 {
        app.click(x, y);
    }
    // The keys walk the enabled rows only, so they never land on it either.
    app.click_text("Language");
    for _ in 0..8 {
        app.press("down").press("space");
    }
    assert!(!app.env().reduced_motion(), "still kept");
    assert!(!read(&dir, "code.conf").contains("reduced-motion"), "{}", read(&dir, "code.conf"));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_box_decides_which_file_a_change_goes_to() {
    let dir = folder("scope");
    shared_file(&dir);
    let mut app = code_in(&dir, Env::builtin());
    // The box under the theme: checked while code follows the family. Clear it with the keys.
    app.click_text("Theme");
    app.press("down").press("space");
    assert_eq!(app.app().appearance.preferences().theme().source, Source::App);
    app.send(Msg::Appearance(AppearanceChange::Theme("nordic".to_owned())));
    assert_eq!(app.env().theme().id(), "nordic", "applied at once");
    assert_eq!(read(&dir, "code.conf"), "theme = \"nordic\"\n");
    assert!(read(&dir, "quvyta.conf").contains("theme = \"monochrome\""), "the family keeps its theme");
    assert_eq!(app.app().settings.theme().as_deref(), Some("nordic"), "the settings in memory agree");

    app.send(Msg::Appearance(AppearanceChange::Everywhere(Shared::Theme, true)));
    assert!(read(&dir, "quvyta.conf").contains("theme = \"nordic\""), "{}", read(&dir, "quvyta.conf"));
    assert_eq!(read(&dir, "code.conf"), "theme = \"quvyta\"\n");
    assert_eq!(app.app().settings.theme(), None, "follows the family again");
    let focus = Family::QUVYTA.preferences_in(&dir, "focus", &I18n::builtin());
    assert_eq!(focus.theme().value, "nordic", "another application follows");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_applications_own_rows_go_to_its_own_file() {
    let dir = folder("own");
    shared_file(&dir);
    fs::write(dir.join("code.conf"), "engine = \"podman\"\n").expect("code.conf");
    let mut app = code_in(&dir, Env::builtin());
    app.send(Msg::Appearance(AppearanceChange::ReducedMotion(true)));
    app.send(Msg::Appearance(AppearanceChange::Pillar(PillarStyle::Thin)));
    assert!(app.env().reduced_motion());
    assert_eq!(app.env().pillar_style(), Some(PillarStyle::Thin));
    assert_eq!(read(&dir, "code.conf"), "engine = \"podman\"\nreduced-motion = true\npillar = \"thin\"\n");
    assert!(!read(&dir, "quvyta.conf").contains("pillar"));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_change_that_cannot_be_saved_is_applied_and_says_why() {
    let dir = folder("unsaved");
    let blocked = dir.join("not-a-folder");
    fs::write(&blocked, "a file where the family's folder should be").expect("blocker");
    let family = Family::QUVYTA;
    let preferences = family.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(family, "code", preferences).in_folder(&blocked);
    let mut app = Harness::with_env(Code { settings: Settings::in_memory(), appearance }, Env::builtin(), 70, 24);
    app.send(Msg::Appearance(AppearanceChange::Theme("amber".to_owned())));
    assert_eq!(app.env().theme().id(), "amber");
    assert!(app.screen().contains("Applied, but not saved"), "{}", app.screen());
    app.send(Msg::Appearance(AppearanceChange::Pillar(PillarStyle::Thin)));
    assert_eq!(app.screen().matches("Applied, but not saved").count(), 1, "only under the last change");
    fs::remove_dir_all(&dir).expect("clean");
}
