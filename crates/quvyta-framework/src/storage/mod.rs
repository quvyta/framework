//! Settings storage: an application's preferences in a TOML file in the platform config
//! directory.
//!
//! ```toml
//! theme = "nordic"
//! language = "tr"
//!
//! [editor]
//! tab-width = 4
//! ```
//!
//! Keys are dotted paths (`"editor.tab-width"`); the dots become TOML tables. Loading never
//! fails: a broken file yields located [`Diagnostic`]s, broken entries are skipped and the rest
//! is used. Saving goes through [`atomic_write`], so a crash never leaves half a file; a file
//! that could not be read completely is kept as `settings.toml.bak` before it is first
//! overwritten.
//!
//! The module also holds what every application needs around its own files, settings or not:
//! [`config_dir`] and [`data_dir`] for the two folders a platform gives an application,
//! [`atomic_write`] for writing any file safely, [`AppLock`] for "one instance at a time", and
//! [`machine_name`] for keeping one file per machine in a folder several machines share.
//!
//! An application can describe its keys with a [`Schema`]. Loading then checks every key against
//! it, and with [`Settings::self_heal`] on it repairs the file: unknown keys are removed, invalid
//! values are replaced by their default, and each repair is reported. Optional keys
//! ([`Schema::optional`]) are kept only while valid, and keys under an open prefix
//! ([`Schema::open`]) are kept as they are. A missing key is never written: reading it gives
//! `None` and the application falls back to its default.

mod atomic;
mod dirs;
#[cfg(test)]
mod healing_tests;
mod lock;
mod machine;
mod schema;
mod value;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use toml::de::{DeTable, DeValue};

pub use atomic::{WriteStep, atomic_write, atomic_write_reporting};
pub use dirs::{config_dir, data_dir};
pub use lock::{AppLock, holder_pid};
pub use machine::machine_name;
pub use schema::{Schema, SettingKind};
pub use value::{Setting, SettingValue};

use crate::diagnostics::{Diagnostic, Location};
use crate::doc::Doc;
use crate::icons::IconMode;
use crate::runtime::Command;

/// The file name settings are stored under.
const FILE_NAME: &str = "settings.toml";

/// An application's settings: typed values under dotted keys, loaded from and saved to one
/// TOML file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    path: Option<PathBuf>,
    values: Vec<(String, SettingValue)>,
    diagnostics: Vec<Diagnostic>,
    keep_backup: bool,
    /// Where each key was written in the loaded file, for diagnostics.
    origins: Vec<(String, Location)>,
    /// Keys of the loaded file whose value settings could not store.
    skipped: Vec<String>,
    /// How many diagnostics came from reading the file; the schema check comes after them.
    read_problems: usize,
    /// Repairs made so far; they stay reported when the check runs again.
    repairs: Vec<Diagnostic>,
    schema: Option<Schema>,
    self_heal: bool,
}

impl Settings {
    /// The key of the theme id.
    pub const THEME: &'static str = "theme";
    /// The key of the locale code.
    pub const LANGUAGE: &'static str = "language";
    /// The key of the icon mode: `auto`, `nerd`, `unicode` or `ascii`.
    pub const ICONS: &'static str = "icons";
    /// The key of the reduced motion flag.
    pub const REDUCED_MOTION: &'static str = "reduced-motion";
    /// The key of the pillar style: `thick` or `thin`.
    pub const PILLAR: &'static str = "pillar";
    /// The key of the selection slide flag.
    pub const SLIDE: &'static str = "slide";

