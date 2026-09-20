//! The first-run setup wizard: the appearance step the framework draws and drives, the steps the
//! application adds, and the two files that are written only when it finishes.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::i18n::I18n;
use crate::icons::nerd_font::{self, Install, Progress};
use crate::icons::{GlyphMode, GlyphSample};
use crate::runtime::Command;
use crate::storage::{Family, Preferences, Scope, Settings, Shared, Source};
use crate::widget::{Length, NodeMut, View};

use super::{Appearance, AppearanceChange, Button, ProgressBar, SettingsList, Text, Wizard};

/// One step an application adds: its name and the page it builds.
type AppStep<'a, Msg> = (String, Box<dyn FnOnce(&mut View<'_, Msg>) + 'a>);

/// Cells the name of a glyph mode takes beside its sample icons.
const SAMPLE_LABEL: u16 = 12;

/// Something that happened on the first step of a [`SetupWizard`], or on its buttons. Every one of
/// them goes to [`Setup::update`]; the application never answers one itself.
#[derive(Debug, Clone, PartialEq)]
pub enum SetupMsg {
    /// A row of the appearance step was changed.
    Appearance(AppearanceChange),
    /// The Nerd Font symbols were asked for.
    Install,
    /// A step of the running install.
    Installing(Progress),
    /// Back: the step before this one.
    Back,
    /// Next: the step after this one.
    Next,
    /// A finished step was chosen from the steps on top.
    Step(usize),
    /// The wizard is over: what was chosen is written, and the application is told with the
    /// message of [`Setup::on_finish`]. "Start with the defaults" sends it from the first step.
    Finish,
}

/// The application-owned state of a [`SetupWizard`]: which step it is on, what has been chosen on
/// the appearance step and how the Nerd Font install is going.
///
/// An application makes one at start, whether or not the wizard is needed, and asks
/// [`Setup::needed`] before drawing its own screen: the wizard opens while the application has no
/// settings file of its own, however much the family has already shared. Nothing is written until
/// it finishes, so closing the application half-way leaves the settings folder as it was and the
/// wizard comes again next start.
///
/// ```
/// use qframe::i18n::I18n;
/// use qframe::prelude::*;
/// use qframe::storage::{Family, Settings};
/// use qframe::widgets::{Select, Setup, SetupMsg, SetupWizard};
///
/// struct Code {
///     setup: Setup<Msg>,
///     settings: Settings,
///     engine: usize,
/// }
///
/// #[derive(Debug, Clone, PartialEq)]
/// enum Msg {
///     Setup(SetupMsg),
///     Engine(usize),
///     Ready,
/// }
///
/// impl App for Code {
///     type Msg = Msg;
///     fn update(&mut self, msg: Msg) -> Command<Msg> {
///         match msg {
///             Msg::Setup(message) => self.setup.update(message, &mut self.settings),
///             Msg::Engine(engine) => {
///                 self.engine = engine;
///                 Command::none()
///             }
///             // The wizard wrote the shared keys and made the file; the application's own keys
///             // are its own to write.
///             Msg::Ready => {
///                 self.settings.set("engine", self.engine as i64);
///                 let _ = self.settings.save();
///                 Command::none()
///             }
///         }
///     }
///     fn view(&self, ui: &mut View<'_, Msg>) {
///         if self.setup.needed() {
///             SetupWizard::new(&self.setup)
///                 .step("Containers", |ui| {
///                     let engines = Select::new(["podman", "docker"]).selected(Some(self.engine));
///                     ui.add(engines.on_select(Msg::Engine));
///                 })
///                 .show(ui);
///         }
///     }
/// }
///
/// # let folder = std::env::temp_dir().join(format!("quvyta-setup-doc-{}", std::process::id()));
/// let family = Family::QUVYTA;
/// // An application calls `Setup::new(family, "code", &i18n, Msg::Setup)`; the example keeps to a
/// // folder of its own.
/// let setup = Setup::new_in(&folder, family, "code", &I18n::builtin(), Msg::Setup).on_finish(Msg::Ready);
/// let settings = Settings::open(folder.join("code.conf")).member_of(&family);
/// let mut app = Harness::new(Code { setup, settings, engine: 0 }, 60, 24);
/// assert!(app.screen().contains("In every Quvyta application"));
/// assert!(!folder.exists(), "nothing is written before the wizard finishes");
/// # std::fs::remove_dir_all(&folder).ok();
/// ```
pub struct Setup<Msg> {
    family: Family,
    app: String,
    /// The family's folder when it is not this platform's own, for a test or a demo.
    folder: Option<PathBuf>,
    appearance: Appearance,
    step: usize,
    /// Whether the wizard is still wanted: the application had no file of its own and the wizard
    /// has not finished.
    needed: bool,
    install: Install,
    /// The folders searched for a Nerd Font, and whether one was found there.
    font_dirs: Vec<PathBuf>,
    installed: bool,
    /// The last step of the running or finished install.
    progress: Option<Progress>,
    wrap: Arc<dyn Fn(SetupMsg) -> Msg + Send + Sync>,
    on_finish: Option<Msg>,
    /// Why finishing could not be saved, until the next try.
    failure: Option<String>,
}

