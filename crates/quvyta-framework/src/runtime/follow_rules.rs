//! Following the ecosystem's preferences while the application runs: another application writes
//! a real file in a real folder, and the harness reads the files again the way the runtime does
//! when its watch hears them change.

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

use crate::env::{AssetDirs, Env};
use crate::icons::{IconMode, PillarStyle};
use crate::runtime::{App, Command, Harness};
use crate::storage::{Ecosystem, Preferences, Scope, Settings, Shared, Source};
use crate::widget::View;
use crate::widgets::{Appearance, AppearanceChange, SettingsList, Text};

const QUVYTA: Ecosystem = Ecosystem::QUVYTA;

/// Writes down every step it hears, oldest first: `init` and `theme <id> <language>` for each
/// preferences the hook hears.
#[derive(Default)]
struct Follower {
    steps: Vec<String>,
    /// How many times the view was built.
    builds: Cell<u32>,
}

enum Msg {
    Heard(Preferences),
}

impl App for Follower {
    type Msg = Msg;

    fn preferences(&self, preferences: &Preferences) -> Option<Msg> {
        Some(Msg::Heard(preferences.clone()))
    }

    fn init(&mut self) -> Command<Msg> {
        self.steps.push("init".to_owned());
        Command::none()
    }

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Heard(preferences) => {
                self.steps.push(format!("theme {} {}", preferences.theme().value, preferences.language().value));
            }
        }
        Command::none()
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        self.builds.set(self.builds.get() + 1);
        ui.add(Text::new(crate::t!("quvyta.appearance.heading")));
    }
}

/// A folder of its own for each test, empty at the start.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("quvyta-follow-rules-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("test folder");
    dir
}

/// The shared file as another application left it.
fn shared(dir: &Path, language: &str, theme: &str) {
    fs::write(
        dir.join("quvyta.conf"),
        format!("language = \"{language}\"\ntheme = \"{theme}\"\nicons = \"unicode\"\n"),
    )
    .expect("shared file");
}

fn follower(dir: &Path) -> Harness<Follower> {
    Harness::member_in(Follower::default(), QUVYTA, dir, "code", 30, 3)
}

#[derive(Default)]
struct Localized {
    clicks: u32,
    actions: u32,
}

#[derive(Clone)]
enum LocalizedMsg {
    Click,
    Action,
}

impl App for Localized {
    type Msg = LocalizedMsg;

    fn update(&mut self, message: LocalizedMsg) -> Command<LocalizedMsg> {
        match message {
            LocalizedMsg::Click => self.clicks += 1,
            LocalizedMsg::Action => self.actions += 1,
        }
        Command::none()
    }

    fn action(&self, name: &str) -> Option<Self::Msg> {
        (name == "open").then_some(LocalizedMsg::Action)
    }

    fn view(&self, ui: &mut View<'_, LocalizedMsg>) {
        let label = ui.env().i18n().translate("member.open", &[]);
        ui.add(crate::widgets::Button::new(label).on_press(LocalizedMsg::Click));
    }
}

fn application_env() -> Env {
    let english = "[meta]\nname = \"English\"\ncode = \"en\"\n[member]\nopen = \"Open notes\"\n";
    let turkish = "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\nfallback = \"en\"\n[member]\nopen = \"Notları aç\"\n";
    let dirs = AssetDirs {
        locale_sources: vec![
            ("app-en.toml".to_owned(), english.to_owned()),
            ("app-tr.toml".to_owned(), turkish.to_owned()),
        ],
        keymap_source: Some(("keymap.toml".to_owned(), "[app]\nopen = \"ctrl+o\"\n".to_owned())),
        ..AssetDirs::default()
    };
    Env::load_with(&dirs, |_| None).expect("the application files load")
}

