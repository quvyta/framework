//! The preferences every application of a family shares: language, theme and icons.
//!
//! The family's shared file holds one value of each, and each application's own file either
//! names its own value or the family's id, which means "use the shared one":
//!
//! ```toml
//! # quvyta.conf
//! language = "tr"
//! theme = "monochrome"
//! icons = "nerd"
//!
//! # code.conf
//! language = "quvyta"
//! theme = "nordic"
//! ```
//!
//! [`Family::preferences`] resolves each key on its own, in this order:
//!
//! 1. the application's value, when it is anything but the family's id;
//! 2. the shared file's value, when the application's value is the family's id or the key is
//!    missing from the application's file;
//! 3. the value detected on this machine, when the shared file does not hold one either.
//!
//! [`Family::set`] changes one key for the whole family or for one application.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{Family, SettingValue, Settings, atomic_write};
use crate::diagnostics::Diagnostic;
use crate::i18n::I18n;
use crate::icons::{GlyphMode, IconMode, default_font_dirs, detect_glyph_mode};
use crate::runtime::Command;

/// The theme a machine starts with: every theme is dark, so there is nothing to detect.
const DETECTED_THEME: &str = "monochrome";

/// The language when the system names none the application speaks.
const FALLBACK_LANGUAGE: &str = "en";

/// A preference every application of a family shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shared {
    /// The language, a locale code such as `tr`.
    Language,
    /// The colour theme, a theme id such as `nordic`.
    Theme,
    /// The icon mode: `auto`, `nerd`, `unicode` or `ascii`.
    Icons,
}

impl Shared {
    /// Every shared preference, in the order a settings screen lists them.
    pub const ALL: [Self; 3] = [Self::Language, Self::Theme, Self::Icons];

    /// The key the preference is written under, in the shared file and in each application's.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Language => Settings::LANGUAGE,
            Self::Theme => Settings::THEME,
            Self::Icons => Settings::ICONS,
        }
    }
}

/// Where [`Family::set`] writes a change: the "In every Quvyta application" choice of a settings
/// screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// The shared file takes the value and the application follows it again, so every
    /// application that follows the family changes with it. Applications that chose their own
    /// value keep it.
    Family,
    /// Only the application's own file takes the value; the shared file is left alone.
    App,
}

/// Where a resolved preference came from, so a settings screen can say "follows every Quvyta
/// application" or "only here".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// The application's own file names it.
    App,
    /// The family's shared file holds it and the application follows it.
    Family,
    /// Neither file holds it; it was detected on this machine.
    Detected,
}

/// A preference's value together with where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved<T> {
    /// The value to use.
    pub value: T,
    /// Where the value came from.
    pub source: Source,
}

/// The shared preferences as one application sees them, from [`Family::preferences`].
///
/// ```
/// use qframe::i18n::I18n;
/// use qframe::storage::{Family, Source};
///
/// # let folder = std::env::temp_dir().join(format!("quvyta-preferences-doc-{}", std::process::id()));
/// // An application calls `Family::QUVYTA.preferences("code", &i18n)`; the example keeps to a
/// // folder of its own.
/// let prefs = Family::QUVYTA.preferences_in(&folder, "code", &I18n::builtin());
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
    diagnostics: Vec<Diagnostic>,
}

impl Preferences {
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

    /// Where `key` came from.
    #[must_use]
    pub fn source(&self, key: Shared) -> Source {
        match key {
            Shared::Language => self.language.source,
            Shared::Theme => self.theme.source,
            Shared::Icons => self.icons.source,
        }
    }

    /// Records that `key` now holds `value`, which came from `source`, after a change written
    /// with [`Family::set`].
    pub(crate) fn record(&mut self, key: Shared, value: &str, source: Source) {
        match key {
            Shared::Language => self.language = Resolved { value: value.to_owned(), source },
            Shared::Theme => self.theme = Resolved { value: value.to_owned(), source },
            Shared::Icons => {
                let mode = IconMode::from_name(value).unwrap_or(self.icons.value);
                self.icons = Resolved { value: mode, source };
            }
        }
    }

    /// The value of `key` as it is written in a file.
    pub(crate) fn text(&self, key: Shared) -> String {
        match key {
            Shared::Language => self.language.value.clone(),
            Shared::Theme => self.theme.value.clone(),
            Shared::Icons => self.icons.value.name().to_owned(),
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

    /// Commands that switch the running application to the resolved language, theme and icons,
    /// for use after a change in `update`. At start give the preferences to
    /// [`Runtime::preferences`](crate::runtime::Runtime::preferences) instead, so the first
    /// frame is already drawn with them.
    #[must_use]
    pub fn apply<Msg: Send + 'static>(&self) -> Command<Msg> {
        Command::batch([
            Command::set_theme(self.theme.value.clone()),
            Command::set_locale(self.language.value.clone()),
            Command::set_icon_mode(self.icons.value),
        ])
    }
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
        }
    }

    /// The shared file as it is first written: the detected value of every key.
    fn file(&self) -> String {
        let mut settings = Settings::in_memory();
        for key in Shared::ALL {
            settings.set(key.key(), self.text(key));
        }
        settings.to_toml()
    }
}

