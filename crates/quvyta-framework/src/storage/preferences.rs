//! The preferences every application of an ecosystem shares: language, theme, icons and reduced
//! motion.
//!
//! The ecosystem's shared file holds one value of each, and each application's own file either
//! names its own value or the ecosystem's id, which means "use the shared one":
//!
//! ```toml
//! # quvyta.conf
//! language = "tr"
//! theme = "monochrome"
//! icons = "nerd"
//! reduced-motion = true
//!
//! # code.conf
//! language = "quvyta"
//! theme = "nordic"
//! reduced-motion = "quvyta"
//! ```
//!
//! [`Ecosystem::preferences`] resolves each key on its own, in this order:
//!
//! 1. the application's value, when it is anything but the ecosystem's id;
//! 2. the shared file's value, when the application's value is the ecosystem's id or the key is
//!    missing from the application's file;
//! 3. the value detected on this machine, when the shared file does not hold one either.
//!
//! [`Ecosystem::set`] changes one key for the whole ecosystem or for one application, and
//! [`Ecosystem::follow`] puts one application back on the ecosystem's value without touching it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{Ecosystem, SettingValue, Settings, atomic_write};
use crate::diagnostics::{Diagnostic, Severity};
use crate::i18n::I18n;
use crate::icons::{GlyphMode, IconMode, default_font_dirs, detect_glyph_mode};
use crate::runtime::Command;

/// The theme a machine starts with: every theme is dark, so there is nothing to detect.
const DETECTED_THEME: &str = "monochrome";

/// The language when the system names none the application speaks.
const FALLBACK_LANGUAGE: &str = "en";

/// A preference every application of an ecosystem shares.
///
/// More may be added in a later release, so a `match` on it needs a `_` arm; iterate
/// [`Shared::ALL`] to list them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Shared {
    /// The language, a locale code such as `tr`.
    Language,
    /// The colour theme, a theme id such as `nordic`.
    Theme,
    /// The icon mode: `auto`, `nerd`, `unicode` or `ascii`.
    Icons,
    /// Reduced motion, `true` or `false`: a need of the person rather than a look of one
    /// application, so it is shared like the language. Written as a boolean; its text form, in
    /// [`Ecosystem::set`], is `true` or `false`.
    ReducedMotion,
}

impl Shared {
    /// Every shared preference, in the order a settings screen lists them.
    pub const ALL: [Self; 4] = [Self::Language, Self::Theme, Self::Icons, Self::ReducedMotion];

    /// The key the preference is written under, in the shared file and in each application's.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Language => Settings::LANGUAGE,
            Self::Theme => Settings::THEME,
            Self::Icons => Settings::ICONS,
            Self::ReducedMotion => Settings::REDUCED_MOTION,
        }
    }

    /// The value under this key in `settings`, in its text form: a boolean as `true` or `false`,
    /// the rest as written. `None` when missing or of a type the key never holds.
    pub(crate) fn read(self, settings: &Settings) -> Option<String> {
        match settings.value(self.key())? {
            SettingValue::Text(text) => Some(text.clone()),
            SettingValue::Bool(flag) if self == Self::ReducedMotion => Some(flag.to_string()),
            _ => None,
        }
    }

    /// `text` as it is written under this key: reduced motion's `true` and `false` as booleans,
    /// every other text, the ecosystem's id among them, as text.
    pub(crate) fn setting(self, text: &str) -> SettingValue {
        match (self, text.parse::<bool>()) {
            (Self::ReducedMotion, Ok(flag)) => SettingValue::Bool(flag),
            _ => SettingValue::Text(text.to_owned()),
        }
    }
}

/// Where [`Ecosystem::set`] writes a change: the "In every Quvyta application" choice of a settings
/// screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// The shared file takes the value and the application follows it again, so every
    /// application that follows the ecosystem changes with it. Applications that chose their own
    /// value keep it.
    Ecosystem,
    /// Only the application's own file takes the value; the shared file is left alone.
    App,
}