impl<Msg> std::fmt::Debug for Setup<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Setup")
            .field("app", &self.app)
            .field("folder", &self.folder)
            .field("step", &self.step)
            .field("needed", &self.needed)
            .field("installed", &self.installed)
            .field("progress", &self.progress)
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

impl<Msg: Clone + Send + 'static> Setup<Msg> {
    /// The setup of application `app` of `family`, on its first step, with every message of the
    /// first step wrapped as `wrap`.
    ///
    /// The shared preferences are resolved without writing anything
    /// ([`Family::preferences_without_saving`]), so the appearance step comes filled with what the
    /// family already shares, or with what this machine detects, and the user's settings folder
    /// stays as it is until the wizard finishes. An application that has taken over an older
    /// settings file migrates it ([`Family::adopt`](Family::adopt)) before making this, so a
    /// migrated application is not asked again.
    #[must_use]
    pub fn new(
        family: Family,
        app: impl Into<String>,
        i18n: &I18n,
        wrap: impl Fn(SetupMsg) -> Msg + Send + Sync + 'static,
    ) -> Self {
        let app = app.into();
        let preferences = family.preferences_without_saving(&app, i18n);
        let own_file = family.config_dir().map(|dir| dir.join(format!("{app}.conf")));
        let needed = own_file.is_none_or(|file| !file.is_file());
        Self::build(family, app, None, preferences, needed, wrap)
    }

    /// [`new`](Self::new) with `config_dir` as the family's folder instead of this platform's, for
    /// a test or a demo that must leave the user's own files alone.
    #[must_use]
    pub fn new_in(
        config_dir: &Path,
        family: Family,
        app: impl Into<String>,
        i18n: &I18n,
        wrap: impl Fn(SetupMsg) -> Msg + Send + Sync + 'static,
    ) -> Self {
        let app = app.into();
        let preferences = family.preferences_without_saving_in(config_dir, &app, i18n);
        let needed = !config_dir.join(format!("{app}.conf")).is_file();
        Self::build(family, app, Some(config_dir.to_path_buf()), preferences, needed, wrap)
    }

    fn build(
        family: Family,
        app: String,
        folder: Option<PathBuf>,
        preferences: Preferences,
        needed: bool,
        wrap: impl Fn(SetupMsg) -> Msg + Send + Sync + 'static,
    ) -> Self {
        let mut appearance = Appearance::new(family, app.clone(), preferences).without_saving();
        if let Some(folder) = &folder {
            appearance = appearance.in_folder(folder);
        }
        let font_dirs = crate::icons::default_font_dirs(|name| std::env::var(name).ok());
        let installed = nerd_font::installed_in(&font_dirs);
        Self {
            family,
            app,
            folder,
            appearance,
            step: 0,
            needed,
            install: Install::new(),
            font_dirs,
            installed,
            progress: None,
            wrap: Arc::new(wrap),
            on_finish: None,
            failure: None,
        }
    }

    /// The message the application is sent once the wizard has written the shared keys and made
    /// the application's file: where the application writes its own keys.
    #[must_use]
    pub fn on_finish(mut self, message: Msg) -> Self {
        self.on_finish = Some(message);
        self
    }

    /// Installs the Nerd Font symbols with `install` instead of [`Install::new`], for a test or a
    /// demo that must leave the user's own fonts alone.
    #[must_use]
    pub fn install(mut self, install: Install) -> Self {
        self.install = install;
        self
    }

    /// Looks for a Nerd Font in `dirs` instead of this system's font folders, for a test or a demo.
    #[must_use]
    pub fn font_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.installed = nerd_font::installed_in(&dirs);
        self.font_dirs = dirs;
        self
    }

    /// Whether the wizard is still to be shown: the application has no settings file of its own
    /// and the wizard has not finished.
    #[must_use]
    pub fn needed(&self) -> bool {
        self.needed
    }

    /// The step the wizard is on, counting the appearance step as 0.
    #[must_use]
    pub fn step(&self) -> usize {
        self.step
    }

    /// The shared preferences as the appearance step has them now, before anything is written.
    #[must_use]
    pub fn preferences(&self) -> &Preferences {
        self.appearance.preferences()
    }

    /// Applies `message` and returns the command that shows it: a theme, a language or an icon
    /// mode is applied at once, so the wizard is drawn the way the user just chose. `settings` are
    /// the application's own settings as it holds them in memory; they take every shared key too,
    /// so a later [`Settings::save`] writes what the wizard wrote instead of what the file said
    /// before.
    ///
    /// On [`SetupMsg::Finish`] the three shared keys are written with [`Family::set`], each to the
    /// family's file or the application's own by its box, and the application's file is made. Only
    /// then is the wizard over and the message of [`Setup::on_finish`] sent. A write that fails
    /// leaves the wizard open and says why.
    pub fn update(&mut self, message: SetupMsg, settings: &mut Settings) -> Command<Msg> {
        match message {
            SetupMsg::Appearance(change) => self.appearance.update(change, settings),
            SetupMsg::Install => {
                let wrap = Arc::clone(&self.wrap);
                self.progress = Some(Progress::Downloading { fraction: None });
                // The task is built here, where the language is known: its own thread has none.
                Command::task(self.install.clone().task(move |progress| wrap(SetupMsg::Installing(progress))))
            }
            SetupMsg::Installing(progress) => {
                if matches!(progress, Progress::Done { .. }) {
                    // Look again, so the offer and the samples read what is on disk now.
                    self.installed = nerd_font::installed_in(&self.font_dirs);
                }
                self.progress = Some(progress);
                Command::none()
            }
            SetupMsg::Back => {
                self.step = self.step.saturating_sub(1);
                Command::none()
            }
            SetupMsg::Next => {
                self.step += 1;
                Command::none()
            }
            SetupMsg::Step(step) => {
                // Only a finished step can be gone back to; the steps on top offer no other.
                self.step = step.min(self.step);
                Command::none()
            }
            SetupMsg::Finish => match self.write(settings) {
                Ok(()) => {
                    self.needed = false;
                    self.failure = None;
                    match self.on_finish.clone() {
                        Some(message) => Command::perform(move || message),
                        None => Command::none(),
                    }
                }
                Err(error) => {
                    self.failure = Some(error.to_string());
                    Command::none()
                }
            },
        }
    }

    /// Writes the three shared keys, each where its box says, which makes both files.
    fn write(&self, settings: &mut Settings) -> io::Result<()> {
        for key in Shared::ALL {
            let value = self.preferences().text(key);
            let scope = if self.preferences().source(key) == Source::App { Scope::App } else { Scope::Family };
            // The file says plainly where the value comes from: the value itself, or the family.
            let written = match scope {
                Scope::Family => self.family.id().to_owned(),
                Scope::App => value.clone(),
            };
            settings.set(key.key(), written);
            match &self.folder {
                Some(folder) => self.family.set_in(folder, &self.app, key, &value, scope)?,
                None => self.family.set(&self.app, key, &value, scope)?,
            }
        }
        Ok(())
    }

    fn send(&self, message: SetupMsg) -> Msg {
        (self.wrap)(message)
    }
}