    /// Settings that live only in memory; saving does nothing. For tests and for applications
    /// run without a config directory.
    #[must_use]
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Loads the settings of application `app` from the platform config directory:
    /// `$XDG_CONFIG_HOME/<app>/settings.toml` or `~/.config/<app>/settings.toml` on Linux and
    /// other Unix systems, `~/Library/Application Support/<app>/settings.toml` on macOS and
    /// `%APPDATA%\<app>\settings.toml` on Windows. Without a home directory the settings stay in
    /// memory and a diagnostic says why.
    #[must_use]
    pub fn load(app: &str) -> Self {
        match config_dir(app) {
            Some(dir) => Self::open(dir.join(FILE_NAME)),
            None => {
                let mut settings = Self::in_memory();
                settings.read_problem(Diagnostic::warning(None, "no config directory found; settings are not saved"));
                settings
            }
        }
    }

    /// Loads settings from `path`. A missing file is an empty start, not a problem.
    #[must_use]
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut settings = Self { path: Some(path.clone()), ..Self::default() };
        match fs::read_to_string(&path) {
            Ok(text) => settings.parse(&display_name(&path), &text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                settings.keep_backup = true;
                settings.read_problem(Diagnostic::error(None, format!("{}: {error}", path.display())));
            }
        }
        settings
    }

    /// Records a problem found while reading, which a later schema check keeps.
    fn read_problem(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
        self.read_problems = self.diagnostics.len();
    }

    /// Reads settings from TOML `text`, reporting problems against `file`. Saving does nothing.
    #[must_use]
    pub fn parse_str(file: &str, text: &str) -> Self {
        let mut settings = Self::default();
        settings.parse(file, text);
        settings
    }

    fn parse(&mut self, file: &str, text: &str) {
        let (root, errors) = Doc::new(file, text).parse_recoverable();
        self.keep_backup |= !errors.is_empty();
        self.diagnostics.extend(errors);
        let mut reader = Reader { file, text, settings: self };
        reader.table(&root, "");
        self.read_problems = self.diagnostics.len();
        self.review();
    }

    /// Checks the loaded keys against `schema` instead of only the built-in keys: keys it does
    /// not know and values it does not accept become located warnings. The file is not touched
    /// unless [`self_heal`](Self::self_heal) is on.
    ///
    /// ```
    /// use qframe::storage::{Schema, Settings};
    ///
    /// let text = "language = \"tr\"\ncolor = \"red\"\npillar = \"thick\"\n";
    /// let checked = Settings::parse_str("settings.toml", text).schema(Schema::builtin());
    /// assert_eq!(checked.diagnostics()[0].to_string(), "settings.toml:2:1: warning: `color` is not a known setting; it is ignored");
    /// assert!(checked.value("color").is_some(), "kept while self-healing is off");
    /// ```
    #[must_use]
    pub fn schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self.review();
        self
    }

    /// Repairs the loaded settings by the [`schema`](Self::schema). Every key is checked on its
    /// own: valid keys are kept, unknown keys are removed and invalid values are replaced by their
    /// default. An invalid [optional](Schema::optional) key is removed, since it has no default;
    /// keys under an [open](Schema::open) prefix are kept as they are unless a rule declares them.
    /// Missing keys are not added. Key order is never a problem and is left as it is. When
    /// anything changed, the file as it was is kept as `settings.toml.bak` and the repaired
    /// settings are saved once; every repair is a located warning in
    /// [`diagnostics`](Self::diagnostics).
    ///
    /// Off by default. Only the application knows all of its keys, so nothing is repaired until
    /// a schema is given; the order of the two calls does not matter.
    #[must_use]
    pub fn self_heal(mut self, on: bool) -> Self {
        self.self_heal = on;
        self.review();
        self
    }

    /// Checks every key against the schema (the built-in one until the application gives its
    /// own) and, when healing, repairs what the check finds.
    fn review(&mut self) {
        self.diagnostics.truncate(self.read_problems);
        self.diagnostics.extend(self.repairs.iter().cloned());
        let explicit = self.schema.is_some();
        let heal = self.self_heal && explicit;
        let schema = self.schema.clone().unwrap_or_else(Schema::builtin);
        let mut changed = false;
        let keys: Vec<String> = self.keys().map(str::to_owned).collect();
        for key in keys {
            let location = self.origin(&key);
            let Some(rule) = schema.get(&key) else {
                if schema.is_open(&key) {
                    continue;
                }
                if heal {
                    self.remove(&key);
                    self.repaired(Diagnostic::warning(location, format!("`{key}` is not a known setting; removed")));
                    changed = true;
                } else if explicit {
                    self.diagnostics
                        .push(Diagnostic::warning(location, format!("`{key}` is not a known setting; it is ignored")));
                }
                continue;
            };
            let Some(value) = self.value(&key).filter(|value| !rule.accepts(value)) else { continue };
            let found = value.literal();
            let expected = rule.describe();
            if heal {
                let message = match rule.default_value().cloned() {
                    Some(default) => {
                        let message =
                            format!("`{key}` must be {expected}, found {found}; replaced with {}", default.literal());
                        self.store(&key, default);
                        message
                    }
                    None => {
                        self.remove(&key);
                        format!("`{key}` must be {expected}, found {found}; removed")
                    }
                };
                self.repaired(Diagnostic::warning(location, message));
                changed = true;
            } else {
                self.diagnostics.push(Diagnostic::warning(
                    location,
                    format!("`{key}` must be {expected}, found {found}; it is ignored"),
                ));
            }
        }
        if heal {
            // A known key whose entry settings could not store (a date, say) is invalid as well.
            for key in std::mem::take(&mut self.skipped) {
                if let Some(rule) = schema.get(&key)
                    && self.value(&key).is_none()
                {
                    let message = match rule.default_value().cloned() {
                        Some(default) => {
                            let message = format!(
                                "`{key}` holds a value settings cannot store; replaced with {}",
                                default.literal()
                            );
                            self.store(&key, default);
                            message
                        }
                        None => format!("`{key}` holds a value settings cannot store; removed"),
                    };
                    self.repaired(Diagnostic::warning(self.origin(&key), message));
                    changed = true;
                }
            }
            changed |= self.separate_tables(&schema);
        }
        if changed && self.path.is_some() {
            // Healing drops what the user wrote, so the file as it was stays next to the repaired one.
            self.keep_backup = true;
            if let Err(error) = self.save() {
                let place = self.path.as_deref().map(|path| path.display().to_string()).unwrap_or_default();
                self.diagnostics
                    .push(Diagnostic::error(None, format!("{place}: repaired settings not saved: {error}")));
            }
        }
    }

    /// Removes every key that sits where another kept key has a table (`plugins.git = true` next
    /// to `plugins.git.sign = true`), which one TOML file cannot hold. A declared key wins over an
    /// open one; otherwise the key written first stays. Returns whether anything was removed.
    fn separate_tables(&mut self, schema: &Schema) -> bool {
        let nested = |a: &str, b: &str| {
            let (short, long) = if a.len() < b.len() { (a, b) } else { (b, a) };
            long.strip_prefix(short).is_some_and(|rest| rest.starts_with('.'))
        };
        let mut removed = false;
        loop {
            let clash = self.values.iter().enumerate().find_map(|(later, (key, _))| {
                self.values[..later].iter().position(|(kept, _)| nested(kept, key)).map(|first| (first, later))
            });
            let Some((first, later)) = clash else { break };
            let declared = |index: usize| schema.get(&self.values[index].0).is_some();
            let (gone, stays) = if declared(later) && !declared(first) { (first, later) } else { (later, first) };
            let kept = self.values[stays].0.clone();
            let (key, _) = self.values.remove(gone);
            let message = format!("`{key}` cannot sit next to `{kept}` in one file; removed");
            self.repaired(Diagnostic::warning(self.origin(&key), message));
            removed = true;
        }
        removed
    }

    fn repaired(&mut self, diagnostic: Diagnostic) {
        self.repairs.push(diagnostic.clone());
        self.diagnostics.push(diagnostic);
    }

    /// Stores `value` under `key`, replacing an existing value where it is so the key keeps its
    /// place. Returns whether anything changed.
    fn store(&mut self, key: &str, value: SettingValue) -> bool {
        match self.values.iter_mut().find(|(k, _)| k == key) {
            Some((_, current)) if *current == value => false,
            Some((_, current)) => {
                *current = value;
                true
            }
            None => {
                self.values.push((key.to_owned(), value));
                true
            }
        }
    }

    /// Where `key` was written in the loaded file.
    fn origin(&self, key: &str) -> Option<Location> {
        self.origins.iter().find(|(k, _)| k == key).map(|(_, location)| location.clone())
    }

    /// Where the settings are saved, if anywhere.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Problems found while loading.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// The raw value under `key`.
    #[must_use]
    pub fn value(&self, key: &str) -> Option<&SettingValue> {
        self.values.iter().find(|(k, _)| k == key).map(|(_, value)| value)
    }

    /// The value under `key` as `T`; `None` when missing or of another type.
    #[must_use]
    pub fn get<T: Setting>(&self, key: &str) -> Option<T> {
        self.value(key).and_then(T::from_setting)
    }

    /// The value under `key` as `T`, or `default`.
    #[must_use]
    pub fn get_or<T: Setting>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// Stores `value` under `key`. Returns whether anything changed.
    pub fn set<T: Setting>(&mut self, key: &str, value: T) -> bool {
        self.store(key, value.to_setting())
    }

    /// Removes `key`. Returns whether it existed.
    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.values.len();
        self.values.retain(|(k, _)| k != key);
        before != self.values.len()
    }

    /// Every key, in file order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.iter().map(|(key, _)| key.as_str())
    }

    /// The saved theme id.
    #[must_use]
    pub fn theme(&self) -> Option<String> {
        self.get(Self::THEME)
    }

    /// The saved locale code.
    #[must_use]
    pub fn language(&self) -> Option<String> {
        self.get(Self::LANGUAGE)
    }

    /// The saved icon mode.
    #[must_use]
    pub fn icon_mode(&self) -> Option<IconMode> {
        self.get::<String>(Self::ICONS).and_then(|name| IconMode::from_name(&name))
    }

    /// The saved reduced motion flag.
    #[must_use]
    pub fn reduced_motion(&self) -> Option<bool> {
        self.get(Self::REDUCED_MOTION)
    }

    /// The saved pillar style.
    #[must_use]
    pub fn pillar_style(&self) -> Option<crate::icons::PillarStyle> {
        self.get::<String>(Self::PILLAR).and_then(|name| crate::icons::PillarStyle::from_name(&name))
    }

    /// The saved selection slide flag.
    #[must_use]
    pub fn slide(&self) -> Option<bool> {
        self.get(Self::SLIDE)
    }

    /// Commands that switch theme, language, icons, reduced motion, pillar and slide to the saved values;
    /// nothing for values that are not saved.
    #[must_use]
    pub fn apply<Msg: Send + 'static>(&self) -> Command<Msg> {
        let mut commands = Vec::new();
        if let Some(theme) = self.theme() {
            commands.push(Command::set_theme(theme));
        }
        if let Some(language) = self.language() {
            commands.push(Command::set_locale(language));
        }
        if let Some(mode) = self.icon_mode() {
            commands.push(Command::set_icon_mode(mode));
        }
        if let Some(reduced) = self.reduced_motion() {
            commands.push(Command::set_reduced_motion(reduced));
        }
        if let Some(style) = self.pillar_style() {
            commands.push(Command::set_pillar(style));
        }
        if let Some(slide) = self.slide() {
            commands.push(Command::set_slide(slide));
        }
        Command::batch(commands)
    }

    /// The settings as TOML text: plain keys first, then one table per dotted prefix.
    #[must_use]
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        let mut sections: Vec<(&str, Vec<(&str, &SettingValue)>)> = Vec::new();
        for (key, value) in &self.values {
            let (section, name) = key.rsplit_once('.').unwrap_or(("", key));
            match sections.iter_mut().find(|(s, _)| *s == section) {
                Some((_, entries)) => entries.push((name, value)),
                None => sections.push((section, vec![(name, value)])),
            }
        }
        sections.sort_by_key(|(section, _)| !section.is_empty());
        for (section, entries) in sections {
            if !section.is_empty() {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push('[');
                for (index, part) in section.split('.').enumerate() {
                    if index > 0 {
                        out.push('.');
                    }
                    value::key(part, &mut out);
                }
                out.push_str("]\n");
            }
            for (name, value) in entries {
                value::key(name, &mut out);
                out.push_str(" = ");
                value.write(&mut out);
                out.push('\n');
            }
        }
        out
    }

    /// Writes the settings to their file atomically, creating the directory when needed.
    /// In-memory settings do nothing.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the directory or file cannot be written.
    pub fn save(&mut self) -> io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        if self.keep_backup && path.exists() {
            fs::copy(&path, path.with_extension("toml.bak"))?;
        }
        atomic_write(&path, self.to_toml().as_bytes())?;
        self.keep_backup = false;
        Ok(())
    }

    /// Saves a copy of the settings on a background thread and reports the result, so a slow
    /// disk never holds up drawing. Use it right after changing a value in `update`.
    #[must_use]
    pub fn save_command<Msg: Send + 'static>(
        &self,
        done: impl FnOnce(Result<(), String>) -> Msg + Send + 'static,
    ) -> Command<Msg> {
        let mut copy = self.clone();
        Command::perform(move || done(copy.save().map_err(|error| error.to_string())))
    }
}