impl Scope {
    /// The former name of [`Scope::Ecosystem`], still working so applications can move over; a
    /// later release marks it deprecated. New code uses [`Scope::Ecosystem`].
    #[allow(non_upper_case_globals)]
    pub const Family: Scope = Scope::Ecosystem;
}

/// Where a resolved preference came from, so a settings screen can say "follows every Quvyta
/// application" or "only here".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// The application's own file names it.
    App,
    /// The ecosystem's shared file holds it and the application follows it.
    Ecosystem,
    /// Neither file holds it; it was detected on this machine.
    Detected,
}

impl Source {
    /// The former name of [`Source::Ecosystem`], still working so applications can move over; a
    /// later release marks it deprecated. New code uses [`Source::Ecosystem`].
    #[allow(non_upper_case_globals)]
    pub const Family: Source = Source::Ecosystem;
}

/// A preference's value together with where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved<T> {
    /// The value to use.
    pub value: T,
    /// Where the value came from.
    pub source: Source,
}

/// The shared preferences as one application sees them, from [`Ecosystem::preferences`].
///
/// ```
/// use qframe::i18n::I18n;
/// use qframe::storage::{Ecosystem, Source};
///
/// # let folder = std::env::temp_dir().join(format!("quvyta-preferences-doc-{}", std::process::id()));
/// // An application calls `Ecosystem::QUVYTA.preferences("code", &i18n)`; the example keeps to a
/// // folder of its own.
/// let prefs = Ecosystem::QUVYTA.preferences_in(&folder, "code", &I18n::builtin());
/// let theme = prefs.theme();
/// assert_eq!((theme.value.as_str(), theme.source), ("monochrome", Source::Detected), "never detected");
/// assert!(folder.join("quvyta.conf").is_file(), "the first start writes the shared file");
/// # std::fs::remove_dir_all(&folder).ok();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Preferences {
    language: Resolved<String>,
    theme: Resolved<String>,
    icons: Resolved<IconMode>,
    reduced_motion: Resolved<bool>,
    update_notice: bool,
    diagnostics: Vec<Diagnostic>,
}

impl Preferences {
    /// Whether the ecosystem's applications say when a newer version of themselves is out; see
    /// [`Ecosystem::update_notice`]. One switch for the whole ecosystem, on unless it was turned off.
    #[must_use]
    pub fn update_notice(&self) -> bool {
        self.update_notice
    }

    /// Records that the update notice is now `on`, after a change written with
    /// [`Ecosystem::set_update_notice`].
    pub(crate) fn record_update_notice(&mut self, on: bool) {
        self.update_notice = on;
    }

    /// The locale code to speak.
    #[must_use]
    pub fn language(&self) -> &Resolved<String> {
        &self.language
    }

    /// The theme id to draw with.
    #[must_use]
    pub fn theme(&self) -> &Resolved<String> {
        &self.theme
    }

    /// The icon mode to draw with.
    #[must_use]
    pub fn icons(&self) -> &Resolved<IconMode> {
        &self.icons
    }

    /// Whether to reduce motion. The `QUVYTA_REDUCED_MOTION` environment variable, when set,
    /// still decides over it in the running application; see
    /// [`Env::reduced_motion`](crate::env::Env::reduced_motion).
    #[must_use]
    pub fn reduced_motion(&self) -> &Resolved<bool> {
        &self.reduced_motion
    }

    /// Where `key` came from.
    #[must_use]
    pub fn source(&self, key: Shared) -> Source {
        match key {
            Shared::Language => self.language.source,
            Shared::Theme => self.theme.source,
            Shared::Icons => self.icons.source,
            Shared::ReducedMotion => self.reduced_motion.source,
        }
    }

