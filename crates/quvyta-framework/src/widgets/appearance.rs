//! The appearance rows every application of a family shows the same way: language, theme and
//! icons, each with the choice of changing it everywhere or here only, then reduced motion and
//! the pillar.

use std::io;
use std::path::PathBuf;

use crate::icons::{IconMode, PillarStyle};
use crate::runtime::Command;
use crate::storage::{Family, Preferences, Scope, Setting, Settings, Shared, Source};
use crate::widget::Length;

use super::{Checkbox, Segmented, Select, SettingRow, SettingsRows, Switch};

/// Cells a choice takes beside its label, wide enough for the longest built-in theme and
/// language names.
const CHOICE_WIDTH: u16 = 18;

/// A change made on the [`Appearance`] rows. The application hands it back to
/// [`Appearance::update`], which saves it and returns the command that shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppearanceChange {
    /// A language was chosen, by locale code.
    Language(String),
    /// A theme was chosen, by id.
    Theme(String),
    /// An icon mode was chosen.
    Icons(IconMode),
    /// The "in every application of the family" box under a shared row was checked (`true`) or
    /// cleared (`false`).
    Everywhere(Shared, bool),
    /// Reduced motion was switched.
    ReducedMotion(bool),
    /// A pillar style was chosen.
    Pillar(PillarStyle),
}

/// Which row a failed save is shown under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Shared(Shared),
    ReducedMotion,
    Pillar,
}

/// The appearance section of a settings page or a setup wizard: language, theme and icons as the
/// family shares them, reduced motion and the pillar, as rows of a
/// [`SettingsList`](super::SettingsList).
///
/// Each shared row has a box under it, "In every Quvyta application", checked while the
/// application follows the family: a change then goes to the family's shared file and every
/// application that follows it changes too. Cleared, the change stays in the application's own
/// file. Reduced motion and the pillar are the application's own. A change is applied at once
/// and saved at once, each file read again right before it is written; see
/// [`Family::set`]. When the `QUVYTA_REDUCED_MOTION` environment variable decides, the reduced
/// motion row is disabled and says why. Texts come from the framework's language files.
///
/// ```
/// use qframe::i18n::I18n;
/// use qframe::prelude::*;
/// use qframe::storage::{Family, Settings};
/// use qframe::widgets::{Appearance, AppearanceChange, SettingsList};
///
/// struct Code {
///     settings: Settings,
///     appearance: Appearance,
/// }
///
/// #[derive(Debug, Clone)]
/// enum Msg {
///     Appearance(AppearanceChange),
/// }
///
/// impl App for Code {
///     type Msg = Msg;
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::Appearance(change) => self.appearance.update(change, &mut self.settings),
///         }
///     }
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         SettingsList::show(ui, |list| self.appearance.section(list, Msg::Appearance));
///     }
/// }
///
/// # let folder = std::env::temp_dir().join(format!("quvyta-appearance-doc-{}", std::process::id()));
/// let family = Family::QUVYTA;
/// // An application passes `family.preferences("code", &i18n)`; the example stays in a folder of its own.
/// let preferences = family.preferences_in(&folder, "code", &I18n::builtin());
/// let appearance = Appearance::new(family, "code", preferences).in_folder(&folder);
/// let settings = Settings::open(folder.join("code.conf")).member_of(&family);
/// let mut app = Harness::new(Code { settings, appearance }, 60, 20);
/// assert!(app.screen().contains("In every Quvyta application"));
/// # std::fs::remove_dir_all(&folder).ok();
/// ```
#[derive(Debug, Clone)]
pub struct Appearance {
    family: Family,
    app: String,
    folder: Option<PathBuf>,
    preferences: Preferences,
    failure: Option<(Row, String)>,
}

impl Appearance {
    /// The appearance of application `app` of `family`, starting from the `preferences`
    /// [`Family::preferences`] resolved for it. Changes are saved in the family's folder.
    #[must_use]
    pub fn new(family: Family, app: impl Into<String>, preferences: Preferences) -> Self {
        Self { family, app: app.into(), folder: None, preferences, failure: None }
    }