/// The first-run wizard: the appearance step the framework draws and drives, then a step for each
/// one the application adds, on the [`Wizard`] every other flow uses.
///
/// The first step asks for the language, the theme and the icons as
/// [`Appearance`] rows, each with its "In every Quvyta application" box, and shows the same icons
/// in all three glyph modes so the user chooses by eye. Without a Nerd Font on the machine it
/// offers to install the symbols, shows how far the install is and, once it is done, what to look
/// at ([`nerd_font::after_install_text`]). Under the rows, "Start with the defaults" writes what is
/// filled in and ends the wizard.
///
/// Build it in `view` from the [`Setup`] the application holds; see [`Setup`] for the whole of it.
pub struct SetupWizard<'a, Msg> {
    setup: &'a Setup<Msg>,
    steps: Vec<AppStep<'a, Msg>>,
    on_cancel: Option<Msg>,
    page_height: Option<u16>,
}

impl<'a, Msg: Clone + Send + 'static> SetupWizard<'a, Msg> {
    /// The wizard of `setup`, with the appearance step alone.
    #[must_use]
    pub fn new(setup: &'a Setup<Msg>) -> Self {
        Self { setup, steps: Vec::new(), on_cancel: None, page_height: None }
    }

    /// Adds a step of the application's own, named `title`, whose page is built by `page` and
    /// whose messages are the application's. Steps come in the order they are added, after the
    /// appearance step.
    #[must_use]
    pub fn step(mut self, title: impl Into<String>, page: impl FnOnce(&mut View<'_, Msg>) + 'a) -> Self {
        self.steps.push((title.into(), Box::new(page)));
        self
    }

    /// Adds a Cancel button, and makes Esc inside the wizard send `message` too. Closing the
    /// wizard writes nothing at all, so the application usually quits on it and the wizard comes
    /// again next start.
    #[must_use]
    pub fn on_cancel(mut self, message: Msg) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Gives every step exactly `rows` rows, so the buttons stay put between steps.
    #[must_use]
    pub fn page_height(mut self, rows: u16) -> Self {
        self.page_height = Some(rows);
        self
    }

    /// Adds the wizard to `ui`.
    pub fn show<'v>(self, ui: &'v mut View<'_, Msg>) -> NodeMut<'v, Msg> {
        let setup = self.setup;
        let mut labels = vec![crate::t!("quvyta.appearance.heading")];
        let mut pages = Vec::new();
        for (title, page) in self.steps {
            labels.push(title);
            pages.push(page);
        }
        let mut wizard = Wizard::new(labels)
            .current(setup.step)
            .on_back(setup.send(SetupMsg::Back))
            .on_next(setup.send(SetupMsg::Next))
            .on_finish(setup.send(SetupMsg::Finish))
            .on_step({
                let wrap = Arc::clone(&setup.wrap);
                move |step| wrap(SetupMsg::Step(step))
            });
        if let Some(message) = self.on_cancel {
            wizard = wizard.on_cancel(message);
        }
        if let Some(rows) = self.page_height {
            wizard = wizard.page_height(rows);
        }
        wizard.show(ui, |ui| match setup.step.checked_sub(1) {
            None => appearance_step(setup, ui),
            Some(index) => {
                if let Some(page) = pages.into_iter().nth(index) {
                    page(ui);
                }
            }
        })
    }
}

