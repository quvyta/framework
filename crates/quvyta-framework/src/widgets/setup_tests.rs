use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Shell;
use std::time::Duration;

use super::*;
use crate::i18n::I18n;
use crate::icons::nerd_font::{Archive, FONT_FILE};
use crate::runtime::{App, Harness};
use crate::storage::{Ecosystem, Settings, Shared};
use crate::widgets::{Select, Text};

/// The application the wizard belongs to in these tests.
const APP: &str = "code";

/// A folder of this test's own; the real settings folder is never touched.
fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("quvyta-setup-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Setup(SetupMsg),
    Done,
    Engine(usize),
}

/// An application whose first start shows the wizard, with one step of its own.
struct Demo {
    setup: Setup<Msg>,
    settings: Settings,
    engine: usize,
    done: bool,
}

impl Demo {
    /// A demo whose ecosystem folder, install and font folders are all inside `folder`.
    fn new(folder: &Path) -> Self {
        let ecosystem = Ecosystem::QUVYTA;
        let setup = Setup::new_in(folder, ecosystem, APP, &I18n::builtin(), Msg::Setup)
            .on_finish(Msg::Done)
            .install(Install::new().target(folder.join("fonts").join("QuvytaNerdFont")).register(false))
            .font_dirs(vec![folder.join("fonts")]);
        let settings = Settings::open(folder.join("code.conf")).member_of(&ecosystem);
        Self { setup, settings, engine: 0, done: false }
    }
}

impl App for Demo {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Setup(message) => self.setup.update(message, &mut self.settings),
            Msg::Done => {
                self.done = true;
                Command::none()
            }
            Msg::Engine(engine) => {
                self.engine = engine;
                Command::none()
            }
        }
    }

    fn view(&self, ui: &mut View<'_, Msg>) {
        if !self.setup.needed() {
            ui.add(Text::new("Everything is set"));
            return;
        }
        SetupWizard::new(&self.setup)
            .step("Containers", |ui| {
                ui.add(Select::new(["podman", "docker"]).selected(Some(self.engine)).on_select(Msg::Engine))
                    .id("engine");
            })
            .show(ui);
    }
}

/// A wizard on its first step in a folder of its own, wide enough for the rows.
fn wizard(name: &str) -> (Harness<Demo>, PathBuf) {
    let folder = folder(name);
    (Harness::new(Demo::new(&folder), 60, 30), folder)
}

/// What a file of the ecosystem folder says, or nothing when it does not exist.
fn file(folder: &Path, name: &str) -> String {
    fs::read_to_string(folder.join(name)).unwrap_or_default()
}

