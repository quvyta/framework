//! Shared preferences: language, theme and icons every application of the family shares, the
//! appearance rows that change them for every application or for one, and the three files that
//! show where each change went.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use qframe::i18n::I18n;
use qframe::prelude::*;
use qframe::storage::{Family, Preferences, Scope, Settings, Shared, Source};
use qframe::widgets::{Appearance, AppearanceChange, CodeView, Language, SettingsList};

use super::{PageMsg, setting, toggle};
use crate::app::Msg as AppMsg;
use crate::log::EventLog;

const PAGE: &str = "shared-preferences";

/// The application the rows belong to in the demo.
const APP: &str = "code";

/// A second application of the family, to show who follows a shared change.
const OTHER: &str = "focus";

/// The theme the second application keeps when the playground gives it one of its own.
const OWN_THEME: &str = "iris";

/// The demo's family folder and what lives in it; made the first time the page is drawn.
#[derive(Debug)]
struct Demo {
    folder: PathBuf,
    appearance: Appearance,
    /// `code`'s own settings as an application holds them in memory.
    settings: Settings,
}

impl Demo {
    fn new() -> Self {
        let folder = demo_dir();
        // region: shared-preferences-start
        let family = Family::QUVYTA;
        // An application passes `family.preferences(APP, &i18n)`; the demo keeps to a folder of its own.
        let preferences = family.preferences_in(&folder, APP, &I18n::builtin());
        let appearance = Appearance::new(family, APP, preferences).in_folder(&folder);
        let settings = Settings::open(folder.join(format!("{APP}.conf"))).member_of(&family);
        // endregion
        Self { folder, appearance, settings }
    }
}

/// The demo, made on first use, and the playground.
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
    /// Takes the demo folder away with the page, so a run leaves nothing in the temporary folder.
    fn drop(&mut self) {
        if let Some(demo) = self.demo.get() {
            let _ = std::fs::remove_dir_all(&demo.folder);
        }
    }
}

/// Tells the demo folders of two showcases in one process apart, which the tests need.
static DEMO: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A family folder of this run, never the user's own.
fn demo_dir() -> PathBuf {
    let ticket = DEMO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("quvyta-showcase-preferences-{}-{ticket}", std::process::id()))
}

/// Demo messages.
#[derive(Debug, Clone)]
pub enum Msg {
    Appearance(AppearanceChange),
    /// The second application keeps a theme of its own (`true`) or follows the family.
    OtherOwnTheme(bool),
    StartOver,
}

fn send(message: Msg) -> AppMsg {
    AppMsg::Page(PageMsg::SharedPreferences(message))
}

/// Applies a demo message.
pub fn update(state: &mut State, message: Msg, log: &mut EventLog) -> Command<AppMsg> {
    match message {
        Msg::Appearance(change) => {
            log.push(PAGE, "Appearance::update", format!("{change:?}"));
            let demo = state.demo_mut();
            // region: shared-preferences-update
            demo.appearance.update(change, &mut demo.settings)
            // endregion
        }
        Msg::OtherOwnTheme(own) => {
            let demo = state.demo();
            let family = Family::QUVYTA;
            // region: shared-preferences-follow
            let written = if own {
                family.set_in(&demo.folder, OTHER, Shared::Theme, OWN_THEME, Scope::App)
            } else {
                // Back to the family: only focus.conf is written, so the theme the whole family
                // draws with stays exactly as it is.
                family.follow_in(&demo.folder, OTHER, Shared::Theme)
            };
            // endregion
            let outcome = match written {
                Ok(()) => format!("{OTHER} keeps its own theme = {own}"),
                Err(error) => error.to_string(),
            };
            log.push(PAGE, if own { "Family::set_in" } else { "Family::follow_in" }, outcome);
            Command::none()
        }
        Msg::StartOver => {
            if let Some(demo) = state.demo.take() {
                // Starting over only needs the page to forget the folder, which the `take` above
                // has done. The folder is this run's own under the temporary folder, so one left
                // behind costs nothing of the person's and the next run makes a fresh one.
                let _ = std::fs::remove_dir_all(&demo.folder);
            }
            log.push(PAGE, "Family::preferences_in", "a new family folder");
            Command::none()
        }
    }
}

/// What `app` resolves in `folder`, as one line per preference.
fn resolved(folder: &Path, app: &str) -> Preferences {
    Family::QUVYTA.preferences_in(folder, app, &I18n::builtin())
}

/// Where a value came from, in words.
fn source(source: Source) -> String {
    match source {
        Source::App => t!("shared-preferences.from-app"),
        Source::Family => t!("shared-preferences.from-family"),
        Source::Detected => t!("shared-preferences.from-detected"),
    }
}

/// One file of the family folder, under its name.
fn file(ui: &mut View<'_, AppMsg>, folder: &Path, name: &str) {
    let contents = std::fs::read_to_string(folder.join(name)).unwrap_or_default();
    ui.add(Text::new(name.to_owned()).role("faint").no_wrap());
    if contents.is_empty() {
        ui.add(Text::new(t!("shared-preferences.no-file")).role("secondary"));
    } else {
        ui.add(CodeView::new(contents, Language::Toml).line_numbers(false)).fill_width();
    }
}

