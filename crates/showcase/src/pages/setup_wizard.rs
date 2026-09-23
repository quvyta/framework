//! The first-run setup wizard: the appearance step the framework draws, two steps of the
//! application's own, and the two files that appear only when it finishes.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use qframe::i18n::I18n;
use qframe::icons::nerd_font::Install;
use qframe::prelude::*;
use qframe::storage::{Ecosystem, Settings};
use qframe::widgets::{CodeView, Language, Select, Setup, SetupMsg, SetupWizard, TextInput};

use super::PageMsg;
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "setup-wizard";

/// The application the demo sets up.
const APP: &str = "code";

/// Rows every step gets, so the buttons never move between them.
const PAGE_ROWS: u16 = 18;

/// The demo's ecosystem folder and everything in it; made the first time the page is drawn.
#[derive(Debug)]
struct Demo {
    folder: PathBuf,
    // region: setup-start
    setup: Setup<AppMsg>,
    /// The application's own settings, as it holds them in memory.
    settings: Settings,
    // endregion
    /// What the application's own steps ask.
    engine: usize,
    location: String,
    /// Whether the wizard has handed over to the application.
    started: bool,
}

impl Demo {
    fn new() -> Self {
        let folder = demo_dir();
        // region: setup-start
        let ecosystem = Ecosystem::QUVYTA;
        // An application calls `Setup::new(ecosystem, APP, &i18n, ..)`; the demo keeps to a folder of
        // its own, and installs the font into it rather than the user's font folder.
        let setup = Setup::new_in(&folder, ecosystem, APP, &I18n::builtin(), |m| send(Msg::Setup(m)))
            .on_finish(send(Msg::Started))
            .install(demo_install(&folder))
            .font_dirs(vec![folder.join("fonts")]);
        let settings = Settings::open(folder.join(format!("{APP}.conf"))).member_of(&ecosystem);
        // endregion
        let location = ecosystem.workspace_dir("Code").map_or_else(String::new, |dir| dir.display().to_string());
        Self { folder, setup, settings, engine: 0, location, started: false }
    }
}

/// The install the demo runs: the real release archive, but into the demo's own folder, which goes
/// away with the page. The system is not told about it, because a temporary folder is no font
/// folder; the `nerd-font` page installs for real.
#[cfg(not(test))]
fn demo_install(folder: &Path) -> Install {
    Install::new().target(folder.join("fonts").join("QuvytaNerdFont")).register(false)
}

/// Under test nothing reaches the network: the archive is a file that does not exist, so a test
/// that presses Install sees the failure instead of a download.
#[cfg(test)]
fn demo_install(folder: &Path) -> Install {
    Install::new()
        .archive(qframe::icons::nerd_font::Archive::new(
            format!("file://{}", folder.join("missing.tar").display()),
            &"0".repeat(64),
        ))
        .target(folder.join("fonts").join("QuvytaNerdFont"))
        .register(false)
}

/// The demo, made on first use.
#[derive(Debug, Default)]
pub struct State {
    demo: OnceCell<Demo>,
}

impl State {
    fn demo(&self) -> &Demo {
        self.demo.get_or_init(Demo::new)
    }

    fn demo_mut(&mut self) -> &mut Demo {
        self.demo.get_or_init(Demo::new);
        self.demo.get_mut().expect("made just above")
    }
}

impl Drop for State {
    /// Takes the demo folder away with the page, so a run leaves nothing behind.
    fn drop(&mut self) {
        if let Some(demo) = self.demo.get() {
            let _ = std::fs::remove_dir_all(&demo.folder);
        }
    }
}

/// Tells the demo folders of two showcases in one process apart, which the tests need.
static DEMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// An ecosystem folder of this run, never the user's own.
fn demo_dir() -> PathBuf {
    let ticket = DEMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("quvyta-showcase-setup-{}-{ticket}", std::process::id()))
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    /// Everything the wizard's own first step and its buttons send.
    Setup(SetupMsg),
    /// The wizard is over: the application writes its own keys.
    Started,
    Engine(usize),
    Location(String),
    StartOver,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::SetupWizard(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        // region: setup-update
        Msg::Setup(message) => {
            log.push(PAGE, "Setup::update", format!("{message:?}"));
            let demo = state.demo_mut();
            demo.setup.update(message, &mut demo.settings)
        }
        // The wizard has written the shared keys and made the application's file; the keys that
        // are the application's own are the application's to write.
        Msg::Started => {
            let demo = state.demo_mut();
            demo.started = true;
            demo.settings.set("engine", engine_name(demo.engine).to_owned());
            demo.settings.set("projects", demo.location.clone());
            let outcome = match demo.settings.save() {
                Ok(()) => "the application's own keys are saved".to_owned(),
                Err(error) => error.to_string(),
            };
            log.push(PAGE, "Settings::save", outcome);
            Command::none()
        }
        // endregion
        Msg::Engine(engine) => {
            state.demo_mut().engine = engine;
            Command::none()
        }
        Msg::Location(location) => {
            state.demo_mut().location = location;
            Command::none()
        }
        Msg::StartOver => {
            if let Some(demo) = state.demo.take() {
                let _ = std::fs::remove_dir_all(&demo.folder);
            }
            log.push(PAGE, "Setup::new_in", "a new ecosystem folder, so the wizard comes again");
            Command::none()
        }
    }
}