/// Walks a parsed document into dotted keys.
struct Reader<'a> {
    file: &'a str,
    text: &'a str,
    settings: &'a mut Settings,
}

impl Reader<'_> {
    fn table(&mut self, table: &DeTable<'_>, prefix: &str) {
        for (name, value) in table {
            let key =
                if prefix.is_empty() { name.get_ref().to_string() } else { format!("{prefix}.{}", name.get_ref()) };
            if let DeValue::Table(inner) = value.get_ref() {
                self.table(inner, &key);
                continue;
            }
            // The key's own position: searching the text for the name would find it inside
            // comments, values or longer keys.
            let origin = Location::from_offset(self.file, self.text, name.span().start);
            self.settings.origins.retain(|(k, _)| *k != key);
            self.settings.origins.push((key.clone(), origin));
            match self.value(value.get_ref()) {
                Some(parsed) => {
                    self.settings.values.retain(|(k, _)| *k != key);
                    self.settings.values.push((key, parsed));
                }
                None => {
                    self.settings.skipped.push(key.clone());
                    let location = Location::from_offset(self.file, self.text, value.span().start);
                    self.settings.diagnostics.push(Diagnostic::warning(
                        Some(location),
                        format!(
                            "`{key}` holds a {} that settings do not store; it is skipped",
                            value.get_ref().type_str()
                        ),
                    ));
                    self.settings.keep_backup = true;
                }
            }
        }
    }

    fn value(&self, value: &DeValue<'_>) -> Option<SettingValue> {
        Some(match value {
            DeValue::Boolean(flag) => SettingValue::Bool(*flag),
            DeValue::String(text) => SettingValue::Text(text.to_string()),
            DeValue::Integer(number) => {
                SettingValue::Integer(i64::from_str_radix(number.as_str(), number.radix()).ok()?)
            }
            DeValue::Float(number) => SettingValue::Float(number.as_str().replace('_', "").parse().ok()?),
            DeValue::Array(items) => {
                SettingValue::List(items.iter().map(|item| self.value(item.get_ref())).collect::<Option<Vec<_>>>()?)
            }
            DeValue::Datetime(_) | DeValue::Table(_) => return None,
        })
    }
}