/// The live demo, the files and the playground.
pub fn view(state: &State, ui: &mut View<'_, AppMsg>) {
    let demo = state.demo();
    ui.add_with(Panel::new().title(t!("demo.live")).gap(0), |ui| {
        ui.add(Text::new(t!("shared-preferences.hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        // region: shared-preferences-rows
        SettingsList::show(ui, |list| {
            demo.appearance.section(list, |change| send(Msg::Appearance(change)));
        })
        .id("appearance");
        // endregion
        // Room for a dropdown opened on the last shared row.
        ui.spacer().height(Length::Cells(3));
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("shared-preferences.files")).gap(0), |ui| {
        ui.add(Text::new(t!("shared-preferences.files-hint")).role("secondary"));
        ui.spacer().height(Length::Cells(1));
        for name in ["quvyta.conf", "code.conf", "focus.conf"] {
            file(ui, &demo.folder, name);
            ui.spacer().height(Length::Cells(1));
        }
        let other = resolved(&demo.folder, OTHER);
        for (label, value, from) in [
            (t!("shared-preferences.language"), other.language().value.clone(), other.language().source),
            (t!("shared-preferences.theme"), other.theme().value.clone(), other.theme().source),
            (t!("shared-preferences.icons"), other.icons().value.name().to_owned(), other.icons().source),
        ] {
            let line = t!("shared-preferences.sees", app = OTHER, key = label, value = value, source = source(from));
            ui.add(Text::new(line).role("body"));
        }
    })
    .fill_width();

    ui.add_with(Panel::new().title(t!("demo.playground")).gap(0), |ui| {
        let own = std::fs::read_to_string(demo.folder.join("focus.conf"))
            .is_ok_and(|text| text.contains(&format!("theme = \"{OWN_THEME}\"")));
        setting(ui, t!("shared-preferences.other-own"), |ui| {
            ui.add(toggle(own, |on| send(Msg::OtherOwnTheme(on)))).id("other-own");
        });
        ui.add(Button::new(t!("shared-preferences.start-over")).on_press(send(Msg::StartOver))).id("start-over");
    })
    .fill_width();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{showcase_on, showcase_tall};

    fn folder(h: &Harness<crate::app::Showcase>) -> PathBuf {
        h.app().pages.shared_preferences.demo().folder.clone()
    }

    #[test]
    fn the_rows_and_the_files_show_the_demo_folder_only() {
        let h = showcase_on(PAGE);
        let screen = h.screen();
        assert!(screen.contains("In every Quvyta application"), "{screen}");
        let folder = folder(&h);
        assert!(folder.starts_with(std::env::temp_dir()), "never the user's own folder");
        assert!(folder.join("quvyta.conf").is_file(), "the shared file is made in the demo folder");
    }

    #[test]
    fn clearing_the_box_keeps_a_change_in_code_and_focus_follows_the_family() {
        let mut h = showcase_tall(crate::app::Showcase::new(), PAGE, 80);
        let folder = folder(&h);
        h.send(send(Msg::Appearance(AppearanceChange::Everywhere(Shared::Theme, false))));
        h.send(send(Msg::Appearance(AppearanceChange::Theme("nordic".to_owned()))));
        assert_eq!(h.env().theme().id(), "nordic");
        let code = std::fs::read_to_string(folder.join("code.conf")).expect("code.conf");
        assert!(code.contains("theme = \"nordic\""), "{code}");
        assert_eq!(resolved(&folder, OTHER).theme().source, Source::Family);
        assert!(h.screen().contains("focus sees"), "{}", h.screen());

        h.send(send(Msg::Appearance(AppearanceChange::Everywhere(Shared::Theme, true))));
        assert_eq!(resolved(&folder, OTHER).theme().value, "nordic", "focus follows the family now");
        h.send(send(Msg::OtherOwnTheme(true)));
        assert_eq!(resolved(&folder, OTHER).theme().value, OWN_THEME);
        let shared = std::fs::read_to_string(folder.join("quvyta.conf")).expect("quvyta.conf");
        h.send(send(Msg::OtherOwnTheme(false)));
        assert_eq!(resolved(&folder, OTHER).theme().source, Source::Family, "focus follows again");
        assert_eq!(resolved(&folder, OTHER).theme().value, "nordic");
        assert_eq!(
            std::fs::read_to_string(folder.join("quvyta.conf")).expect("quvyta.conf"),
            shared,
            "following again leaves the family's own theme alone"
        );
    }

    #[test]
    fn starting_over_leaves_a_fresh_folder() {
        let mut h = showcase_on(PAGE);
        let first = folder(&h);
        h.send(send(Msg::StartOver));
        assert!(!first.exists());
        assert_ne!(folder(&h), first);
    }
}