#[test]
fn a_member_with_the_application_env_keeps_its_locale_and_keymap() {
    let dir = folder("member-env");
    shared(&dir, "en", "nordic");
    let mut app = Harness::member_in_with_env(Localized::default(), application_env(), QUVYTA, &dir, "code", 40, 3);
    assert_eq!(app.env().theme().id(), "nordic", "shared preferences still apply");
    assert!(app.screen().contains("Open notes"), "{}", app.screen());
    assert!(!app.screen().contains("⟦member.open⟧"), "{}", app.screen());
    app.click_text("Open notes");
    assert_eq!(app.app().clicks, 1);
    app.press("ctrl+o");
    assert_eq!(app.app().actions, 1, "the application keymap is still in the environment");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_shared_language_uses_the_application_locale() {
    let dir = folder("member-env-language");
    shared(&dir, "en", "nordic");
    QUVYTA.set_in(&dir, "desk", Shared::Language, "tr", Scope::Ecosystem).expect("saved");
    let mut app = Harness::member_in_with_env(Localized::default(), application_env(), QUVYTA, &dir, "code", 40, 3);
    assert_eq!(app.env().i18n().active(), "tr");
    assert_eq!(app.env().theme().id(), "nordic");
    assert!(app.screen().contains("Notları aç"), "{}", app.screen());
    assert!(!app.screen().contains("⟦member.open⟧"), "{}", app.screen());
    app.click_text("Notları aç");
    assert_eq!(app.app().clicks, 1);
    fs::remove_dir_all(&dir).ok();
}

/// The runtime an application would build in its own `runtime()` function: its locale files and
/// keymap as sources, and membership when `member` is true.
fn notes_runtime(member: bool) -> crate::runtime::Runtime<Localized> {
    let english = "[meta]\nname = \"English\"\ncode = \"en\"\n[member]\nopen = \"Open notes\"\n";
    let turkish = "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\nfallback = \"en\"\n[member]\nopen = \"Notları aç\"\n";
    let runtime = crate::runtime::Runtime::new(Localized::default())
        .locale_source("app-en.toml", english)
        .locale_source("app-tr.toml", turkish)
        .keymap_source("keymap.toml", "[app]\nopen = \"ctrl+o\"\n");
    if member { runtime.member(QUVYTA, "notes") } else { runtime }
}

#[test]
fn an_application_runtime_opens_in_a_harness_the_way_run_starts_it() {
    let dir = folder("runtime-harness");
    shared(&dir, "tr", "nordic");
    let mut app = notes_runtime(true).harness_in(&dir, 40, 3).expect("the runtime starts");
    assert_eq!(app.env().theme().id(), "nordic", "the shared theme applies");
    assert!(
        app.screen().contains("Notları aç"),
        "in the shared language, in the application's own words: {}",
        app.screen()
    );
    app.click_text("Notları aç");
    assert_eq!(app.app().clicks, 1);
    app.press("ctrl+o");
    assert_eq!(app.app().actions, 1, "the application's keymap source is in place");
    shared(&dir, "tr", "amber");
    app.poll_preferences();
    assert_eq!(app.env().theme().id(), "amber", "another member's change is heard");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_application_runtime_in_a_harness_draws_in_true_colour_like_any_harness() {
    let dir = folder("runtime-depth");
    let app = notes_runtime(false).harness_in(&dir, 40, 3).expect("the runtime starts");
    let plain = Harness::new(Localized::default(), 40, 3);
    assert_eq!(app.env().depth(), crate::color::ColorDepth::TrueColor);
    // The ground of the screen is the theme's own colour, as in a harness made with `new`.
    assert_eq!(app.bg(39, 2), plain.bg(39, 2), "the canvas is painted the same");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_runtime_that_lost_its_membership_does_not_follow_the_ecosystem() {
    let dir = folder("runtime-harness-alone");
    shared(&dir, "tr", "nordic");
    let app = notes_runtime(false).harness_in(&dir, 40, 3).expect("the runtime starts");
    assert_ne!(app.env().theme().id(), "nordic", "no member, no shared theme");
    assert!(app.screen().contains("Open notes"), "{}", app.screen());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_member_starts_with_the_shared_preferences_and_hears_them_before_init() {
    let dir = folder("start");
    shared(&dir, "tr", "nordic");
    let app = follower(&dir);
    assert_eq!(app.env().theme().id(), "nordic");
    assert_eq!(app.env().i18n().active(), "tr");
    assert_eq!(app.env().icon_mode(), IconMode::Unicode);
    assert_eq!(app.app().steps, ["theme nordic tr", "init"], "heard first, then init");
    assert!(app.screen().contains("Görünüm"), "the first frame speaks Turkish\n{}", app.screen());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_following_app_switches_its_theme_when_the_shared_file_changes_and_hears_it() {
    let dir = folder("theme");
    shared(&dir, "en", "nordic");
    let mut app = follower(&dir);
    // Another application switches every follower to amber.
    QUVYTA.set_in(&dir, "desk", Shared::Theme, "amber", Scope::Ecosystem).expect("saved");
    app.poll_preferences();
    assert_eq!(app.env().theme().id(), "amber");
    assert_eq!(app.app().steps, ["theme nordic en", "init", "theme amber en"]);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_app_with_its_own_theme_keeps_it() {
    let dir = folder("own");
    shared(&dir, "en", "nordic");
    fs::write(dir.join("code.conf"), "theme = \"iris\"\n").expect("code.conf");
    let mut app = follower(&dir);
    assert_eq!(app.env().theme().id(), "iris");
    QUVYTA.set_in(&dir, "desk", Shared::Theme, "amber", Scope::Ecosystem).expect("saved");
    app.poll_preferences();
    assert_eq!(app.env().theme().id(), "iris", "its own choice stays");
    assert_eq!(app.app().steps, ["theme iris en", "init"], "nothing it uses changed");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn writing_the_same_value_again_does_nothing() {
    let dir = folder("same");
    shared(&dir, "en", "nordic");
    let mut app = follower(&dir);
    let builds = app.app().builds.get();
    // A real write of the same text, as a save that changed nothing would make.
    shared(&dir, "en", "nordic");
    app.poll_preferences();
    assert_eq!(app.app().steps, ["theme nordic en", "init"], "no hook");
    assert_eq!(app.app().builds.get(), builds, "no frame");
    assert_eq!(app.env().theme().id(), "nordic");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_language_changes_live() {
    let dir = folder("language");
    shared(&dir, "en", "nordic");
    let mut app = follower(&dir);
    assert!(app.screen().contains("Appearance"), "{}", app.screen());
    QUVYTA.set_in(&dir, "desk", Shared::Language, "tr", Scope::Ecosystem).expect("saved");
    app.poll_preferences();
    assert_eq!(app.env().i18n().active(), "tr");
    assert!(app.screen().contains("Görünüm"), "the next frame speaks Turkish\n{}", app.screen());
    assert_eq!(app.app().steps.last().map(String::as_str), Some("theme nordic tr"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_apps_own_file_changes_reduced_motion_and_the_pillar() {
    let dir = folder("motion");
    shared(&dir, "en", "nordic");
    let mut app = follower(&dir);
    assert!(!app.env().reduced_motion());
    fs::write(dir.join("code.conf"), "theme = \"amber\"\nreduced-motion = true\npillar = \"thin\"\n")
        .expect("code.conf");
    app.poll_preferences();
    assert!(app.env().reduced_motion());
    assert_eq!(app.env().pillar_style(), Some(PillarStyle::Thin));
    assert_eq!(app.env().theme().id(), "amber");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_change_the_app_made_and_saved_itself_does_not_loop() {
    let dir = folder("loop");
    shared(&dir, "en", "nordic");
    let mut app = follower(&dir);
    // The application switches and saves, as its settings screen would.
    app.set_theme("amber");
    QUVYTA.set_in(&dir, "code", Shared::Theme, "amber", Scope::Ecosystem).expect("saved");
    app.poll_preferences();
    assert_eq!(app.env().theme().id(), "amber");
    let heard = app.app().steps.len();
    let builds = app.app().builds.get();
    app.poll_preferences();
    assert_eq!(app.app().steps.len(), heard, "heard once at most");
    assert_eq!(app.app().builds.get(), builds, "and nothing more");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_theme_tried_but_not_saved_stays_when_another_preference_changes() {
    let dir = folder("preview");
    shared(&dir, "en", "nordic");
    let mut app = follower(&dir);
    // The person tries a theme on a settings screen that saves only on confirm.
    app.set_theme("amber");
    QUVYTA.set_in(&dir, "desk", Shared::Language, "tr", Scope::Ecosystem).expect("saved");
    app.poll_preferences();
    assert_eq!(app.env().i18n().active(), "tr", "what changed is applied");
    assert_eq!(app.env().theme().id(), "amber", "what did not change is left as the screen shows it");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_folder_is_fine() {
    let dir = std::env::temp_dir().join(format!("quvyta-follow-rules-missing-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let mut app = follower(&dir);
    assert_eq!(app.env().theme().id(), "monochrome", "the detected theme");
    assert!(dir.join("quvyta.conf").is_file(), "the first start writes the shared file, as always");
    fs::remove_dir_all(&dir).expect("remove");
    let builds = app.app().builds.get();
    app.poll_preferences();
    assert!(!dir.exists(), "reading again writes nothing");
    assert_eq!(app.env().theme().id(), "monochrome");
    assert_eq!(app.app().builds.get(), builds);
}

#[test]
fn an_app_started_the_old_way_is_not_followed() {
    let mut app = Harness::new(Follower::default(), 30, 3);
    app.poll_preferences();
    assert_eq!(app.app().steps, ["init"], "no preferences to hear");
}

/// An application with an appearance section that refreshes it when the files change.
struct Code {
    settings: Settings,
    appearance: Appearance,
}

#[derive(Clone)]
enum CodeMsg {
    Appearance(AppearanceChange),
    Heard(Preferences),
}

impl App for Code {
    type Msg = CodeMsg;

    fn preferences(&self, preferences: &Preferences) -> Option<CodeMsg> {
        Some(CodeMsg::Heard(preferences.clone()))
    }

    fn update(&mut self, msg: CodeMsg) -> Command<CodeMsg> {
        match msg {
            CodeMsg::Appearance(change) => self.appearance.update(change, &mut self.settings),
            CodeMsg::Heard(preferences) => {
                self.appearance.refresh(preferences);
                Command::none()
            }
        }
    }

    fn view(&self, ui: &mut View<'_, CodeMsg>) {
        SettingsList::show(ui, |list| self.appearance.section(list, CodeMsg::Appearance));
    }
}

#[test]
fn an_open_appearance_section_shows_what_another_application_changed() {
    let dir = folder("appearance");
    shared(&dir, "en", "nordic");
    let preferences = QUVYTA.preferences_in(&dir, "code", &crate::i18n::I18n::builtin());
    let code = Code {
        settings: Settings::open(dir.join("code.conf")).member_of(&QUVYTA),
        appearance: Appearance::new(QUVYTA, "code", preferences).in_folder(&dir),
    };
    let mut app = Harness::member_in(code, QUVYTA, &dir, "code", 70, 24);
    assert!(app.screen().contains("Nordic"), "{}", app.screen());
    // Another application, the one that lists every member, gives code a theme of its own.
    QUVYTA.set_in(&dir, "code", Shared::Theme, "iris", Scope::App).expect("saved");
    app.poll_preferences();
    assert!(app.screen().contains("Iris"), "the row shows the new theme\n{}", app.screen());
    assert_eq!(app.app().appearance.preferences().theme().source, Source::App, "the box is cleared");
    // So the next theme picked here stays with code, as the box now says.
    app.click_text("Iris");
    app.click_text("Amber");
    assert_eq!(fs::read_to_string(dir.join("code.conf")).expect("code.conf"), "theme = \"amber\"\n");
    assert!(
        fs::read_to_string(dir.join("quvyta.conf")).expect("quvyta.conf").contains("theme = \"nordic\""),
        "the other applications keep theirs"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn reduced_motion_chosen_for_every_application_reaches_a_follower_at_start_and_live() {
    let dir = folder("shared-motion");
    fs::write(dir.join("quvyta.conf"), "language = \"en\"\ntheme = \"nordic\"\nreduced-motion = true\n")
        .expect("shared file");
    let mut app = follower(&dir);
    assert!(app.env().reduced_motion(), "the first frame is already still");
    QUVYTA.set_in(&dir, "desk", Shared::ReducedMotion, "false", Scope::Ecosystem).expect("saved");
    app.poll_preferences();
    assert!(!app.env().reduced_motion(), "motion again, as the ecosystem now says");
    // An application that keeps its own need is not moved by the ecosystem.
    fs::write(dir.join("code.conf"), "reduced-motion = true\n").expect("code.conf");
    app.poll_preferences();
    assert!(app.env().reduced_motion());
    QUVYTA.set_in(&dir, "desk", Shared::ReducedMotion, "true", Scope::Ecosystem).expect("saved");
    QUVYTA.set_in(&dir, "desk", Shared::ReducedMotion, "false", Scope::Ecosystem).expect("saved");
    app.poll_preferences();
    assert!(app.env().reduced_motion(), "its own need stays");
    fs::remove_dir_all(&dir).ok();
}
