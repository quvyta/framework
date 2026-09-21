//! The appearance rows every application of a family shows the same way: language, theme and
//! icons, each with the choice of changing it everywhere or here only, then reduced motion and
//! the pillar; and, for an application that asks for its updates, the family's update notice.

use std::io;
use std::path::PathBuf;

use crate::icons::{IconMode, PillarStyle};
use crate::runtime::Command;
use crate::storage::{Family, Preferences, Scope, Setting, Settings, Shared, Source};
use crate::widget::Length;

use super::{Checkbox, Segmented, Select, SettingRow, SettingsRows, Switch};

/// Narrowest a choice is drawn at, so the three rows keep one column even when every name in
/// them is short.
const CHOICE_MIN: u16 = 18;

/// Widest a choice is drawn at, a little over half of the narrow width the catalogue promises: a
/// name longer than this is cut rather than left to take the row from its label. No built-in
/// language, theme or icon name is near it.
const CHOICE_MAX: u16 = 28;

/// Cells a choice needs to show the longest of `names` whole: the name itself, the three the
/// chevron and the space before it take, and the ground a [`Select`] leaves at each side, which
/// the theme decides and which is why it is asked for rather than assumed.
fn choice_width(names: &[String], padding: u16) -> u16 {
    let longest = names.iter().map(|name| crate::text::width(name)).max().unwrap_or(0);
    crate::widgets::cells::sum([longest, 3, padding.saturating_mul(2)]).clamp(CHOICE_MIN, CHOICE_MAX)
}

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
    /// The family's update notice was switched on (`true`) or off; see
    /// [`Family::update_notice`].
    UpdateNotice(bool),
}

/// Which row a failed save is shown under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Shared(Shared),
    ReducedMotion,
    Pillar,
    UpdateNotice,
}

/// The appearance section of a settings page or a setup wizard: language, theme and icons as the
/// family shares them, reduced motion and the pillar, as rows of a
/// [`SettingsList`](super::SettingsList); and, where the application asks for its updates, the
/// family's update notice with [`updates`](Self::updates).
///
/// Each shared row has a box under it, "In every Quvyta application", checked while the
/// application follows the family: a change then goes to the family's shared file and every
/// application that follows it changes too. Cleared, the change stays in the application's own
/// file. Reduced motion and the pillar are the application's own. The update notice is one switch
/// for the whole family, kept in the shared file; see [`Family::update_notice`]. A change is applied at once
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
    /// Whether a change is written to the files; a setup wizard holds them back.
    saving: bool,
    failure: Option<(Row, String)>,
}

impl Appearance {
    /// The appearance of application `app` of `family`, starting from the `preferences`
    /// [`Family::preferences`] resolved for it. Changes are saved in the family's folder.
    #[must_use]
    pub fn new(family: Family, app: impl Into<String>, preferences: Preferences) -> Self {
        Self { family, app: app.into(), folder: None, preferences, saving: true, failure: None }
    }

    /// Saves changes in `folder` as the family's folder instead of this platform's, for a test
    /// or a demo that must leave the user's own files alone; see [`Family::set_in`].
    #[must_use]
    pub fn in_folder(mut self, folder: impl Into<PathBuf>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    /// Applies every change without writing a file: the [shared preferences](Self::preferences)
    /// and the `settings` given to [`update`](Self::update) take it, the screen shows it, and the
    /// files are left to whoever writes them later.
    ///
    /// For the first step of a [setup wizard](super::Setup), which writes both files only when the
    /// wizard finishes, so a wizard closed half-way leaves nothing behind.
    #[must_use]
    pub fn without_saving(mut self) -> Self {
        self.saving = false;
        self
    }

    /// The shared preferences as they stand after the changes made so far.
    #[must_use]
    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    /// Adds an "Appearance" heading, the three [shared rows](Self::rows) and the application's own
    /// rows, reduced motion and the pillar, to `list`.
    pub fn section<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        message: impl Fn(AppearanceChange) -> Msg + Clone + 'static,
    ) {
        list.heading(crate::t!("quvyta.appearance.heading"));
        self.rows(list, message.clone());
        self.own_rows(list, message);
    }