    /// Records that `key` now holds `value`, which came from `source`, after a change written
    /// with [`Ecosystem::set`].
    pub(crate) fn record(&mut self, key: Shared, value: &str, source: Source) {
        match key {
            Shared::Language => self.language = Resolved { value: value.to_owned(), source },
            Shared::Theme => self.theme = Resolved { value: value.to_owned(), source },
            Shared::Icons => {
                let mode = IconMode::from_name(value).unwrap_or(self.icons.value);
                self.icons = Resolved { value: mode, source };
            }
            Shared::ReducedMotion => {
                let reduced = value.parse().unwrap_or(self.reduced_motion.value);
                self.reduced_motion = Resolved { value: reduced, source };
            }
        }
    }

    /// The value of `key` as it is written in a file.
    pub(crate) fn text(&self, key: Shared) -> String {
        match key {
            Shared::Language => self.language.value.clone(),
            Shared::Theme => self.theme.value.clone(),
            Shared::Icons => self.icons.value.name().to_owned(),
            Shared::ReducedMotion => self.reduced_motion.value.to_string(),
        }
    }

    /// Problems found on the way: a broken line in the shared file, a shared file that could not
    /// be written. Each is located where a file is to blame; the key it concerns fell back to the
    /// detected value. Problems in the application's own file are reported by the application's
    /// own [`Settings`] load and are not repeated here.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Commands that switch the running application to the resolved language, theme, icons and
    /// reduced motion, for use after a change in `update`. At start give the preferences to
    /// [`Runtime::preferences`](crate::runtime::Runtime::preferences) instead, so the first
    /// frame is already drawn with them.
    #[must_use]
    pub fn apply<Msg: Send + 'static>(&self) -> Command<Msg> {
        Command::batch([
            Command::set_theme(self.theme.value.clone()),
            Command::set_locale(self.language.value.clone()),
            Command::set_icon_mode(self.icons.value),
            Command::set_reduced_motion(self.reduced_motion.value),
        ])
    }
}

/// What a resolution does about a shared file that is not there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Missing {
    /// Write it with the detected values, so the next application finds them.
    Create,
    /// Leave it missing and detect its keys, so nothing is written before a setup wizard finishes.
    Leave,
}

/// What this machine would choose for each shared preference.
struct Detected {
    language: String,
    icons: IconMode,
}

impl Detected {
    fn on_this_machine(i18n: &I18n, lookup: impl Fn(&str) -> Option<String>, font_dirs: &[PathBuf]) -> Self {
        let language = i18n.detect(&lookup).unwrap_or_else(|| FALLBACK_LANGUAGE.to_owned());
        let icons = match detect_glyph_mode(IconMode::Auto, &lookup, font_dirs) {
            GlyphMode::Nerd => IconMode::Nerd,
            GlyphMode::Unicode => IconMode::Unicode,
            GlyphMode::Ascii => IconMode::Ascii,
        };
        Self { language, icons }
    }

    /// The detected value of `key` as it is written in a file.
    fn text(&self, key: Shared) -> String {
        match key {
            Shared::Language => self.language.clone(),
            Shared::Theme => DETECTED_THEME.to_owned(),
            Shared::Icons => self.icons.name().to_owned(),
            Shared::ReducedMotion => false.to_string(),
        }
    }

    /// The shared file as it is first written: the detected value of language, theme and icons.
    /// Reduced motion is left out until someone chooses it, as in every shared file written before
    /// it was shared; missing, it reads as motion.
    fn file(&self) -> String {
        let mut settings = Settings::in_memory();
        for key in [Shared::Language, Shared::Theme, Shared::Icons] {
            settings.set(key.key(), self.text(key));
        }
        settings.to_toml()
    }
}

