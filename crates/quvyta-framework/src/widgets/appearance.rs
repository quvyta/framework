//! The appearance rows every application of an ecosystem shows the same way: language, theme,
//! icons and reduced motion, each with the choice of changing it everywhere or here only, then the
//! pillar; and, for an application that asks for its updates, the ecosystem's update notice.

use std::io;
use std::path::PathBuf;

use crate::i18n::I18n;
use crate::icons::{IconMode, PillarStyle};
use crate::runtime::Command;
use crate::storage::{Ecosystem, Preferences, Scope, Setting, Settings, Shared, Source};
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
    /// The "in every application of the ecosystem" box under a shared row was checked (`true`) or
    /// cleared (`false`).
    Everywhere(Shared, bool),
    /// Reduced motion was switched.
    ReducedMotion(bool),
    /// A pillar style was chosen.
    Pillar(PillarStyle),
    /// The ecosystem's update notice was switched on (`true`) or off; see
    /// [`Ecosystem::update_notice`].
    UpdateNotice(bool),
}

/// What became of a change the [`Appearance`] rows saved in the background; see
/// [`Appearance::updates_in_background`].
///
/// The application builds its own message out of it, so a write that failed can be shown as a
/// toast, and hands it back to [`Appearance::saved`], which puts the row back where the person
/// left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppearanceSave {
    /// The file says the new value now.
    Saved,
    /// It could not be written, with the reason. The file still says what it said, and the row
    /// is back to that.
    Failed(String),
}

/// Which row a failed save is shown under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Shared(Shared),
    Pillar,
    UpdateNotice,
}

/// The appearance section of a settings page or a setup wizard: language, theme, icons and reduced
/// motion as the ecosystem shares them, and the pillar, as rows of a
/// [`SettingsList`](super::SettingsList); and, where the application asks for its updates, the
/// ecosystem's update notice with [`updates`](Self::updates).
///
/// Each shared row has a box under it, "In every Quvyta application", checked while the
/// application follows the ecosystem: a change then goes to the ecosystem's shared file and every
/// application that follows it changes too. Cleared, the change stays in the application's own
/// file. The pillar is the application's own. The update notice is one switch
/// for the whole ecosystem, kept in the shared file; see [`Ecosystem::update_notice`]. Its switch
/// can be written in the background, so a settings page never waits for the disk; see
/// [`updates_in_background`](Self::updates_in_background). A change is applied at once
/// and saved at once, each file read again right before it is written; see
/// [`Ecosystem::set`]. When the `QUVYTA_REDUCED_MOTION` environment variable decides, the reduced
/// motion row and its box are disabled and the row says why. Texts come from the framework's language files.
///
/// ```
/// use qframe::i18n::I18n;
/// use qframe::prelude::*;
/// use qframe::storage::{Ecosystem, Settings};
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
/// let ecosystem = Ecosystem::QUVYTA;
/// // An application passes `ecosystem.preferences("code", &i18n)`; the example stays in a folder of its own.
/// let preferences = ecosystem.preferences_in(&folder, "code", &I18n::builtin());
/// let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&folder);
/// let settings = Settings::open(folder.join("code.conf")).member_of(&ecosystem);
/// let mut app = Harness::new(Code { settings, appearance }, 60, 20);
/// assert!(app.screen().contains("In every Quvyta application"));
/// # std::fs::remove_dir_all(&folder).ok();
/// ```
#[derive(Debug, Clone)]
pub struct Appearance {
    ecosystem: Ecosystem,
    app: String,
    folder: Option<PathBuf>,
    preferences: Preferences,
    /// Whether a change is written to the files; a setup wizard holds them back.
    saving: bool,
    /// Whether the update notice row writes the shared file on a thread of its own.
    background: bool,
    failure: Option<(Row, String)>,
}

impl Appearance {
    /// The title the Appearance box gives the row of `key`, in the language of `i18n`, so an
    /// application that shows the same preference elsewhere, such as in a table of what follows
    /// the ecosystem, calls it by the same name.
    #[must_use]
    pub fn label(i18n: &I18n, key: Shared) -> String {
        i18n.translate(shared_label_key(key), &[])
    }

    /// The appearance of application `app` of `ecosystem`, starting from the `preferences`
    /// [`Ecosystem::preferences`] resolved for it. Changes are saved in the ecosystem's folder.
    #[must_use]
    pub fn new(ecosystem: Ecosystem, app: impl Into<String>, preferences: Preferences) -> Self {
        Self { ecosystem, app: app.into(), folder: None, preferences, saving: true, background: false, failure: None }
    }

    /// Saves changes in `folder` as the ecosystem's folder instead of this platform's, for a test
    /// or a demo that must leave the user's own files alone; see [`Ecosystem::set_in`].
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

