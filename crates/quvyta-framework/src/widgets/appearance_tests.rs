//! The appearance rows: texts in several languages, changes applied at once and saved where the
//! box under each shared row says, and the update notice's switch written in the background.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::color::Rgb;
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

/// An application whose update notice is written in the background, as one with a settings folder
/// on a slow disk asks for: it writes down every outcome it hears.
struct Noticing {
    settings: Settings,
    appearance: Appearance,
    log: Vec<String>,
}

#[derive(Debug, Clone)]
enum Notice {
    Appearance(AppearanceChange),
    Saved(AppearanceSave),
    Preferences(Preferences),
}

impl App for Noticing {
    type Msg = Notice;

    fn update(&mut self, msg: Notice) -> Command<Notice> {
        match msg {
            Notice::Appearance(change) => self.appearance.update_saving(change, &mut self.settings, Notice::Saved),
            Notice::Saved(save) => {
                self.log.push(match &save {
                    AppearanceSave::Saved => "saved".to_owned(),
                    AppearanceSave::Failed(reason) => format!("failed: {reason}"),
                });
                self.appearance.saved(&save);
                Command::none()
            }
            Notice::Preferences(preferences) => {
                self.appearance.refresh(preferences);
                Command::none()
            }
        }
    }

    fn preferences(&self, preferences: &Preferences) -> Option<Notice> {
        Some(Notice::Preferences(preferences.clone()))
    }

    fn view(&self, ui: &mut View<'_, Notice>) {
        SettingsList::show(ui, |list| {
            self.appearance.rows(list, Notice::Appearance);
            self.appearance.updates(list, Notice::Appearance);
        })
        .id("appearance");
    }
}

/// `code` in `dir` with its update notice written in the background, drawn 70 × 24 cells.
fn noticing_in(dir: &Path) -> Harness<Noticing> {
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(dir).updates_in_background();
    let settings = Settings::open(dir.join("code.conf")).member_of(&ecosystem);
    let app = Noticing { settings, appearance, log: Vec::new() };
    Harness::with_env(app, Env::builtin(), 70, 24)
}

/// The row `y` of the update notice switch, and whether it is on: the knob of a switch is its
/// brightest part and is two cells wide, so the lit end of the five cells it is drawn in says
/// where the person left it.
fn switch_on(app: &Harness<Noticing>, y: i32) -> bool {
    let left = app.buffer().area.width - 7;
    let row = u16::try_from(y).expect("a row on the screen");
    let lit = |cell: u16| app.bg(left + cell, row).map_or(0.0, Rgb::relative_luminance);
    lit(4) > lit(0)
}

/// The update notice's row, and whether its switch is on.
fn notice_row(app: &Harness<Noticing>) -> (i32, bool) {
    let (_, y) =
        app.find("Say when an update is out").unwrap_or_else(|| panic!("the row is on the screen:\n{}", app.screen()));
    (y, switch_on(app, y))
}

fn read(dir: &Path, file: &str) -> String {
    fs::read_to_string(dir.join(file)).unwrap_or_default()
}

/// The rows of `code` in `dir`, drawn in `env`, 70 × 24 cells.
fn code_in(dir: &Path, env: Env) -> Harness<Code> {
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&ecosystem);
    Harness::with_env(Code { settings, appearance }, env, 70, 24)
}