    /// Adds the family's update notice switch to `list`, with the text saying what it asks and
    /// what it never sends: for an application that asks whether a newer version of itself is out
    /// ([`Command::check_for_update`](crate::runtime::Command::check_for_update)), right after
    /// [`section`](Self::section). The switch is the family's, one for every application, kept in
    /// the shared file; see [`Family::update_notice`]. An application that never asks leaves the
    /// row out, so its settings offer nothing that does nothing there.
    pub fn updates<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        message: impl Fn(AppearanceChange) -> Msg + Clone + 'static,
    ) {
        let row = SettingRow::new(crate::t!("quvyta.appearance.updates"));
        let row = match self.failed(Row::UpdateNotice) {
            Some(failure) => row.description(failure),
            None => row.description(crate::t!("quvyta.appearance.updates-text", family = self.family.title())),
        };
        let on = self.preferences.update_notice();
        list.row(row, |ui| {
            ui.add(Switch::new(on).on_toggle(move |on| message(AppearanceChange::UpdateNotice(on))));
        });
    }

    /// Adds the three rows the family shares, language, theme and icons, each with its box, to
    /// `list`, without a heading and without the application's own rows: what the first step of a
    /// [setup wizard](super::Setup) asks, on a page that names the section itself. Every change is
    /// sent as `message`.
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
        let icon_names = IconMode::ALL.map(|mode| crate::t!(&format!("quvyta.appearance.icons-{}", mode.name())));

        // One width for the three rows, from the longest name any of them offers: a language list
        // whose longest name is `Português (Brasil)` needs more than the built-in themes do, and a
        // column that changed width from row to row would read as three controls, not one group.
        // The ground a select leaves at its sides is the theme's, so the width is asked of the
        // theme rather than assumed; without it the name is cut by exactly that much.
        let padding = env.theme().style("select", None, &[]).pair("padding").map_or(1, |(_, horizontal)| horizontal);
        let width = choice_width(
            &languages
                .iter()
                .map(|(_, name)| name.clone())
                .chain(themes.iter().map(|(_, name)| name.clone()))
                .chain(icon_names.iter().cloned())
                .collect::<Vec<String>>(),
            padding,
        );

        let codes: Vec<String> = languages.iter().map(|(code, _)| code.clone()).collect();
        let chosen = codes.iter().position(|code| *code == active);
        let send = message.clone();
        list.row(self.row(Row::Shared(Shared::Language), crate::t!("quvyta.appearance.language")), |ui| {
            let names = languages.into_iter().map(|(_, name)| name);
            let select = Select::new(names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Language(codes[index].clone())));
            ui.add(select).width(Length::Cells(width));
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
            ui.add(select).width(Length::Cells(width));
        });
        self.everywhere(list, Shared::Theme, &message);

        let chosen = IconMode::ALL.iter().position(|mode| *mode == icons);
        let send = message.clone();
        list.row(self.row(Row::Shared(Shared::Icons), crate::t!("quvyta.appearance.icons")), |ui| {
            let select = Select::new(icon_names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Icons(IconMode::ALL[index])));
            ui.add(select).width(Length::Cells(width));
        });
        self.everywhere(list, Shared::Icons, &message);
    }

    /// Adds the rows that are the application's own, reduced motion and the pillar, to `list`.
    fn own_rows<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        message: impl Fn(AppearanceChange) -> Msg + Clone + 'static,
    ) {
        let env = list.env();
        let (reduced, forced) = (env.reduced_motion(), env.reduced_motion_forced());
        let pillar = env.pillar_style().unwrap_or(PillarStyle::Thick);

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
            AppearanceChange::UpdateNotice(on) => (Row::UpdateNotice, self.update_notice(on), Command::none()),
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
        if !self.saving {
            return Ok(());
        }
        match &self.folder {
            Some(folder) => self.family.set_in(folder, &self.app, key, value, scope),
            None => self.family.set(&self.app, key, value, scope),
        }
    }

    /// Switches the family's update notice and records it.
    fn update_notice(&mut self, on: bool) -> io::Result<()> {
        self.preferences.record_update_notice(on);
        if !self.saving {
            return Ok(());
        }
        match &self.folder {
            Some(folder) => self.family.set_update_notice_in(folder, on),
            None => self.family.set_update_notice(on),
        }
    }

    /// Writes the application's own `key` as `value`.
    fn own<T: Setting + Clone>(&self, key: &str, value: T, settings: &mut Settings) -> io::Result<()> {
        settings.set(key, value.clone());
        if !self.saving {
            return Ok(());
        }
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