impl Ecosystem {
    /// Resolves the shared preferences of application `app`: for each of language, theme, icons
    /// and reduced motion, the application's own value when its file names one other than the ecosystem's id,
    /// else the value in the [shared file](Self::shared_file), else the value detected on this
    /// machine. A key missing from the application's file follows the ecosystem, as the ecosystem's id
    /// does, so a file written by hand before the ecosystem shared anything follows it too.
    ///
    /// Detection reads the environment: the language as [`I18n::detect`] finds it among the
    /// languages `i18n` knows (English when it knows none of the system's), the theme always
    /// `monochrome`, the icons as the strongest set the terminal and the installed fonts allow
    /// ([`detect_glyph_mode`]), reduced motion always off.
    ///
    /// When the shared file does not exist it is created with the detected values, so the next
    /// application that starts finds them. A broken line never stops anything: that key falls
    /// back to the detected value and the reason, located at file, line and column, is in
    /// [`Preferences::diagnostics`]. Without a home folder nothing is read or written and every
    /// value is detected.
    ///
    /// The rest of the application's settings are read as before, with
    /// [`Settings::load_member`]; this only resolves the keys [`Shared`] names.
    #[must_use]
    pub fn preferences(&self, app: &str, i18n: &I18n) -> Preferences {
        let lookup = |name: &str| std::env::var(name).ok();
        let font_dirs = default_font_dirs(lookup);
        let detected = Detected::on_this_machine(i18n, lookup, &font_dirs);
        match self.config_dir() {
            Some(dir) => self.resolve(&dir, app, &detected, Missing::Create),
            None => {
                let mut prefs = resolved_from(&detected, |_| None, |_| None);
                prefs
                    .diagnostics
                    .push(Diagnostic::warning(None, "no config directory found; preferences are not saved"));
                prefs
            }
        }
    }

    /// [`preferences`](Self::preferences) with `config_dir` as the ecosystem's folder instead of
    /// this platform's, for a test or a demo that must leave the user's own files alone.
    #[must_use]
    pub fn preferences_in(&self, config_dir: &Path, app: &str, i18n: &I18n) -> Preferences {
        let lookup = |name: &str| std::env::var(name).ok();
        let detected = Detected::on_this_machine(i18n, lookup, &default_font_dirs(lookup));
        self.resolve(config_dir, app, &detected, Missing::Create)
    }

    /// [`preferences`](Self::preferences) without writing anything: a missing shared file is left
    /// missing and its keys are detected instead.
    ///
    /// For an application whose first start shows a [setup wizard](crate::widgets::Setup): a
    /// wizard closed half-way leaves the user's settings folder as empty as it found it, and the
    /// wizard's Finish writes both files. An application without a wizard uses
    /// [`preferences`](Self::preferences), so the first application to start leaves the shared
    /// file for the next one.
    #[must_use]
    pub fn preferences_without_saving(&self, app: &str, i18n: &I18n) -> Preferences {
        let lookup = |name: &str| std::env::var(name).ok();
        let font_dirs = default_font_dirs(lookup);
        let detected = Detected::on_this_machine(i18n, lookup, &font_dirs);
        match self.config_dir() {
            Some(dir) => self.resolve(&dir, app, &detected, Missing::Leave),
            None => resolved_from(&detected, |_| None, |_| None),
        }
    }

    /// [`preferences_without_saving`](Self::preferences_without_saving) with `config_dir` as the
    /// ecosystem's folder instead of this platform's, for a test or a demo.
    #[must_use]
    pub fn preferences_without_saving_in(&self, config_dir: &Path, app: &str, i18n: &I18n) -> Preferences {
        let lookup = |name: &str| std::env::var(name).ok();
        let detected = Detected::on_this_machine(i18n, lookup, &default_font_dirs(lookup));
        self.resolve(config_dir, app, &detected, Missing::Leave)
    }