/// The step the framework draws: the shared rows, the glyph samples, the font install and the way
/// out through the defaults.
fn appearance_step<Msg: Clone + Send + 'static>(setup: &Setup<Msg>, ui: &mut View<'_, Msg>) {
    let wrap = Arc::clone(&setup.wrap);
    SettingsList::show(ui, |list| {
        setup.appearance.rows(list, move |change| wrap(SetupMsg::Appearance(change)));
    })
    .fill_width()
    .id("setup-appearance");

    ui.spacer().height(Length::Cells(1));
    ui.add(Text::new(crate::t!("quvyta.setup.sample-hint")).role("secondary")).fill_width();
    for (mode, name) in [(GlyphMode::Nerd, "nerd"), (GlyphMode::Unicode, "unicode"), (GlyphMode::Ascii, "ascii")] {
        ui.row(|ui| {
            let label = crate::t!(&format!("quvyta.appearance.icons-{name}"));
            ui.add(Text::new(label).role("secondary").no_wrap()).width(Length::Cells(SAMPLE_LABEL));
            ui.add(GlyphSample::new(mode)).id(format!("setup-sample-{name}"));
        })
        .fill_width();
    }

    if !setup.installed {
        ui.spacer().height(Length::Cells(1));
        ui.add(Text::new(nerd_font::status_text(false)).role("secondary")).fill_width().id("setup-font-status");
        ui.add(Button::new(crate::t!("quvyta.setup.install")).on_press(setup.send(SetupMsg::Install)))
            .id("setup-install");
    }
    install_progress(setup, ui);
    if let Some(reason) = &setup.failure {
        let mark = ui.env().icons().glyph("warning").into_owned();
        let reason = crate::t!("quvyta.setup.not-saved", reason = reason.as_str());
        ui.add(Text::new(format!("{mark} {reason}")).color("danger")).fill_width().id("setup-failure");
    }
    ui.spacer().height(Length::Cells(1));
    ui.add(Button::new(crate::t!("quvyta.setup.defaults")).on_press(setup.send(SetupMsg::Finish))).id("setup-defaults");
}

/// How far the font install is, and the honest word once it is done.
fn install_progress<Msg: Clone + Send + 'static>(setup: &Setup<Msg>, ui: &mut View<'_, Msg>) {
    let Some(progress) = &setup.progress else { return };
    match progress {
        Progress::Downloading { fraction: Some(fraction) } => {
            ui.add(ProgressBar::new(*fraction).percent(true)).fill_width().id("setup-install-bar");
        }
        Progress::Downloading { fraction: None } | Progress::Verifying | Progress::Installing => {
            ui.add(ProgressBar::indeterminate()).fill_width().id("setup-install-bar");
        }
        Progress::Done { .. } | Progress::Failed(_) => {}
    }
    ui.add(Text::new(progress.text()).role("secondary")).fill_width().id("setup-install-step");
    if matches!(progress, Progress::Done { .. }) {
        ui.add(Text::new(nerd_font::after_install_text())).fill_width().id("setup-after-install");
    }
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