/// Where `text` is first drawn below row `after`.
fn below(app: &Harness<Code>, text: &str, after: i32) -> Option<(i32, i32)> {
    let screen = app.screen();
    screen.lines().enumerate().skip(usize::try_from(after).ok()? + 1).find_map(|(y, line)| {
        let byte = line.find(text)?;
        Some((i32::try_from(line[..byte].chars().count()).ok()?, i32::try_from(y).ok()?))
    })
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
        assert_eq!(screen.matches(texts[2]).count(), 4, "one box under each shared row");
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
    assert_eq!(read(&dir, "code.conf"), "icons = \"quvyta\"\n", "code follows the ecosystem");
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
    assert!(!read(&dir, "quvyta.conf").contains("reduced-motion"), "{}", read(&dir, "quvyta.conf"));
    // The box under the row is kept as well: a click on it changes nothing.
    let (_, row) = app.find("Reduce motion").expect("the row");
    let (x, y) = below(&app, "In every Quvyta application", row).expect("the box under reduced motion");
    for x in x..70 {
        app.click(x, y);
    }
    assert_eq!(app.app().appearance.preferences().source(Shared::ReducedMotion), Source::Detected);
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_box_decides_which_file_a_change_goes_to() {
    let dir = folder("scope");
    shared_file(&dir);
    let mut app = code_in(&dir, Env::builtin());
    // The box under the theme: checked while code follows the ecosystem. Clear it with the keys.
    app.click_text("Theme");
    app.press("down").press("space");
    assert_eq!(app.app().appearance.preferences().theme().source, Source::App);
    app.send(Msg::Appearance(AppearanceChange::Theme("nordic".to_owned())));
    assert_eq!(app.env().theme().id(), "nordic", "applied at once");
    assert_eq!(read(&dir, "code.conf"), "theme = \"nordic\"\n");
    assert!(read(&dir, "quvyta.conf").contains("theme = \"monochrome\""), "the ecosystem keeps its theme");
    assert_eq!(app.app().settings.theme().as_deref(), Some("nordic"), "the settings in memory agree");

    app.send(Msg::Appearance(AppearanceChange::Everywhere(Shared::Theme, true)));
    assert!(read(&dir, "quvyta.conf").contains("theme = \"nordic\""), "{}", read(&dir, "quvyta.conf"));
    assert_eq!(read(&dir, "code.conf"), "theme = \"quvyta\"\n");
    assert_eq!(app.app().settings.theme(), None, "follows the ecosystem again");
    let focus = Ecosystem::QUVYTA.preferences_in(&dir, "focus", &I18n::builtin());
    assert_eq!(focus.theme().value, "nordic", "another application follows");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_pillar_goes_to_the_applications_own_file() {
    let dir = folder("own");
    shared_file(&dir);
    fs::write(dir.join("code.conf"), "engine = \"podman\"\n").expect("code.conf");
    let mut app = code_in(&dir, Env::builtin());
    app.send(Msg::Appearance(AppearanceChange::Pillar(PillarStyle::Thin)));
    assert_eq!(app.env().pillar_style(), Some(PillarStyle::Thin));
    assert_eq!(read(&dir, "code.conf"), "engine = \"podman\"\npillar = \"thin\"\n");
    assert!(!read(&dir, "quvyta.conf").contains("pillar"));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn reduced_motion_is_shared_unless_its_box_is_cleared() {
    let dir = folder("motion");
    shared_file(&dir);
    fs::write(dir.join("code.conf"), "engine = \"podman\"\n").expect("code.conf");
    let mut app = code_in(&dir, Env::builtin());
    // The switch, as a person turns it: its row picked with a click, then space.
    app.click_text("Reduce motion");
    app.press("space");
    assert!(app.env().reduced_motion(), "applied at once\n{}", app.screen());
    assert!(read(&dir, "quvyta.conf").contains("reduced-motion = true"), "{}", read(&dir, "quvyta.conf"));
    assert_eq!(read(&dir, "code.conf"), "engine = \"podman\"\nreduced-motion = \"quvyta\"\n");
    let focus = Ecosystem::QUVYTA.preferences_in(&dir, "focus", &I18n::builtin());
    assert!(focus.reduced_motion().value, "another application reduces motion too");

    // The box under it, cleared: the need stays, now in code's own file.
    app.press("down").press("space");
    assert_eq!(app.app().appearance.preferences().source(Shared::ReducedMotion), Source::App);
    assert_eq!(read(&dir, "code.conf"), "engine = \"podman\"\nreduced-motion = true\n", "a boolean, not text");
    app.press("up").press("space");
    assert!(!app.env().reduced_motion());
    assert!(read(&dir, "code.conf").contains("reduced-motion = false"));
    assert!(read(&dir, "quvyta.conf").contains("reduced-motion = true"), "the ecosystem keeps its own");
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_change_that_cannot_be_saved_is_applied_and_says_why() {
    let dir = folder("unsaved");
    let blocked = dir.join("not-a-folder");
    fs::write(&blocked, "a file where the ecosystem's folder should be").expect("blocker");
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&blocked);
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
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&ecosystem);
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
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&ecosystem);
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
fn the_update_notice_is_one_switch_for_the_ecosystem_kept_in_the_shared_file() {
    let dir = folder("updates");
    shared_file(&dir);
    fs::write(dir.join("code.conf"), "engine = \"podman\"\n").expect("code.conf");
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&dir);
    let settings = Settings::open(dir.join("code.conf")).member_of(&ecosystem);
    assert!(
        !Harness::new(Code { settings: settings.clone(), appearance: appearance.clone() }, 70, 40)
            .screen()
            .contains("Say when an update is out"),
        "not in the section itself: an application that never asks has no use for it"
    );
    let mut app = Harness::new(Asking(Code { settings, appearance }), 70, 40);
    let screen = app.screen();
    assert!(screen.contains("Say when an update is out"), "{screen}");
    assert!(screen.contains("name and version"), "it says what is sent and nothing more\n{screen}");
    assert!(ecosystem.update_notice_in(&dir), "on until someone turns it off");
    // The switch is a tone at the row's right edge; a person reaches it by the row and Space.
    app.click_text("Say when an update is out").press("space");
    assert!(!ecosystem.update_notice_in(&dir), "the click turned it off for the ecosystem\n{}", app.screen());
    assert!(read(&dir, "quvyta.conf").contains("update-notice = false"), "{}", read(&dir, "quvyta.conf"));
    assert_eq!(read(&dir, "code.conf"), "engine = \"podman\"\n", "not the application's own choice");
    app.press("space");
    assert!(ecosystem.update_notice_in(&dir));
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn the_names_of_the_rows_and_choices_are_the_words_the_box_shows() {
    let mut i18n = I18n::builtin();
    let english: Vec<String> = Shared::ALL.iter().map(|key| Appearance::label(&i18n, *key)).collect();
    assert_eq!(english, ["Language", "Theme", "Icons", "Reduce motion"]);
    assert_eq!(IconMode::Auto.label(&i18n), "Detect");
    assert_eq!(IconMode::Nerd.label(&i18n), "Nerd Font");
    assert_eq!(PillarStyle::Thin.label(&i18n), "Thin");
    assert!(i18n.set_active("tr"));
    assert_eq!(Appearance::label(&i18n, Shared::Language), "Dil");
    assert_eq!(Appearance::label(&i18n, Shared::ReducedMotion), "Hareketi azalt");
    assert_eq!(IconMode::Auto.label(&i18n), "Algıla");
    assert_eq!(PillarStyle::Thin.label(&i18n), "İnce");
    for mode in IconMode::ALL {
        assert!(!mode.label(&i18n).starts_with('⟦'), "{mode:?}");
    }
}

#[test]
fn a_switch_whose_file_is_written_in_the_background_keeps_the_value_it_took() {
    let dir = folder("background-saved");
    shared_file(&dir);
    let mut app = noticing_in(&dir);
    app.set_reduced_motion(true);
    let (y, on) = notice_row(&app);
    assert!(on, "on until someone turns it off");
    // The switch is a tone at the row's right edge; a person reaches it by the row and Space.
    app.click_text("Say when an update is out").press("space");
    assert_eq!(app.app().log, ["saved"], "the write went through, and the application heard it");
    assert!(read(&dir, "quvyta.conf").contains("update-notice = false"), "{}", read(&dir, "quvyta.conf"));
    assert!(!Ecosystem::QUVYTA.update_notice_in(&dir), "every application that follows the ecosystem stops asking");
    assert!(!app.app().appearance.preferences().update_notice(), "the rows hold the value the file took");
    let (row, on) = notice_row(&app);
    assert_eq!(row, y, "the row stays where it was");
    assert!(!on, "the switch keeps the value the file now has\n{}", app.screen());
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn a_switch_whose_file_cannot_be_written_goes_back_and_the_application_hears_one_reason() {
    let dir = folder("background-failed");
    let blocked = dir.join("not-a-folder");
    fs::write(&blocked, "a file where the ecosystem's folder should be").expect("blocker");
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&blocked).updates_in_background();
    let state = Noticing { settings: Settings::in_memory(), appearance, log: Vec::new() };
    let mut app = Harness::with_env(state, Env::builtin(), 70, 24);
    app.set_reduced_motion(true);
    let (y, on) = notice_row(&app);
    assert!(on, "a file nobody can write is the one nobody turned it off in");
    app.click_text("Say when an update is out").press("space");
    assert_eq!(app.app().log.len(), 1, "one outcome, not one per frame: {:?}", app.app().log);
    let reason = app.app().log[0].strip_prefix("failed: ").expect("the outcome is a failure").to_owned();
    assert!(!reason.is_empty(), "it says why: {:?}", app.app().log);
    let (row, on) = notice_row(&app);
    assert_eq!(row, y, "the row stays where it was");
    assert!(on, "the switch is back where the person left it\n{}", app.screen());
    // The row itself says nothing: the application has the reason and shows it as a toast.
    assert!(!app.screen().contains("Applied, but not saved"), "{}", app.screen());
    fs::remove_dir_all(&dir).expect("clean");
}

#[test]
fn another_member_turning_the_notice_off_shows_on_the_row() {
    let dir = folder("background-follow");
    shared_file(&dir);
    let ecosystem = Ecosystem::QUVYTA;
    let preferences = ecosystem.preferences_in(&dir, "code", &I18n::builtin());
    let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&dir).updates_in_background();
    let settings = Settings::open(dir.join("code.conf")).member_of(&ecosystem);
    let state = Noticing { settings, appearance, log: Vec::new() };
    let mut app = Harness::member_in(state, ecosystem, &dir, "code", 70, 24);
    app.set_reduced_motion(true);
    let (y, on) = notice_row(&app);
    assert!(on, "on until someone turns it off");
    // Another application of the ecosystem turns it off in the shared file.
    ecosystem.set_update_notice_in(&dir, false).expect("focus wrote it");
    app.poll_preferences();
    let (row, on) = notice_row(&app);
    assert_eq!(row, y, "the row stays where it was");
    assert!(!on, "the row shows what the ecosystem says now\n{}", app.screen());
    assert!(app.app().log.is_empty(), "nobody in this application asked for it: {:?}", app.app().log);
    fs::remove_dir_all(&dir).expect("clean");
}