    /// [`preferences_in`](Self::preferences_in) with the machine's detection read through
    /// `lookup` and `font_dirs` instead of the process environment, for tests.
    #[cfg(test)]
    fn preferences_detecting(
        &self,
        config_dir: &Path,
        app: &str,
        i18n: &I18n,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Preferences {
        self.resolve(config_dir, app, &Detected::on_this_machine(i18n, lookup, &[]), Missing::Create)
    }

    /// Changes shared preference `key` of application `app` to `value`, for the whole ecosystem or
    /// for the application alone:
    ///
    /// | `scope` | shared file | application's file |
    /// |---|---|---|
    /// | [`Scope::Ecosystem`] | `key = value` | `key = "<ecosystem id>"` |
    /// | [`Scope::App`] | unchanged | `key = value` |
    ///
    /// Each file is read from disk right before it is written and only `key` changes in it, so
    /// two applications changing preferences at the same time both keep their change instead
    /// of one writing back what it read earlier. On Unix systems the ecosystem's folder is held
    /// with an advisory lock from the reading to the writing, so even two changes in the same
    /// instant follow one another; elsewhere the window between them is a few microseconds. Files are written with [`atomic_write`]; the
    /// other keys stay as they were, though comments do not survive, as with
    /// [`Settings::save`]. The running application is not switched; use
    /// [`Preferences::apply`] or the matching [`Command`] for that.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`io::ErrorKind::InvalidInput`] when `value` is not a valid
    /// value of `key` (an unknown icon mode, the ecosystem's own id, an empty text), of kind
    /// [`io::ErrorKind::NotFound`] when there is no home folder, and any error from writing.
    pub fn set(&self, app: &str, key: Shared, value: &str, scope: Scope) -> io::Result<()> {
        match self.config_dir() {
            Some(dir) => self.set_in(&dir, app, key, value, scope),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no config directory found")),
        }
    }

    /// [`set`](Self::set) with `config_dir` as the ecosystem's folder instead of this platform's.
    ///
    /// # Errors
    ///
    /// As [`set`](Self::set), except that there is always a folder.
    pub fn set_in(&self, config_dir: &Path, app: &str, key: Shared, value: &str, scope: Scope) -> io::Result<()> {
        let value = self.checked(key, value)?;
        fs::create_dir_all(config_dir)?;
        let _held = hold_folder(config_dir)?;
        let app_file = config_dir.join(super::ecosystem::file_name(app));
        match scope {
            Scope::Ecosystem => {
                let shared_file = config_dir.join(super::ecosystem::file_name(self.id()));
                rewrite(&shared_file, key.key(), key.setting(&value), None)?;
                rewrite(&app_file, key.key(), SettingValue::Text(self.id().to_owned()), Some(self))
            }
            Scope::App => rewrite(&app_file, key.key(), key.setting(&value), Some(self)),
        }
    }

    /// Puts application `app` back on the ecosystem's value of `key`: its own file says the ecosystem's
    /// id and the [shared file](Self::shared_file) is neither read nor written, so the next
    /// resolution answers the shared value with [`Source::Ecosystem`] and no other application
    /// changes. The one way back from a value of an application's own, for the settings screen
    /// that lists every member of the ecosystem: "follow the shared setting" on one member's cell
    /// must not change what the whole ecosystem draws with.
    ///
    /// The file is read from disk right before it is written and only `key` changes in it, as
    /// [`set`](Self::set) does, with the ecosystem's folder held by an advisory lock on Unix. A
    /// missing file is created holding that one key. A key that already follows the ecosystem is
    /// left alone, file and all. Comments do not survive a change, as with [`Settings::save`].
    /// The running application is not switched; use [`Preferences::apply`] for that.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`io::ErrorKind::NotFound`] when there is no home folder, of kind
    /// [`io::ErrorKind::InvalidData`] when the application's file cannot be read as settings,
    /// and any error from writing. A file that could not be read is left exactly as it was.
    pub fn follow(&self, app: &str, key: Shared) -> io::Result<()> {
        match self.config_dir() {
            Some(dir) => self.follow_in(&dir, app, key),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no config directory found")),
        }
    }

    /// [`follow`](Self::follow) with `config_dir` as the ecosystem's folder instead of this
    /// platform's, for a test or a demo that must leave the user's own files alone.
    ///
    /// ```
    /// use qframe::storage::{Ecosystem, Shared};
    ///
    /// # let folder = std::env::temp_dir().join(format!("quvyta-follow-doc-{}", std::process::id()));
    /// # std::fs::create_dir_all(&folder).expect("folder");
    /// std::fs::write(folder.join("code.conf"), "theme = \"amber\"\n").expect("the file");
    /// Ecosystem::QUVYTA.follow_in(&folder, "code", Shared::Theme).expect("follow");
    /// assert_eq!(std::fs::read_to_string(folder.join("code.conf")).expect("read"), "theme = \"quvyta\"\n");
    /// assert!(!folder.join("quvyta.conf").exists(), "the shared file is left alone");
    /// # std::fs::remove_dir_all(&folder).ok();
    /// ```
    ///
    /// # Errors
    ///
    /// As [`follow`](Self::follow), except that there is always a folder.
    pub fn follow_in(&self, config_dir: &Path, app: &str, key: Shared) -> io::Result<()> {
        fs::create_dir_all(config_dir)?;
        let _held = hold_folder(config_dir)?;
        let path = config_dir.join(super::ecosystem::file_name(app));
        let mut settings = Settings::open(&path).member_of(self);
        if let Some(problem) = settings.diagnostics().iter().find(|problem| problem.severity == Severity::Error) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, problem.to_string()));
        }
        let value = SettingValue::Text(self.id().to_owned());
        if settings.value(key.key()) == Some(&value) && path.exists() {
            return Ok(());
        }
        settings.store(key.key(), value);
        settings.save()
    }

    /// Changes `key` in application `app`'s own file in `config_dir` to `value`, the way
    /// [`set_in`](Self::set_in) changes a shared key: read right before writing, only that key,
    /// with the folder held. For the application's settings that sit beside the shared ones on an
    /// appearance screen, such as reduced motion.
    pub(crate) fn set_own_in(&self, config_dir: &Path, app: &str, key: &str, value: SettingValue) -> io::Result<()> {
        fs::create_dir_all(config_dir)?;
        let _held = hold_folder(config_dir)?;
        rewrite(&config_dir.join(super::ecosystem::file_name(app)), key, value, Some(self))
    }

    /// `value` as it is written under `key`, or why it cannot be.
    fn checked(&self, key: Shared, value: &str) -> io::Result<String> {
        let invalid = |why: String| io::Error::new(io::ErrorKind::InvalidInput, why);
        let value = value.trim();
        if value.is_empty() {
            return Err(invalid(format!("`{}` cannot be empty", key.key())));
        }
        if value == self.id() {
            return Err(invalid(format!("`{}` cannot be set to the ecosystem's own id `{value}`", key.key())));
        }
        match key {
            Shared::Icons => IconMode::from_name(value)
                .map(|mode| mode.name().to_owned())
                .ok_or_else(|| invalid(format!("`{value}` is not an icon mode; use auto, nerd, unicode or ascii"))),
            Shared::ReducedMotion => value
                .parse::<bool>()
                .map(|reduced| reduced.to_string())
                .map_err(|_| invalid(format!("`{value}` is not a value of reduced motion; use true or false"))),
            Shared::Language | Shared::Theme => Ok(value.to_owned()),
        }
    }

    /// Resolves the preferences of `app` from the files in `config_dir`, creating the shared file
    /// with the `detected` values when it is missing and `missing` says to.
    fn resolve(&self, config_dir: &Path, app: &str, detected: &Detected, missing: Missing) -> Preferences {
        let mut diagnostics = Vec::new();
        let shared_path = config_dir.join(super::ecosystem::file_name(self.id()));
        let shared = if shared_path.exists() {
            let shared = Settings::open(&shared_path);
            diagnostics.extend(shared.diagnostics().iter().cloned());
            Some(shared)
        } else {
            if missing == Missing::Create
                && let Err(error) = create(&shared_path, &detected.file())
            {
                diagnostics.push(Diagnostic::error(
                    None,
                    format!("{}: shared preferences not saved: {error}", shared_path.display()),
                ));
            }
            None
        };
        let own = Settings::open(config_dir.join(super::ecosystem::file_name(app))).member_of(self);
        let ecosystem_value = |key: Shared| -> Option<String> {
            let shared = shared.as_ref()?;
            let text = key.read(shared).filter(|text| valid(key, text))?;
            (text != self.id()).then_some(text)
        };
        let app_value =
            |key: Shared| -> Option<String> { key.read(&own).filter(|text| text != self.id() && valid(key, text)) };
        if let Some(shared) = &shared {
            for key in Shared::ALL {
                if key.read(shared).is_some_and(|text| text == self.id()) {
                    diagnostics.push(Diagnostic::warning(
                        shared.origin(key.key()),
                        format!(
                            "`{}` cannot follow the ecosystem in the ecosystem's own file; the detected value is used",
                            key.key()
                        ),
                    ));
                }
            }
        }
        let mut prefs = resolved_from(detected, app_value, ecosystem_value);
        prefs.update_notice = shared.as_ref().is_none_or(super::update_notice::from_shared);
        prefs.diagnostics = diagnostics;
        prefs
    }
}