impl Family {
    /// Resolves the shared preferences of application `app`: for each of language, theme and
    /// icons, the application's own value when its file names one other than the family's id,
    /// else the value in the [shared file](Self::shared_file), else the value detected on this
    /// machine. A key missing from the application's file follows the family, as the family's id
    /// does, so a file written by hand before the family shared anything follows it too.
    ///
    /// Detection reads the environment: the language as [`I18n::detect`] finds it among the
    /// languages `i18n` knows (English when it knows none of the system's), the theme always
    /// `monochrome`, the icons as the strongest set the terminal and the installed fonts allow
    /// ([`detect_glyph_mode`]).
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
            Some(dir) => self.resolve(&dir, app, &detected),
            None => {
                let mut prefs = resolved_from(&detected, |_| None, |_| None);
                prefs
                    .diagnostics
                    .push(Diagnostic::warning(None, "no config directory found; preferences are not saved"));
                prefs
            }
        }
    }

    /// [`preferences`](Self::preferences) with `config_dir` as the family's folder instead of
    /// this platform's, for a test or a demo that must leave the user's own files alone.
    #[must_use]
    pub fn preferences_in(&self, config_dir: &Path, app: &str, i18n: &I18n) -> Preferences {
        let lookup = |name: &str| std::env::var(name).ok();
        let detected = Detected::on_this_machine(i18n, lookup, &default_font_dirs(lookup));
        self.resolve(config_dir, app, &detected)
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
        self.resolve(config_dir, app, &Detected::on_this_machine(i18n, lookup, &[]))
    }

    /// Changes shared preference `key` of application `app` to `value`, for the whole family or
    /// for the application alone:
    ///
    /// | `scope` | shared file | application's file |
    /// |---|---|---|
    /// | [`Scope::Family`] | `key = value` | `key = "<family id>"` |
    /// | [`Scope::App`] | unchanged | `key = value` |
    ///
    /// Each file is read from disk right before it is written and only `key` changes in it, so
    /// two applications changing preferences at the same time both keep their change instead
    /// of one writing back what it read earlier. On Unix systems the family's folder is held
    /// with an advisory lock from the reading to the writing, so even two changes in the same
    /// instant follow one another; elsewhere the window between them is a few microseconds. Files are written with [`atomic_write`]; the
    /// other keys stay as they were, though comments do not survive, as with
    /// [`Settings::save`]. The running application is not switched; use
    /// [`Preferences::apply`] or the matching [`Command`] for that.
    ///
    /// # Errors
    ///
    /// Returns an error of kind [`io::ErrorKind::InvalidInput`] when `value` is not a valid
    /// value of `key` (an unknown icon mode, the family's own id, an empty text), of kind
    /// [`io::ErrorKind::NotFound`] when there is no home folder, and any error from writing.
    pub fn set(&self, app: &str, key: Shared, value: &str, scope: Scope) -> io::Result<()> {
        match self.config_dir() {
            Some(dir) => self.set_in(&dir, app, key, value, scope),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no config directory found")),
        }
    }

    /// [`set`](Self::set) with `config_dir` as the family's folder instead of this platform's.
    ///
    /// # Errors
    ///
    /// As [`set`](Self::set), except that there is always a folder.
    pub fn set_in(&self, config_dir: &Path, app: &str, key: Shared, value: &str, scope: Scope) -> io::Result<()> {
        let value = self.checked(key, value)?;
        fs::create_dir_all(config_dir)?;
        let _held = hold_folder(config_dir)?;
        let app_file = config_dir.join(super::family::file_name(app));
        match scope {
            Scope::Family => {
                let shared_file = config_dir.join(super::family::file_name(self.id()));
                rewrite(&shared_file, key.key(), SettingValue::Text(value), None)?;
                rewrite(&app_file, key.key(), SettingValue::Text(self.id().to_owned()), Some(self))
            }
            Scope::App => rewrite(&app_file, key.key(), SettingValue::Text(value), Some(self)),
        }
    }

    /// Changes `key` in application `app`'s own file in `config_dir` to `value`, the way
    /// [`set_in`](Self::set_in) changes a shared key: read right before writing, only that key,
    /// with the folder held. For the application's settings that sit beside the shared ones on an
    /// appearance screen, such as reduced motion.
    pub(crate) fn set_own_in(&self, config_dir: &Path, app: &str, key: &str, value: SettingValue) -> io::Result<()> {
        fs::create_dir_all(config_dir)?;
        let _held = hold_folder(config_dir)?;
        rewrite(&config_dir.join(super::family::file_name(app)), key, value, Some(self))
    }

    /// `value` as it is written under `key`, or why it cannot be.
    fn checked(&self, key: Shared, value: &str) -> io::Result<String> {
        let invalid = |why: String| io::Error::new(io::ErrorKind::InvalidInput, why);
        let value = value.trim();
        if value.is_empty() {
            return Err(invalid(format!("`{}` cannot be empty", key.key())));
        }
        if value == self.id() {
            return Err(invalid(format!("`{}` cannot be set to the family's own id `{value}`", key.key())));
        }
        match key {
            Shared::Icons => IconMode::from_name(value)
                .map(|mode| mode.name().to_owned())
                .ok_or_else(|| invalid(format!("`{value}` is not an icon mode; use auto, nerd, unicode or ascii"))),
            Shared::Language | Shared::Theme => Ok(value.to_owned()),
        }
    }

    /// Resolves the preferences of `app` from the files in `config_dir`, creating the shared file
    /// with the `detected` values when it is missing.
    fn resolve(&self, config_dir: &Path, app: &str, detected: &Detected) -> Preferences {
        let mut diagnostics = Vec::new();
        let shared_path = config_dir.join(super::family::file_name(self.id()));
        let shared = if shared_path.exists() {
            let shared = Settings::open(&shared_path);
            diagnostics.extend(shared.diagnostics().iter().cloned());
            Some(shared)
        } else {
            if let Err(error) = create(&shared_path, &detected.file()) {
                diagnostics.push(Diagnostic::error(
                    None,
                    format!("{}: shared preferences not saved: {error}", shared_path.display()),
                ));
            }
            None
        };
        let own = Settings::open(config_dir.join(super::family::file_name(app))).member_of(self);
        let family_value = |key: Shared| -> Option<String> {
            let shared = shared.as_ref()?;
            let text = shared.get::<String>(key.key()).filter(|text| valid(key, text))?;
            (text != self.id()).then_some(text)
        };
        let app_value = |key: Shared| -> Option<String> {
            own.get::<String>(key.key()).filter(|text| text != self.id() && valid(key, text))
        };
        if let Some(shared) = &shared {
            for key in Shared::ALL {
                if shared.get::<String>(key.key()).is_some_and(|text| text == self.id()) {
                    diagnostics.push(Diagnostic::warning(
                        shared.origin(key.key()),
                        format!(
                            "`{}` cannot follow the family in the family's own file; the detected value is used",
                            key.key()
                        ),
                    ));
                }
            }
        }
        let mut prefs = resolved_from(detected, app_value, family_value);
        prefs.diagnostics = diagnostics;
        prefs
    }
}