#[test]
fn it_opens_only_while_the_application_has_no_file_of_its_own() {
    let folder = folder("needed");
    fs::create_dir_all(&folder).expect("the folder");
    let first = Demo::new(&folder);
    assert!(first.setup.needed(), "no code.conf yet");
    fs::write(folder.join("code.conf"), "theme = \"iris\"\n").expect("code.conf");
    assert!(!Demo::new(&folder).setup.needed(), "the application has been set up");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn closing_it_half_way_writes_nothing_at_all() {
    let (mut h, folder) = wizard("half-way");
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Theme("nordic".to_owned()))));
    h.send(Msg::Setup(SetupMsg::Next));
    assert_eq!(h.app().setup.step(), 1);
    assert!(!folder.join("quvyta.conf").exists(), "no shared file");
    assert!(!folder.join("code.conf").exists(), "no application file");
    assert!(Demo::new(&folder).setup.needed(), "it comes again next start");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn starting_with_the_defaults_writes_both_files_and_never_asks_again() {
    let (mut h, folder) = wizard("defaults");
    h.click_text("Start with the defaults");
    h.advance(Duration::from_millis(0));
    assert!(h.app().done, "the application is told to write its own keys");
    assert!(file(&folder, "quvyta.conf").contains("theme = \"monochrome\""), "{}", file(&folder, "quvyta.conf"));
    assert!(file(&folder, "code.conf").contains("theme = \"quvyta\""), "{}", file(&folder, "code.conf"));
    assert!(!h.app().setup.needed());
    assert!(h.screen().contains("Everything is set"), "{}", h.screen());
    assert!(!Demo::new(&folder).setup.needed(), "and never again");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn the_box_decides_whether_a_choice_goes_to_the_ecosystem_or_to_the_application() {
    let (mut h, folder) = wizard("scope");
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Language("tr".to_owned()))));
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Everywhere(Shared::Theme, false))));
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Theme("nordic".to_owned()))));
    h.send(Msg::Setup(SetupMsg::Next));
    h.send(Msg::Setup(SetupMsg::Finish));
    h.advance(Duration::from_millis(0));
    let shared = file(&folder, "quvyta.conf");
    let own = file(&folder, "code.conf");
    assert!(shared.contains("language = \"tr\""), "the ecosystem takes the language: {shared}");
    assert!(own.contains("language = \"quvyta\""), "and the application follows it: {own}");
    assert!(own.contains("theme = \"nordic\""), "the theme stays here: {own}");
    assert!(!shared.contains("theme"), "a choice made here alone never reaches the ecosystem: {shared}");
    assert!(shared.contains("icons = "), "what the boxes keep does reach it: {shared}");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn back_works_on_every_step_and_keeps_the_choices() {
    let (mut h, folder) = wizard("back");
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Theme("iris".to_owned()))));
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Everywhere(Shared::Icons, false))));
    h.click_text("Next");
    assert_eq!(h.app().setup.step(), 1);
    h.click_text("Back");
    assert_eq!(h.app().setup.step(), 0);
    assert_eq!(h.env().theme().id(), "iris", "the theme is still the chosen one");
    assert_eq!(h.app().setup.preferences().source(Shared::Icons), Source::App, "the cleared box is still cleared");
    assert!(h.screen().contains("Iris"), "{}", h.screen());
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_theme_chosen_on_the_first_step_draws_the_wizard_in_that_theme_at_once() {
    let (mut h, folder) = wizard("theme");
    let before = h.bg(0, 0);
    h.send(Msg::Setup(SetupMsg::Appearance(AppearanceChange::Theme("nordic".to_owned()))));
    assert_eq!(h.env().theme().id(), "nordic");
    assert_ne!(h.bg(0, 0), before, "the wizard is drawn in the new theme");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn every_language_names_the_rows_of_the_first_step() {
    for (code, appearance, language, defaults) in [
        ("en", "Appearance", "Language", "Start with the defaults"),
        ("tr", "Görünüm", "Dil", "Varsayılanlarla başla"),
        ("de", "Darstellung", "Sprache", "Mit den Vorgaben beginnen"),
    ] {
        let (mut h, folder) = wizard(&format!("language-{code}"));
        h.set_locale(code);
        let screen = h.screen();
        assert!(screen.contains(appearance), "{code}: {screen}");
        assert!(screen.contains(language), "{code}: {screen}");
        assert!(screen.contains(defaults), "{code}: {screen}");
        let _ = fs::remove_dir_all(&folder);
    }
}

#[test]
fn forty_columns_cuts_no_word() {
    let folder = folder("narrow");
    let mut h = Harness::new(Demo::new(&folder), 40, 40);
    let screen = h.screen();
    for word in ["Appearance", "Language", "In every Quvyta application", "Start with the defaults"] {
        assert!(screen.contains(word), "“{word}” is cut: {screen}");
    }
    assert!(screen.lines().all(|line| line.chars().count() <= 40), "{screen}");
    h.click_text("Start with the defaults");
    h.advance(Duration::from_millis(0));
    assert!(h.app().done, "the narrow layout is clickable too");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn the_three_glyph_modes_show_their_own_icons_beside_their_names() {
    let (mut h, folder) = wizard("glyphs");
    h.set_glyph_mode(GlyphMode::Ascii);
    let screen = h.screen();
    assert!(screen.contains("\u{f07b}  \u{f00c}  \u{f002}  \u{f013}"), "the Nerd glyphs: {screen}");
    assert!(screen.contains("■  ✓  ⌕  ▤"), "the Unicode glyphs: {screen}");
    assert!(screen.contains("#  v  /  *"), "the ASCII glyphs: {screen}");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn the_install_offer_comes_only_without_a_nerd_font_and_says_what_to_look_at() {
    let folder = folder("install");
    fs::create_dir_all(folder.join("content")).expect("the archive content");
    fs::write(folder.join("content").join(FONT_FILE), b"glyphs").expect("the font");
    let archive = folder.join("symbols.tar");
    let packed = Shell::new("tar")
        .arg("-cf")
        .arg(&archive)
        .arg("-C")
        .arg(folder.join("content"))
        .arg(FONT_FILE)
        .status()
        .expect("tar");
    assert!(packed.success());
    let digest = Shell::new("sha256sum").arg(&archive).output().expect("sha256sum");
    let digest = String::from_utf8_lossy(&digest.stdout).split_whitespace().next().expect("a digest").to_owned();

    let mut demo = Demo::new(&folder);
    demo.setup = demo
        .setup
        .install(
            Install::new()
                .archive(Archive::new(format!("file://{}", archive.display()), &digest))
                .target(folder.join("fonts").join("QuvytaNerdFont"))
                .register(false),
        )
        .font_dirs(vec![folder.join("fonts")]);
    let mut h = Harness::new(demo, 60, 30);
    assert!(h.screen().contains("No Nerd Font was found on this machine"), "{}", h.screen());
    h.click_text("Install the font");
    h.advance(Duration::from_millis(0));
    let screen = h.screen();
    assert!(folder.join("fonts/QuvytaNerdFont").join(FONT_FILE).is_file(), "{screen}");
    assert!(screen.contains("JetBrainsMono Nerd Font"), "the honest next step: {screen}");
    assert!(!screen.contains("Install the font"), "nothing left to install: {screen}");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_finish_that_cannot_be_saved_says_so_and_the_wizard_comes_again() {
    let folder = folder("failure");
    fs::create_dir_all(folder.parent().expect("a parent")).expect("the parent folder");
    // A file where the ecosystem folder should be: nothing can be written into it.
    fs::write(&folder, b"not a folder").expect("the file in the way");
    let mut h = Harness::new(Demo::new(&folder), 60, 30);
    h.click_text("Start with the defaults");
    h.advance(Duration::from_millis(0));
    assert!(!h.app().done, "the application is not told it is over");
    assert!(h.app().setup.needed(), "the wizard stays");
    assert!(h.screen().contains("could not be saved"), "{}", h.screen());
    let _ = fs::remove_file(&folder);
}