/// The container engines the application's own step offers.
fn engine_name(index: usize) -> &'static str {
    ["podman", "docker"].get(index).copied().unwrap_or("podman")
}

/// The live demo, the files it writes and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let demo = state.demo();
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("setup-wizard.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        if demo.setup.needed() {
            wizard(demo, ui);
        } else {
            ui.add(Text::new(t!("setup-wizard.started"))).fill_width().id("started");
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("setup-wizard.files")).gap(0), |ui| {
        ui.add(Text::new(t!("setup-wizard.files-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        for name in ["quvyta.conf", "code.conf"] {
            file(ui, &demo.folder, name);
            ui.spacer().height(Length::Cells(1));
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        ui.add(Button::new(t!("setup-wizard.start-over")).on_press(send(Msg::StartOver))).id("start-over");
    })
    .fill_width();
}

// region: setup-view
/// The wizard: the framework's appearance step, then the two the application adds.
fn wizard(demo: &Demo, ui: &mut View<'_, AppMsg>) {
    SetupWizard::new(&demo.setup)
        .page_height(PAGE_ROWS)
        .step(t!("setup-wizard.step-containers"), |ui| {
            ui.add(Text::new(t!("setup-wizard.engine")).role("secondary"));
            let engines = Select::new(["podman", "docker"]).selected(Some(demo.engine));
            ui.add(engines.on_select(|index| send(Msg::Engine(index)))).width(Length::Cells(20)).id("engine");
        })
        .step(t!("setup-wizard.step-projects"), |ui| {
            ui.add(Text::new(t!("setup-wizard.projects")).role("secondary"));
            ui.add(TextInput::new(&demo.location).on_change(|text| send(Msg::Location(text))))
                .fill_width()
                .id("projects");
        })
        .show(ui)
        .id("setup");
}
// endregion

/// One file of the ecosystem folder, under its name, or a word that it is not there yet.
fn file(ui: &mut View<'_, AppMsg>, folder: &Path, name: &str) {
    let contents = std::fs::read_to_string(folder.join(name)).unwrap_or_default();
    ui.add(Text::new(name.to_owned()).role("faint").no_wrap());
    if contents.is_empty() {
        ui.add(Text::new(t!("setup-wizard.no-file")).role("secondary"));
    } else {
        ui.add(CodeView::new(contents, Language::Toml).line_numbers(false)).fill_width();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::tests::{showcase_on, showcase_tall};

    fn folder(h: &Harness<crate::app::Showcase>) -> PathBuf {
        h.app().pages.setup_wizard.demo().folder.clone()
    }

    #[test]
    fn the_first_step_is_the_appearance_and_the_folder_is_the_demo_s_own() {
        let h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("In every Quvyta application"), "{screen}");
        assert!(screen.contains("Containers"), "the application's own step: {screen}");
        let folder = folder(&h);
        assert!(folder.starts_with(std::env::temp_dir()), "never the user's own folder");
        assert!(!folder.exists(), "nothing is written until the wizard finishes");
    }

    #[test]
    fn walking_through_it_writes_both_files_and_hands_over_to_the_application() {
        let mut h = showcase_tall(crate::app::Showcase::new(), PAGE, 80);
        let folder = folder(&h);
        h.send(send(Msg::Setup(SetupMsg::Next)));
        h.send(send(Msg::Engine(1)));
        h.send(send(Msg::Setup(SetupMsg::Next)));
        h.send(send(Msg::Setup(SetupMsg::Finish)));
        h.advance(Duration::from_millis(0));
        assert!(h.app().pages.setup_wizard.demo().started, "the application takes over");
        let shared = std::fs::read_to_string(folder.join("quvyta.conf")).expect("quvyta.conf");
        let own = std::fs::read_to_string(folder.join("code.conf")).expect("code.conf");
        assert!(shared.contains("theme = \"monochrome\""), "{shared}");
        assert!(own.contains("theme = \"quvyta\""), "{own}");
        assert!(own.contains("engine = \"docker\""), "the application's own keys: {own}");
        assert!(h.screen().contains("Everything is set up"), "{}", h.screen());
    }

    #[test]
    fn starting_over_brings_the_wizard_back_in_a_fresh_folder() {
        let mut h = showcase_on(PAGE);
        let first = folder(&h);
        h.send(send(Msg::StartOver));
        assert!(!first.exists());
        assert_ne!(folder(&h), first);
        assert!(h.app().pages.setup_wizard.demo().setup.needed());
    }
}