/// Whether `text` is a usable value of `key`; what is not was already reported by the settings
/// load that read it.
fn valid(key: Shared, text: &str) -> bool {
    match key {
        Shared::Icons => IconMode::from_name(text).is_some(),
        Shared::ReducedMotion => text.parse::<bool>().is_ok(),
        Shared::Language | Shared::Theme => !text.trim().is_empty(),
    }
}

/// Each key from the application's value, the ecosystem's value or the detected one, in that order.
fn resolved_from(
    detected: &Detected,
    app_value: impl Fn(Shared) -> Option<String>,
    ecosystem_value: impl Fn(Shared) -> Option<String>,
) -> Preferences {
    let text = |key: Shared| -> Resolved<String> {
        if let Some(value) = app_value(key) {
            Resolved { value, source: Source::App }
        } else if let Some(value) = ecosystem_value(key) {
            Resolved { value, source: Source::Ecosystem }
        } else {
            Resolved { value: detected.text(key), source: Source::Detected }
        }
    };
    let icons = text(Shared::Icons);
    let reduced = text(Shared::ReducedMotion);
    Preferences {
        language: text(Shared::Language),
        theme: text(Shared::Theme),
        icons: Resolved { value: IconMode::from_name(&icons.value).unwrap_or(detected.icons), source: icons.source },
        reduced_motion: Resolved { value: reduced.value.parse().unwrap_or(false), source: reduced.source },
        update_notice: true,
        diagnostics: Vec::new(),
    }
}

