//! Localisation: locale files, plural forms, system language detection and [`t!`](crate::t!).
//!
//! ```toml
//! [meta]
//! name = "Türkçe"
//! code = "tr"
//! fallback = "en"
//!
//! [files]
//! count = { one = "{n} dosya", other = "{n} dosya" }
//! ```
//!
//! Lookups try the active locale, then its `fallback` chain, then English. A key found
//! nowhere is shown as `⟦key⟧` so a missing translation is visible on screen.

mod locale;
mod plural;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io;
use std::path::Path;
use std::sync::Arc;

pub use plural::PluralCategory;

use crate::assets;
use crate::diagnostics::Diagnostic;
use locale::{Locale, Message, Piece, Template};

/// The final fallback locale.
const ROOT_LOCALE: &str = "en";

/// A value substituted into a `{placeholder}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// Text.
    Text(String),
    /// A count; the argument named `n` also selects the plural form.
    Int(i64),
}

impl From<&str> for Arg {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for Arg {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<i64> for Arg {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<i32> for Arg {
    fn from(value: i32) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<u16> for Arg {
    fn from(value: u16) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<u32> for Arg {
    fn from(value: u32) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<usize> for Arg {
    fn from(value: usize) -> Self {
        Self::Int(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

/// All locales known to an application and the active one.
#[derive(Debug, Clone)]
pub struct I18n {
    locales: BTreeMap<String, Locale>,
    active: String,
    diagnostics: Vec<Diagnostic>,
}

impl I18n {
    /// The built-in locales with English active.
    #[must_use]
    pub fn builtin() -> Self {
        let mut i18n = Self { locales: BTreeMap::new(), active: ROOT_LOCALE.to_owned(), diagnostics: Vec::new() };
        for (code, text) in assets::LOCALES {
            i18n.add_source(&format!("{code}.toml"), text);
        }
        i18n
    }

    /// Adds a locale from TOML text. A locale with an existing code is merged into it, the
    /// new messages winning, so applications can extend and override the built-in text.
    /// Returns whether the file was usable.
    pub fn add_source(&mut self, file: &str, text: &str) -> bool {
        let Some(parsed) = locale::parse(file, text, &mut self.diagnostics) else {
            return false;
        };
        match self.locales.get_mut(&parsed.code) {
            Some(existing) => {
                existing.name = parsed.name;
                if parsed.fallback.is_some() {
                    existing.fallback = parsed.fallback;
                }
                existing.messages.extend(parsed.messages);
            }
            None => {
                self.locales.insert(parsed.code.clone(), parsed);
            }
        }
        true
    }

    /// Loads every `*.toml` file in `dir`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the directory cannot be read. A file that cannot be read is
    /// skipped and reported in the diagnostics.
    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        let found = assets::read_toml_dir(dir)?;
        self.diagnostics.extend(found.skipped);
        for (_, file, text) in found.files {
            self.add_source(&file, &text);
        }
        Ok(())
    }

    /// Problems found while loading.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// `(code, display name)` of every locale, sorted by code; for a settings screen.
    #[must_use]
    pub fn list(&self) -> Vec<(String, String)> {
        self.locales.values().map(|l| (l.code.clone(), l.name.clone())).collect()
    }

    /// The active locale code.
    #[must_use]
    pub fn active(&self) -> &str {
        &self.active
    }

    /// Activates `code`. Returns `false` and changes nothing when the locale is unknown.
    pub fn set_active(&mut self, code: &str) -> bool {
        if self.locales.contains_key(code) {
            code.clone_into(&mut self.active);
            true
        } else {
            false
        }
    }

    /// Translates `key` with `args`.
    #[must_use]
    pub fn translate(&self, key: &str, args: &[(&str, Arg)]) -> String {
        let Some((language, message)) = self.find(key) else {
            return format!("⟦{key}⟧");
        };
        let template = match message {
            Message::Plain(template) => template,
            Message::Plural(forms) => {
                let count = args.iter().find_map(|(name, arg)| match (name, arg) {
                    (&"n", Arg::Int(n)) => Some(*n),
                    _ => None,
                });
                let category = count.map_or(PluralCategory::Other, |n| PluralCategory::of(language, n));
                match forms.get(&category).or_else(|| forms.get(&PluralCategory::Other)) {
                    Some(template) => template,
                    None => return format!("⟦{key}⟧"),
                }
            }
        };
        render(template, args)
    }

    /// Keys present in `reference` but missing from `code`, sorted. Use in tests to keep
    /// every translation complete.
    #[must_use]
    pub fn missing_keys(&self, code: &str, reference: &str) -> Vec<String> {
        let (Some(target), Some(reference)) = (self.locales.get(code), self.locales.get(reference)) else {
            return Vec::new();
        };
        reference.messages.keys().filter(|key| !target.messages.contains_key(*key)).cloned().collect()
    }

    /// The locale code to use for a system: the first of `LC_ALL`, `LC_MESSAGES`, `LANG`,
    /// then the operating system setting, reduced to a known locale code.
    #[must_use]
    pub fn detect(&self, env: impl Fn(&str) -> Option<String>) -> Option<String> {
        let from_env =
            ["LC_ALL", "LC_MESSAGES", "LANG"].iter().filter_map(|name| env(name)).find(|value| !value.is_empty());
        let candidate = from_env.or_else(sys_locale::get_locale)?;
        let language = language_of(&candidate)?;
        self.locales.contains_key(&language).then_some(language)
    }

    fn find(&self, key: &str) -> Option<(&str, &Message)> {
        let mut visited: Vec<&str> = Vec::new();
        let mut code = Some(self.active.as_str());
        while let Some(current) = code {
            if visited.contains(&current) {
                break;
            }
            visited.push(current);
            let Some(locale) = self.locales.get(current) else {
                break;
            };
            if let Some(message) = locale.messages.get(key) {
                return Some((locale.code.as_str(), message));
            }
            code = locale.fallback.as_deref();
        }
        if visited.contains(&ROOT_LOCALE) {
            return None;
        }
        self.locales.get(ROOT_LOCALE).and_then(|root| root.messages.get(key).map(|m| (root.code.as_str(), m)))
    }
}

/// Reduces `tr_TR.UTF-8`, `tr-TR` or `tr` to `tr`. `C` and `POSIX` have no language.
fn language_of(locale: &str) -> Option<String> {
    let language: String = locale.split(['_', '-', '.', '@']).next().unwrap_or_default().to_ascii_lowercase();
    let is_language = (2..=3).contains(&language.len()) && language.chars().all(|c| c.is_ascii_lowercase());
    (is_language && language != "c").then_some(language)
}

fn render(template: &Template, args: &[(&str, Arg)]) -> String {
    let mut out = String::new();
    for piece in &template.0 {
        match piece {
            Piece::Text(text) => out.push_str(text),
            Piece::Arg(name) => match args.iter().find(|(arg_name, _)| arg_name == name) {
                Some((_, Arg::Text(text))) => out.push_str(text),
                Some((_, Arg::Int(n))) => {
                    let _ = write!(out, "{n}");
                }
                None => {
                    let _ = write!(out, "{{{name}}}");
                }
            },
        }
    }
    out
}

thread_local! {
    static ACTIVE: RefCell<Option<Arc<I18n>>> = const { RefCell::new(None) };
}

/// Runs `f` with `i18n` as the translator used by [`t!`](crate::t!) on this thread, restoring the
/// previous translator afterwards, even if `f` panics.
pub fn scope<R>(i18n: Arc<I18n>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Arc<I18n>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            ACTIVE.with(|active| *active.borrow_mut() = previous);
        }
    }
    let previous = ACTIVE.with(|active| active.borrow_mut().replace(i18n));
    let _restore = Restore(previous);
    f()
}

/// Translates with the translator installed by [`scope`]. Outside a scope every key is
/// shown as `⟦key⟧`. Prefer the [`t!`](crate::t!) macro.
#[must_use]
pub fn translate_active(key: &str, args: &[(&str, Arg)]) -> String {
    ACTIVE.with(|active| match active.borrow().as_ref() {
        Some(i18n) => i18n.translate(key, args),
        None => format!("⟦{key}⟧"),
    })
}

/// Translates a key with the active translator.
///
/// ```
/// use std::sync::Arc;
/// use qframe::{i18n, t};
///
/// let mut catalog = i18n::I18n::builtin();
/// catalog.add_source(
///     "app-tr.toml",
///     "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\n[files]\ncount = { one = \"{n} dosya\", other = \"{n} dosya\" }\n",
/// );
/// catalog.set_active("tr");
/// let label = i18n::scope(Arc::new(catalog), || t!("files.count", n = 3));
/// assert_eq!(label, "3 dosya");
/// ```
#[macro_export]
macro_rules! t {
    ($key:expr $(,)?) => {
        $crate::i18n::translate_active($key, &[])
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::translate_active(
            $key,
            &[$((stringify!($name), $crate::i18n::Arg::from($value))),+],
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn catalog() -> I18n {
        let mut i18n = I18n::builtin();
        assert!(i18n.add_source(
            "app-en.toml",
            "[meta]\nname = \"English\"\ncode = \"en\"\n[files]\ncount = { one = \"{n} file\", other = \"{n} files\" }\nhello = \"Hello {name}\"\nonly-en = \"English only\"\n",
        ));
        assert!(i18n.add_source(
            "app-tr.toml",
            "[meta]\nname = \"Türkçe\"\ncode = \"tr\"\nfallback = \"en\"\n[files]\ncount = { one = \"{n} dosya\", other = \"{n} dosya\" }\nhello = \"Merhaba {name}\"\n",
        ));
        i18n
    }

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn translates_with_args_plurals_and_fallback() {
        let mut i18n = catalog();
        assert_eq!(i18n.translate("files.count", &[("n", Arg::from(1))]), "1 file");
        assert_eq!(i18n.translate("files.count", &[("n", Arg::from(3))]), "3 files");
        assert!(i18n.set_active("tr"));
        assert_eq!(i18n.translate("files.hello", &[("name", Arg::from("Ada"))]), "Merhaba Ada");
        assert_eq!(i18n.translate("files.only-en", &[]), "English only");
        assert_eq!(i18n.translate("files.nope", &[]), "⟦files.nope⟧");
        assert_eq!(i18n.translate("files.hello", &[]), "Merhaba {name}");
        assert!(!i18n.set_active("xx"));
        assert_eq!(i18n.active(), "tr");
    }

    #[test]
    fn later_files_extend_and_override_a_locale() {
        let mut i18n = catalog();
        assert!(i18n.add_source(
            "more-en.toml",
            "[meta]\nname = \"English (app)\"\ncode = \"en\"\n[files]\nhello = \"Hi {name}\"\n",
        ));
        assert_eq!(i18n.translate("files.hello", &[("name", Arg::from("Ada"))]), "Hi Ada");
        assert_eq!(i18n.translate("files.only-en", &[]), "English only");
        assert_eq!(
            i18n.list(),
            vec![("en".to_owned(), "English (app)".to_owned()), ("tr".to_owned(), "Türkçe".to_owned()),]
        );
    }

    #[test]
    fn reports_missing_translations() {
        assert_eq!(catalog().missing_keys("tr", "en"), vec!["files.only-en".to_owned()]);
    }

    #[test]
    fn detects_language_from_environment() {
        let i18n = catalog();
        assert_eq!(i18n.detect(env(&[("LANG", "tr_TR.UTF-8")])), Some("tr".to_owned()));
        assert_eq!(i18n.detect(env(&[("LC_ALL", "en_US.UTF-8"), ("LANG", "tr_TR.UTF-8")])), Some("en".to_owned()));
        assert_eq!(i18n.detect(env(&[("LANG", "de_DE.UTF-8")])), None);
        assert_eq!(language_of("C"), None);
        assert_eq!(language_of("pt-BR"), Some("pt".to_owned()));
    }

    #[test]
    fn macro_uses_scoped_translator() {
        let mut i18n = catalog();
        i18n.set_active("tr");
        assert_eq!(t!("files.count", n = 2), "⟦files.count⟧");
        let text = scope(Arc::new(i18n), || t!("files.count", n = 2));
        assert_eq!(text, "2 dosya");
        assert_eq!(t!("files.count", n = 2), "⟦files.count⟧");
    }
}