fn display_name(path: &Path) -> String {
    path.file_name().and_then(|name| name.to_str()).unwrap_or(FILE_NAME).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quvyta-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn typed_get_set_and_round_trip() {
        let mut settings = Settings::in_memory();
        assert!(settings.set(Settings::THEME, "nordic".to_owned()));
        assert!(!settings.set(Settings::THEME, "nordic".to_owned()));
        settings.set("editor.tab-width", 4u16);
        settings.set("editor.ratio", 0.25f64);
        settings.set("recent.projects", vec!["api".to_owned(), "web \"beta\"".to_owned()]);
        settings.set(Settings::REDUCED_MOTION, true);
        let text = settings.to_toml();
        assert_eq!(
            text,
            "theme = \"nordic\"\nreduced-motion = true\n\n[editor]\ntab-width = 4\nratio = 0.25\n\n[recent]\nprojects = [\"api\", \"web \\\"beta\\\"\"]\n"
        );
        let back = Settings::parse_str("settings.toml", &text);
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        assert_eq!(back.get::<u16>("editor.tab-width"), Some(4));
        assert_eq!(back.get::<Vec<String>>("recent.projects").map(|p| p.len()), Some(2));
        assert_eq!(back.get::<bool>("editor.tab-width"), None);
        assert_eq!(back.get_or("missing", 7u8), 7);
        assert_eq!(back.theme().as_deref(), Some("nordic"));
    }

    #[test]
    fn broken_files_give_located_diagnostics_and_keep_good_values() {
        let text =
            "theme = \"amber\"\nlanguage = \nicons = \"sparkly\"\nreduced-motion = \"yes\"\nstarted = 2026-09-16\n";
        let settings = Settings::parse_str("settings.toml", text);
        assert_eq!(settings.theme().as_deref(), Some("amber"));
        let lines: Vec<(usize, String)> = settings
            .diagnostics()
            .iter()
            .map(|d| (d.location.as_ref().map_or(0, |l| l.line), d.message.clone()))
            .collect();
        assert!(lines.iter().any(|(line, _)| *line == 2), "{lines:?}");
        assert!(lines.iter().any(|(line, m)| *line == 3 && m.contains("auto, nerd")), "{lines:?}");
        assert!(lines.iter().any(|(line, m)| *line == 4 && m.contains("boolean")), "{lines:?}");
        assert!(lines.iter().any(|(line, m)| *line == 5 && m.contains("datetime")), "{lines:?}");
        assert_eq!(settings.icon_mode(), None);
        assert_eq!(settings.reduced_motion(), None);
    }

    #[test]
    fn saves_atomically_and_backs_up_broken_files() {
        let dir = temp_dir("save");
        let path = dir.join("nested").join(FILE_NAME);
        let mut settings = Settings::open(&path);
        assert!(settings.diagnostics().is_empty());
        settings.set(Settings::LANGUAGE, "tr".to_owned());
        settings.save().expect("saved");
        assert_eq!(fs::read_to_string(&path).expect("written"), "language = \"tr\"\n");
        let leftovers: Vec<_> = fs::read_dir(path.parent().expect("dir")).expect("list").collect();
        assert_eq!(leftovers.len(), 1, "no temporary file stays behind");

        fs::write(&path, "language = \"tr\"\nicons = [\n").expect("break the file");
        let mut broken = Settings::open(&path);
        assert!(!broken.diagnostics().is_empty());
        assert_eq!(broken.language().as_deref(), Some("tr"));
        broken.set(Settings::ICONS, "ascii".to_owned());
        broken.save().expect("saved");
        assert!(fs::read_to_string(path.with_extension("toml.bak")).expect("backup").contains("icons = ["));
        assert_eq!(Settings::open(&path).icon_mode(), Some(IconMode::Ascii));
        fs::remove_dir_all(&dir).expect("clean");
    }

    #[test]
    fn apply_turns_saved_values_into_commands() {
        let settings = Settings::parse_str("s.toml", "theme = \"iris\"\nicons = \"ascii\"\n");
        let command: Command<()> = settings.apply();
        assert_eq!(command.actions.len(), 2);
    }
}