/// Holds the ecosystem's folder with an advisory lock while it lives, so no other writer reads a file
/// between this writer's reading and writing it. Locking the folder rather than a file of its own
/// leaves nothing behind in the user's settings. Unix only: elsewhere the framework has no
/// advisory lock, as for [`AppLock`](super::AppLock).
#[cfg(unix)]
pub(super) fn hold_folder(dir: &Path) -> io::Result<Option<fs::File>> {
    let folder = fs::File::open(dir)?;
    folder.lock()?;
    Ok(Some(folder))
}

#[cfg(not(unix))]
pub(super) fn hold_folder(_dir: &Path) -> io::Result<Option<fs::File>> {
    Ok(None)
}

/// Writes the first shared file, creating its folder.
fn create(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    atomic_write(path, text.as_bytes())
}

/// Reads the file at `path` as it is on disk now, changes only `key` to `value` and writes it
/// back. `ecosystem` marks an application's file, whose own values follow the ecosystem.
pub(super) fn rewrite(path: &Path, key: &str, value: SettingValue, ecosystem: Option<&Ecosystem>) -> io::Result<()> {
    let mut settings = Settings::open(path);
    if let Some(ecosystem) = ecosystem {
        settings = settings.member_of(ecosystem);
    }
    if settings.value(key) == Some(&value) && path.exists() {
        return Ok(());
    }
    settings.store(key, value);
    settings.save()
}

#[cfg(test)]
#[path = "preferences_tests.rs"]
mod tests;