/// Whether `text` is a usable value of `key`; what is not was already reported by the settings
/// load that read it.
fn valid(key: Shared, text: &str) -> bool {
    match key {
        Shared::Icons => IconMode::from_name(text).is_some(),
        Shared::Language | Shared::Theme => !text.trim().is_empty(),
    }
}

/// Each key from the application's value, the family's value or the detected one, in that order.
fn resolved_from(
    detected: &Detected,
    app_value: impl Fn(Shared) -> Option<String>,
    family_value: impl Fn(Shared) -> Option<String>,
) -> Preferences {
    let text = |key: Shared| -> Resolved<String> {
        if let Some(value) = app_value(key) {
            Resolved { value, source: Source::App }
        } else if let Some(value) = family_value(key) {
            Resolved { value, source: Source::Family }
        } else {
            Resolved { value: detected.text(key), source: Source::Detected }
        }
    };
    let icons = text(Shared::Icons);
    Preferences {
        language: text(Shared::Language),
        theme: text(Shared::Theme),
        icons: Resolved { value: IconMode::from_name(&icons.value).unwrap_or(detected.icons), source: icons.source },
        diagnostics: Vec::new(),
    }
}

/// Holds the family's folder with an advisory lock while it lives, so no other writer reads a file
/// between this writer's reading and writing it. Locking the folder rather than a file of its own
/// leaves nothing behind in the user's settings. Unix only: elsewhere the framework has no
/// advisory lock, as for [`AppLock`](super::AppLock).
#[cfg(unix)]
fn hold_folder(dir: &Path) -> io::Result<Option<fs::File>> {
    let folder = fs::File::open(dir)?;
    folder.lock()?;
    Ok(Some(folder))
}

#[cfg(not(unix))]
fn hold_folder(_dir: &Path) -> io::Result<Option<fs::File>> {
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
/// back. `family` marks an application's file, whose own values follow the family.
fn rewrite(path: &Path, key: &str, value: SettingValue, family: Option<&Family>) -> io::Result<()> {
    let mut settings = Settings::open(path);
    if let Some(family) = family {
        settings = settings.member_of(family);
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