    /// Has the [update notice](Self::updates) row write the shared file on a thread of its own,
    /// so a settings page never waits for the disk to turn the switch over: the switch shows the
    /// new value at once and the file takes it behind the screen. A file that cannot be written
    /// puts the switch back where the person left it and says so as an
    /// [`AppearanceSave`], which the application shows as a toast.
    ///
    /// The application hands every change to [`update_saving`](Self::update_saving) instead of
    /// [`update`](Self::update), and the outcome back to [`saved`](Self::saved). Every other row
    /// is written as `update` writes it, on the thread that draws.
    #[must_use]
    pub fn updates_in_background(mut self) -> Self {
        self.background = true;
        self
    }

    /// The shared preferences as they stand after the changes made so far.
    #[must_use]
    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    /// Takes `preferences` resolved again after the files changed while the section is open, such
    /// as the ones [`App::preferences`](crate::runtime::App::preferences) hears when another
    /// application switches the theme for the whole ecosystem. The rows then show the new values
    /// and the box under each shared row whether the application follows the ecosystem now, and
    /// the next change is saved where that box says.
    ///
    /// Nothing is written and nothing is applied: the runtime has already switched the screen.
    /// What the person is doing on the section stays as it is: an open list stays open, and a
    /// reason a change could not be saved stays under its row until the next change.
    pub fn refresh(&mut self, preferences: Preferences) {
        self.preferences = preferences;
    }