    /// Saves changes in `folder` as the family's folder instead of this platform's, for a test
    /// or a demo that must leave the user's own files alone; see [`Family::set_in`].
    #[must_use]
    pub fn in_folder(mut self, folder: impl Into<PathBuf>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    /// The shared preferences as they stand after the changes made so far.
    #[must_use]
    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    /// Adds an "Appearance" heading and the [rows](Self::rows) to `list`.
    pub fn section<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        message: impl Fn(AppearanceChange) -> Msg + Clone + 'static,
    ) {
        list.heading(crate::t!("quvyta.appearance.heading"));
        self.rows(list, message);
    }

    /// Adds the rows to `list`, without a heading, for a page that names the section itself,
    /// such as the first step of a setup wizard. Every change is sent as `message`.
    pub fn rows<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        message: impl Fn(AppearanceChange) -> Msg + Clone + 'static,
    ) {
        let env = list.env();
        let languages = env.i18n().list();
        let active = env.i18n().active().to_owned();
        let themes = env.themes();
        let theme = env.theme().id().to_owned();
        let icons = env.icon_mode();
        let (reduced, forced) = (env.reduced_motion(), env.reduced_motion_forced());
        let pillar = env.pillar_style().unwrap_or(PillarStyle::Thick);

        let codes: Vec<String> = languages.iter().map(|(code, _)| code.clone()).collect();
        let chosen = codes.iter().position(|code| *code == active);
        let send = message.clone();
        list.row(self.row(Row::Shared(Shared::Language), crate::t!("quvyta.appearance.language")), |ui| {
            let names = languages.into_iter().map(|(_, name)| name);
            let select = Select::new(names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Language(codes[index].clone())));
            ui.add(select).width(Length::Cells(CHOICE_WIDTH));
        });
        self.everywhere(list, Shared::Language, &message);

        let ids: Vec<String> = themes.iter().map(|(id, _)| id.clone()).collect();
        let chosen = ids.iter().position(|id| *id == theme);
        let send = message.clone();
        list.row(self.row(Row::Shared(Shared::Theme), crate::t!("quvyta.appearance.theme")), |ui| {
            let names = themes.into_iter().map(|(_, name)| name);
            let select = Select::new(names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Theme(ids[index].clone())));
            ui.add(select).width(Length::Cells(CHOICE_WIDTH));
        });
        self.everywhere(list, Shared::Theme, &message);

        let chosen = IconMode::ALL.iter().position(|mode| *mode == icons);
        let send = message.clone();
        list.row(self.row(Row::Shared(Shared::Icons), crate::t!("quvyta.appearance.icons")), |ui| {
            let names = IconMode::ALL.map(|mode| crate::t!(&format!("quvyta.appearance.icons-{}", mode.name())));
            let select = Select::new(names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Icons(IconMode::ALL[index])));
            ui.add(select).width(Length::Cells(CHOICE_WIDTH));
        });
        self.everywhere(list, Shared::Icons, &message);

        let note = match (forced, reduced) {
            (true, true) => crate::t!("quvyta.appearance.forced-on"),
            (true, false) => crate::t!("quvyta.appearance.forced-off"),
            (false, _) => crate::t!("quvyta.appearance.reduce-motion-text"),
        };
        let row = SettingRow::new(crate::t!("quvyta.appearance.reduce-motion")).disabled(forced);
        let row = match self.failed(Row::ReducedMotion) {
            Some(failure) => row.description(failure),
            None => row.description(note),
        };
        let send = message.clone();
        list.row(row, |ui| {
            ui.add(
                Switch::new(reduced).disabled(forced).on_toggle(move |on| send(AppearanceChange::ReducedMotion(on))),
            );
        });

        let styles = PillarStyle::ALL.map(|style| crate::t!(&format!("quvyta.appearance.pillar-{}", style.name())));
        let chosen = PillarStyle::ALL.iter().position(|style| *style == pillar).unwrap_or(0);
        list.row(self.row(Row::Pillar, crate::t!("quvyta.appearance.pillar")), |ui| {
            let segmented = Segmented::new(styles)
                .selected(chosen)
                .on_select(move |index| message(AppearanceChange::Pillar(PillarStyle::ALL[index])));
            ui.add(segmented);
        });
    }

    /// A row labelled `label` that says why its last change could not be saved, if it could not.
    fn row<Msg>(&self, row: Row, label: String) -> SettingRow<Msg> {
        let setting = SettingRow::new(label);
        match self.failed(row) {
            Some(failure) => setting.description(failure),
            None => setting,
        }
    }

    /// Why the last change of `row` was not saved.
    fn failed(&self, row: Row) -> Option<String> {
        self.failure
            .as_ref()
            .filter(|(failed, _)| *failed == row)
            .map(|(_, reason)| crate::t!("quvyta.appearance.not-saved", reason = reason.as_str()))
    }

    /// The box under shared row `key`: checked while the application follows the family.
    fn everywhere<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        key: Shared,
        message: &(impl Fn(AppearanceChange) -> Msg + Clone + 'static),
    ) {
        let checked = self.preferences.source(key) != Source::App;
        let label = crate::t!("quvyta.appearance.everywhere", family = self.family.title());
        let send = message.clone();
        list.row(SettingRow::new(label).nested(true), |ui| {
            ui.add(Checkbox::new(checked).on_toggle(move |on| send(AppearanceChange::Everywhere(key, on))));
        });
    }

    /// Saves `change` and returns the command that shows it at once. `settings` are the
    /// application's own settings as it holds them in memory; they take the change too, so a
    /// later [`Settings::save`] writes what the file now says instead of what it said before.
    ///
    /// A change that cannot be saved is still applied, and the row it was made on says why it was
    /// not saved until the next change.
    pub fn update<Msg: Send + 'static>(&mut self, change: AppearanceChange, settings: &mut Settings) -> Command<Msg> {
        let (row, saved, command) = match change {
            AppearanceChange::Language(code) => {
                let saved = self.share(Shared::Language, &code, None, settings);
                (Row::Shared(Shared::Language), saved, Command::set_locale(code))
            }
            AppearanceChange::Theme(id) => {
                let saved = self.share(Shared::Theme, &id, None, settings);
                (Row::Shared(Shared::Theme), saved, Command::set_theme(id))
            }
            AppearanceChange::Icons(mode) => {
                let saved = self.share(Shared::Icons, mode.name(), None, settings);
                (Row::Shared(Shared::Icons), saved, Command::set_icon_mode(mode))
            }
            AppearanceChange::Everywhere(key, on) => {
                let scope = if on { Scope::Family } else { Scope::App };
                let value = self.preferences.text(key);
                (Row::Shared(key), self.share(key, &value, Some(scope), settings), Command::none())
            }
            AppearanceChange::ReducedMotion(on) => {
                let saved = self.own(Settings::REDUCED_MOTION, on, settings);
                (Row::ReducedMotion, saved, Command::set_reduced_motion(on))
            }
            AppearanceChange::Pillar(style) => {
                let saved = self.own(Settings::PILLAR, style.name().to_owned(), settings);
                (Row::Pillar, saved, Command::set_pillar(style))
            }
        };
        self.failure = saved.err().map(|error| (row, error.to_string()));
        command
    }

    /// Writes shared `key` as `value` in `scope`, or in the scope the application follows now when
    /// `None`, and records it.
    fn share(&mut self, key: Shared, value: &str, scope: Option<Scope>, settings: &mut Settings) -> io::Result<()> {
        let scope =
            scope.unwrap_or(if self.preferences.source(key) == Source::App { Scope::App } else { Scope::Family });
        let written = match scope {
            Scope::Family => self.family.id().to_owned(),
            Scope::App => value.to_owned(),
        };
        settings.set(key.key(), written);
        let source = if scope == Scope::Family { Source::Family } else { Source::App };
        self.preferences.record(key, value, source);
        match &self.folder {
            Some(folder) => self.family.set_in(folder, &self.app, key, value, scope),
            None => self.family.set(&self.app, key, value, scope),
        }
    }

    /// Writes the application's own `key` as `value`.
    fn own<T: Setting + Clone>(&self, key: &str, value: T, settings: &mut Settings) -> io::Result<()> {
        settings.set(key, value.clone());
        let folder = match &self.folder {
            Some(folder) => folder.clone(),
            None => self
                .family
                .config_dir()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory found"))?,
        };
        self.family.set_own_in(&folder, &self.app, key, value.to_setting())
    }
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
