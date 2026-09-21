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

/// An application that asks for its updates: the notice's row right after the section.
struct Asking(Code);

impl App for Asking {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        self.0.update(msg)
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        SettingsList::show(ui, |list| {
            self.0.appearance.section(list, Msg::Appearance);
            self.0.appearance.updates(list, Msg::Appearance);
        });
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

#[test]
fn the_longest_language_name_is_shown_whole_and_the_three_choices_keep_one_width() {
    let dir = folder("choice-width");
    shared_file(&dir);
    let family = Family::QUVYTA;
    let preferences = family.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(family, "code", preferences).in_folder(&dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&family);
    let mut app = Harness::with_env(Code { settings, appearance }, Env::builtin(), 100, 24);
    app.set_locale("pt-BR");
    let screen = app.screen();
    // The name of the language a person chose is the one thing on this row they must be able to
    // read: cut after `Português`, the two Portugueses cannot be told apart.
    assert!(screen.contains("Português (Brasil)"), "the longest language name stands whole:\n{screen}");
    assert!(!screen.contains("Português …"), "and is not cut although the row has room:\n{screen}");
    // One column, not three: every choice starts in the same cell.
    let starts: Vec<usize> = ["Português (Brasil)", "Monochrome", "Unicode"]
        .iter()
        .map(|name| app.find(name).unwrap_or_else(|| panic!("{name} is on the screen:\n{screen}")).0 as usize)
        .collect();
    assert_eq!(starts[0], starts[1], "language and theme start in the same cell:\n{screen}");
    assert_eq!(starts[1], starts[2], "and so does the icon set:\n{screen}");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_narrow_row_keeps_its_label_and_its_choice() {
    let dir = folder("choice-narrow");
    shared_file(&dir);
    let family = Family::QUVYTA;
    let preferences = family.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(family, "code", preferences).in_folder(&dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&family);
    let mut app = Harness::with_env(Code { settings, appearance }, Env::builtin(), 48, 24);
    app.set_locale("pt-BR");
    let screen = app.screen();
    assert!(screen.contains("Idioma"), "the label stays:\n{screen}");
    assert!(screen.contains('▾'), "and the choice is still a choice:\n{screen}");
    for line in screen.lines() {
        assert!(crate::text::width(line) <= 48, "nothing is drawn past the edge: {line:?}");
    }
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_choice_is_as_wide_as_its_longest_name_between_a_floor_and_a_cap() {
    assert_eq!(choice_width(&["Nerd Font".to_owned()], 2), CHOICE_MIN, "short names keep the column's floor");
    assert_eq!(choice_width(&["Português (Brasil)".to_owned()], 2), 25, "the name, the chevron and the ground");
    assert_eq!(choice_width(&["Português (Brasil)".to_owned()], 0), 21, "a theme with no ground needs less");
    assert_eq!(choice_width(&["x".repeat(60)], 2), CHOICE_MAX, "a name of an application's own cannot take the row");
}

#[test]
fn the_update_notice_is_one_switch_for_the_family_kept_in_the_shared_file() {
    let dir = folder("updates");
    shared_file(&dir);
    fs::write(dir.join("code.conf"), "engine = \"podman\"\n").expect("code.conf");
    let family = Family::QUVYTA;
    let preferences = family.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(family, "code", preferences).in_folder(&dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&family);
    assert!(
        !Harness::new(Code { settings: settings.clone(), appearance: appearance.clone() }, 70, 40)
            .screen()
            .contains("Say when an update is out"),
        "not in the section itself: an application that never asks has no use for it"
    );
    let mut app = Harness::new(Asking(Code { settings, appearance }), 70, 40);
    let screen = app.screen();
    assert!(screen.contains("Say when an update is out"), "{screen}");
    assert!(screen.contains("only its name is sent"), "it says what is never sent\n{screen}");
    assert!(family.update_notice_in(&dir), "on until someone turns it off");
    // The switch is a tone at the row's right edge; a person reaches it by the row and Space.
    app.click_text("Say when an update is out").press("space");
    assert!(!family.update_notice_in(&dir), "the click turned it off for the family\n{}", app.screen());
    assert!(read(&dir, "quvyta.conf").contains("update-notice = false"), "{}", read(&dir, "quvyta.conf"));
    assert_eq!(read(&dir, "code.conf"), "engine = \"podman\"\n", "not the application's own choice");
    app.press("space");
    assert!(family.update_notice_in(&dir));
    fs::remove_dir_all(&dir).expect("clean");
}