    /// Adds an "Appearance" heading, the three [shared rows](Self::rows), reduced motion with its
    /// box and the application's own pillar to `list`.
    pub fn section<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        message: impl Fn(AppearanceChange) -> Msg + Clone + 'static,
    ) {
        list.heading(crate::t!("quvyta.appearance.heading"));
        self.rows(list, message.clone());
        self.motion_and_pillar(list, message);
    }

    /// Adds the ecosystem's update notice switch to `list`, with the text saying what it asks and
    /// what it never sends: for an application that asks whether a newer version of itself is out
    /// ([`Command::check_for_update`](crate::runtime::Command::check_for_update)), right after
    /// [`section`](Self::section). The switch is the ecosystem's, one for every application, kept in
    /// the shared file; see [`Ecosystem::update_notice`]. An application that never asks leaves the
    /// row out, so its settings offer nothing that does nothing there.
    ///
    /// The change is written where the section is drawn, unless the application asked for
    /// [`updates_in_background`](Self::updates_in_background).
    pub fn updates<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        message: impl Fn(AppearanceChange) -> Msg + Clone + 'static,
    ) {
        let row = SettingRow::new(crate::t!("quvyta.appearance.updates"));
        let row = match self.failed(Row::UpdateNotice) {
            Some(failure) => row.description(failure),
            None => row.description(crate::t!("quvyta.appearance.updates-text", family = self.ecosystem.title())),
        };
        let on = self.preferences.update_notice();
        list.row(row, |ui| {
            ui.add(Switch::new(on).on_toggle(move |on| message(AppearanceChange::UpdateNotice(on))));
        });
    }

    /// Adds the three rows the ecosystem shares, language, theme and icons, each with its box, to
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
        let icon_names = IconMode::ALL.map(|mode| crate::t!(&mode.label_key()));

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
        list.row(self.row(Row::Shared(Shared::Language), crate::t!(shared_label_key(Shared::Language))), |ui| {
            let names = languages.into_iter().map(|(_, name)| name);
            let select = Select::new(names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Language(codes[index].clone())));
            ui.add(select).width(Length::Cells(width));
        });
        self.everywhere(list, Shared::Language, false, &message);

        let ids: Vec<String> = themes.iter().map(|(id, _)| id.clone()).collect();
        let chosen = ids.iter().position(|id| *id == theme);
        let send = message.clone();
        list.row(self.row(Row::Shared(Shared::Theme), crate::t!(shared_label_key(Shared::Theme))), |ui| {
            let names = themes.into_iter().map(|(_, name)| name);
            let select = Select::new(names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Theme(ids[index].clone())));
            ui.add(select).width(Length::Cells(width));
        });
        self.everywhere(list, Shared::Theme, false, &message);

        let chosen = IconMode::ALL.iter().position(|mode| *mode == icons);
        let send = message.clone();
        list.row(self.row(Row::Shared(Shared::Icons), crate::t!(shared_label_key(Shared::Icons))), |ui| {
            let select = Select::new(icon_names)
                .selected(chosen)
                .on_select(move |index| send(AppearanceChange::Icons(IconMode::ALL[index])));
            ui.add(select).width(Length::Cells(width));
        });
        self.everywhere(list, Shared::Icons, false, &message);
    }

    /// Adds reduced motion with its box, and the pillar, which is the application's own, to `list`.
    fn motion_and_pillar<Msg: Clone + 'static>(
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
        let row = SettingRow::new(crate::t!(shared_label_key(Shared::ReducedMotion))).disabled(forced);
        let row = match self.failed(Row::Shared(Shared::ReducedMotion)) {
            Some(failure) => row.description(failure),
            None => row.description(note),
        };
        let send = message.clone();
        list.row(row, |ui| {
            ui.add(
                Switch::new(reduced).disabled(forced).on_toggle(move |on| send(AppearanceChange::ReducedMotion(on))),
            );
        });
        self.everywhere(list, Shared::ReducedMotion, forced, &message);

        let styles = PillarStyle::ALL.map(|style| crate::t!(&style.label_key()));
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

    /// The box under shared row `key`: checked while the application follows the ecosystem, and
    /// `disabled` with its row.
    fn everywhere<Msg: Clone + 'static>(
        &self,
        list: &mut SettingsRows<'_, Msg>,
        key: Shared,
        disabled: bool,
        message: &(impl Fn(AppearanceChange) -> Msg + Clone + 'static),
    ) {
        let checked = self.preferences.source(key) != Source::App;
        let label = crate::t!("quvyta.appearance.everywhere", family = self.ecosystem.title());
        let send = message.clone();
        list.row(SettingRow::new(label).nested(true).disabled(disabled), |ui| {
            ui.add(
                Checkbox::new(checked)
                    .disabled(disabled)
                    .on_toggle(move |on| send(AppearanceChange::Everywhere(key, on))),
            );
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
                let scope = if on { Scope::Ecosystem } else { Scope::App };
                let value = self.preferences.text(key);
                (Row::Shared(key), self.share(key, &value, Some(scope), settings), Command::none())
            }
            AppearanceChange::ReducedMotion(on) => {
                let saved = self.share(Shared::ReducedMotion, &on.to_string(), None, settings);
                (Row::Shared(Shared::ReducedMotion), saved, Command::set_reduced_motion(on))
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

    /// [`update`](Self::update), and the command that writes the ecosystem's update notice in the
    /// background when [`updates_in_background`](Self::updates_in_background) is on: the switch
    /// shows the new value at once and `saved` is called with what became of the write once it is
    /// done, so the application can show it as a toast and hand it to [`saved`](Self::saved).
    ///
    /// Without the option every change is written as [`update`](Self::update) writes it, on the
    /// thread that draws, and `saved` is never called; an application can then turn the option on
    /// without touching this line.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::storage::{Ecosystem, Settings};
    /// use qframe::widgets::{Appearance, AppearanceChange, AppearanceSave, SettingsList};
    ///
    /// struct Code {
    ///     settings: Settings,
    ///     appearance: Appearance,
    ///     /// What the last background write left.
    ///     saved: Option<AppearanceSave>,
    /// }
    ///
    /// #[derive(Debug, Clone)]
    /// enum Msg {
    ///     Appearance(AppearanceChange),
    ///     Saved(AppearanceSave),
    /// }
    ///
    /// impl App for Code {
    ///     type Msg = Msg;
    ///     fn update(&mut self, msg: Msg) -> Command<Msg> {
    ///         match msg {
    ///             Msg::Appearance(change) => {
    ///                 self.appearance.update_saving(change, &mut self.settings, Msg::Saved)
    ///             }
    ///             Msg::Saved(save) => {
    ///                 self.saved = Some(save.clone());
    ///                 self.appearance.saved(&save);
    ///                 Command::none()
    ///             }
    ///         }
    ///     }
    ///     fn view(&self, ui: &mut View<'_, Msg>) {
    ///         SettingsList::show(ui, |list| {
    ///             self.appearance.section(list, Msg::Appearance);
    ///             self.appearance.updates(list, Msg::Appearance);
    ///         });
    ///     }
    /// }
    ///
    /// # let folder = std::env::temp_dir().join(format!("quvyta-appearance-saving-doc-{}", std::process::id()));
    /// # let ecosystem = Ecosystem::QUVYTA;
    /// // An application passes `ecosystem.preferences("code", &i18n)`; the example stays in a folder of its own.
    /// # let preferences = ecosystem.preferences_in(&folder, "code", &qframe::i18n::I18n::builtin());
    /// let appearance = Appearance::new(ecosystem, "code", preferences).in_folder(&folder).updates_in_background();
    /// let settings = Settings::open(folder.join("code.conf")).member_of(&ecosystem);
    /// let mut app = Harness::new(Code { settings, appearance, saved: None }, 60, 20);
    /// // The row's change is written off the drawing thread and its outcome comes back as a message.
    /// app.send(Msg::Appearance(AppearanceChange::UpdateNotice(false)));
    /// assert_eq!(app.app().saved, Some(AppearanceSave::Saved), "the file took the new value");
    /// let shared = std::fs::read_to_string(folder.join("quvyta.conf")).expect("the shared file");
    /// assert!(shared.contains("update-notice = false"), "{shared}");
    /// # std::fs::remove_dir_all(&folder).ok();
    /// ```
    pub fn update_saving<Msg: Send + 'static>(
        &mut self,
        change: AppearanceChange,
        settings: &mut Settings,
        saved: impl FnOnce(AppearanceSave) -> Msg + Send + 'static,
    ) -> Command<Msg> {
        match change {
            AppearanceChange::UpdateNotice(on) if self.background => self.write_notice_in_background(on, saved),
            change => self.update(change, settings),
        }
    }

    /// Takes what became of a change the rows saved in the background, as
    /// [`update_saving`](Self::update_saving) and [`updates_in_background`](Self::updates_in_background)
    /// say. A value the file could not take leaves the row showing what the file still says, so
    /// the switch is where the person left it. Nothing is written under the row: the application
    /// has the outcome and shows it itself, as a toast.
    pub fn saved(&mut self, save: &AppearanceSave) {
        if let AppearanceSave::Failed(_) = save {
            self.preferences.record_update_notice(self.notice_in_the_file());
        }
    }

    /// The update notice's value as `on`, written on a thread of its own, and the command that
    /// tells the application how it went.
    fn write_notice_in_background<Msg: Send + 'static>(
        &mut self,
        on: bool,
        saved: impl FnOnce(AppearanceSave) -> Msg + Send + 'static,
    ) -> Command<Msg> {
        // The switch shows the new value at once, as it does for a write on this thread; only the
        // file waits, and a file that refuses the value takes the switch back in `saved`.
        self.preferences.record_update_notice(on);
        if !self.saving {
            return Command::none();
        }
        let ecosystem = self.ecosystem;
        let folder = self.folder.clone();
        Command::perform(move || {
            let written = match &folder {
                Some(folder) => ecosystem.set_update_notice_in(folder, on),
                None => ecosystem.set_update_notice(on),
            };
            saved(match written {
                Ok(()) => AppearanceSave::Saved,
                Err(error) => AppearanceSave::Failed(error.to_string()),
            })
        })
    }

    /// What the ecosystem's file says the update notice is, read now: the truth a failed write
    /// left behind, since the file was not touched.
    fn notice_in_the_file(&self) -> bool {
        match &self.folder {
            Some(folder) => self.ecosystem.update_notice_in(folder),
            None => self.ecosystem.update_notice(),
        }
    }

    /// Writes shared `key` as `value` in `scope`, or in the scope the application follows now when
    /// `None`, and records it.
    fn share(&mut self, key: Shared, value: &str, scope: Option<Scope>, settings: &mut Settings) -> io::Result<()> {
        let scope =
            scope.unwrap_or(if self.preferences.source(key) == Source::App { Scope::App } else { Scope::Ecosystem });
        let written = match scope {
            Scope::Ecosystem => self.ecosystem.id().to_owned(),
            Scope::App => value.to_owned(),
        };
        settings.store(key.key(), key.setting(&written));
        let source = if scope == Scope::Ecosystem { Source::Ecosystem } else { Source::App };
        self.preferences.record(key, value, source);
        if !self.saving {
            return Ok(());
        }
        match &self.folder {
            Some(folder) => self.ecosystem.set_in(folder, &self.app, key, value, scope),
            None => self.ecosystem.set(&self.app, key, value, scope),
        }
    }

    /// Switches the ecosystem's update notice and records it.
    fn update_notice(&mut self, on: bool) -> io::Result<()> {
        self.preferences.record_update_notice(on);
        if !self.saving {
            return Ok(());
        }
        match &self.folder {
            Some(folder) => self.ecosystem.set_update_notice_in(folder, on),
            None => self.ecosystem.set_update_notice(on),
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
                .ecosystem
                .config_dir()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory found"))?,
        };
        self.ecosystem.set_own_in(&folder, &self.app, key, value.to_setting())
    }
}

/// The locale key of the row of `key`; see [`Appearance::label`].
fn shared_label_key(key: Shared) -> &'static str {
    match key {
        Shared::Language => "quvyta.appearance.language",
        Shared::Theme => "quvyta.appearance.theme",
        Shared::Icons => "quvyta.appearance.icons",
        Shared::ReducedMotion => "quvyta.appearance.reduce-motion",
    }
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
