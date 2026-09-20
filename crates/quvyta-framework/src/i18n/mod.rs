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
//! [`I18n::has`] asks whether one language carries a key itself, without the fallbacks, so a
//! test can keep every language complete.

mod locale;
mod plural;
mod tag;
mod week;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io;
use std::path::Path;
use std::sync::Arc;

pub use plural::PluralCategory;

use crate::assets;
use crate::date::Weekday;
use crate::diagnostics::Diagnostic;
use locale::{Locale, Message, Piece, Template};
use tag::Tag;

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
    region: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

impl I18n {
    /// The built-in locales with English active.
    #[must_use]
    pub fn builtin() -> Self {
        let mut i18n =
            Self { locales: BTreeMap::new(), active: ROOT_LOCALE.to_owned(), region: None, diagnostics: Vec::new() };
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

    /// Activates the locale that serves the language tag `tag`, such as a language setting of
    /// `en-GB` or `pt_BR.UTF-8`, matched the way [`detect`](Self::detect) matches the system's
    /// language: `en-GB` activates `en` when there is no `en-GB` locale.
    ///
    /// A tag that names a region also sets the [region](Self::region), so `en-GB` starts weeks on
    /// Monday although English alone starts them on Sunday. A tag without one keeps the region, so
    /// choosing `tr` from a list of languages does not forget the country the system is set to.
    /// Returns `false` and changes nothing when no locale serves the tag.
    pub fn select(&mut self, tag: &str) -> bool {
        let Some(parsed) = Tag::parse(tag) else {
            return self.set_active(tag);
        };
        let Some(code) = self.matching(&parsed) else {
            return false;
        };
        self.active = code;
        if let Some(region) = parsed.region().and_then(week::region_code) {
            self.region = Some(region);
        }
        true
    }

    /// The region whose conventions apply, such as `GB`, uppercase: the one set with
    /// [`set_region`](Self::set_region) or [`select`](Self::select), or found by
    /// [`detect_region`](Self::detect_region) when the environment was loaded. `None` leaves the
    /// conventions to the language.
    #[must_use]
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Sets the region, two letters such as `GB` or three digits such as `419`, in either case;
    /// `None` leaves the conventions to the language again. Returns `false` and changes nothing
    /// when `region` is not a region code.
    pub fn set_region(&mut self, region: Option<&str>) -> bool {
        match region {
            None => {
                self.region = None;
                true
            }
            Some(text) => match week::region_code(text) {
                Some(code) => {
                    self.region = Some(code);
                    true
                }
                None => false,
            },
        }
    }

    /// The day a calendar week starts on.
    ///
    /// With a [region](Self::region) it is the region's, from the Unicode CLDR: Sunday in the
    /// United States, Canada, Brazil, Portugal and Japan, Saturday in much of the Middle East,
    /// Monday in the United Kingdom and most of the world. Without one it is the active
    /// language's own `quvyta.date.first-weekday` key (`1` Monday to `7` Sunday), and Monday, the
    /// ISO 8601 week, for a language that does not give it.
    #[must_use]
    pub fn first_weekday(&self) -> Weekday {
        if let Some(region) = &self.region {
            return week::first_day(region);
        }
        let own = self.locales.get(&self.active).and_then(|locale| match locale.messages.get(FIRST_WEEKDAY) {
            Some(Message::Plain(template)) => render(template, &[]).trim().parse::<u8>().ok(),
            _ => None,
        });
        own.and_then(Weekday::from_number).unwrap_or(Weekday::Monday)
    }

    /// What this language writes between a number's whole part and its decimals: a point in
    /// English, Japanese and Chinese, a comma in German, Spanish, French, Portuguese, Russian and
    /// Turkish.
    ///
    /// It comes from the active language's `quvyta.number.decimal` key, and is a point for a
    /// language that does not give it. [`number`] writes a value with it; every number the
    /// framework itself draws — a slider's value, a chart's labels, a file's size — already does.
    #[must_use]
    pub fn decimal_separator(&self) -> char {
        self.find(DECIMAL).map(|_| self.translate(DECIMAL, &[])).and_then(|text| text.chars().next()).unwrap_or('.')
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

    /// Whether the locale `code` itself defines `key`, as a plain message or as a plural table
    /// (a plural key counts once, whatever forms it has).
    ///
    /// The language is always the one named, never the active one, so the answer does not change
    /// with [`set_active`](Self::set_active). Only that locale's own text counts: a key it would
    /// borrow from its `fallback` or from English is not its own, so `has` answers `false` for it
    /// even though [`translate`](Self::translate) shows the borrowed text on screen. That lets a
    /// test require every language to carry its own translation. An unknown `code` has no keys.
    ///
    /// Comparing `translate(key, &[])` with `key` cannot stand in for this: a key found nowhere
    /// translates to `⟦key⟧`, which differs from the key.
    #[must_use]
    pub fn has(&self, code: &str, key: &str) -> bool {
        self.locales.get(code).is_some_and(|locale| locale.messages.contains_key(key))
    }

    /// The plain text of `key` in every locale that defines it itself, the active locale first and
    /// the others in code order. Lets input be read in any known language, such as the unit words
    /// of a length of time typed by someone whose interface is in another language.
    pub(crate) fn in_every_locale(&self, key: &str) -> Vec<String> {
        let active = self.locales.get(&self.active).into_iter();
        let others = self.locales.values().filter(|locale| locale.code != self.active);
        active
            .chain(others)
            .filter_map(|locale| match locale.messages.get(key) {
                Some(Message::Plain(template)) => Some(render(template, &[])),
                _ => None,
            })
            .collect()
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
    /// then the operating system setting, matched to a known locale code.
    ///
    /// Separators and case do not matter (`pt_BR.UTF-8` finds `pt-BR`), and the encoding and
    /// modifier are ignored. The first of these that names a known locale wins:
    ///
    /// 1. the whole tag: `pt_BR` → `pt-BR`, `zh_Hant` → `zh-Hant`;
    /// 2. the language with its writing system, which for Chinese follows the region: `zh_CN`
    ///    and `zh_SG` → `zh-Hans`; `zh_TW`, `zh_HK` and `zh_MO` → `zh-Hant`;
    /// 3. the language alone: `de_AT` → `de`;
    /// 4. the one locale of that language, when there is exactly one: `pt_PT` → `pt-BR` when
    ///    `pt-BR` is the only Portuguese, `zh` → `zh-Hans` when it is the only Chinese.
    ///
    /// `C` and `POSIX` name no language and give `None`, as does a language with no locale.
    #[must_use]
    pub fn detect(&self, env: impl Fn(&str) -> Option<String>) -> Option<String> {
        self.matching(&system_tag(&["LC_ALL", "LC_MESSAGES", "LANG"], env)?)
    }

    /// The region the system is set to, uppercase: `GB` for `LANG=en_GB.UTF-8`. Reads the first of
    /// `LC_ALL`, `LC_TIME`, `LANG`, then the operating system setting, since the calendar
    /// conventions belong to `LC_TIME` where the language belongs to `LC_MESSAGES`. `None` when
    /// that name gives no region, as `en` and `C.UTF-8` do.
    #[must_use]
    pub fn detect_region(&self, env: impl Fn(&str) -> Option<String>) -> Option<String> {
        system_tag(&["LC_ALL", "LC_TIME", "LANG"], env)?.region().and_then(week::region_code)
    }

    /// The known locale code that serves `tag`; see [`I18n::detect`] for the order.
    fn matching(&self, tag: &Tag) -> Option<String> {
        let known = |wanted: &str| self.locales.keys().find(|code| code.eq_ignore_ascii_case(wanted)).cloned();
        let only_one_of_the_language = || {
            let mut same = self.locales.keys().filter(|code| tag::language_of(code) == tag.language);
            let first = same.next()?;
            same.next().is_none().then(|| first.clone())
        };
        known(&tag.full())
            .or_else(|| tag.script().and_then(|script| known(&format!("{}-{script}", tag.language))))
            .or_else(|| known(&tag.language))
            .or_else(only_one_of_the_language)
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

/// The locale key giving a language's first day of the week, for a language without a region.
const FIRST_WEEKDAY: &str = "quvyta.date.first-weekday";

/// The key that carries what a language writes between a number and its decimals.
const DECIMAL: &str = "quvyta.number.decimal";

/// The locale name in the first of `variables` that is set, or else the operating system's.
fn system_tag(variables: &[&str], env: impl Fn(&str) -> Option<String>) -> Option<Tag> {
    let from_env = variables.iter().filter_map(|name| env(name)).find(|value| !value.is_empty());
    Tag::parse(&from_env.or_else(sys_locale::get_locale)?)
}

fn render(template: &Template, args: &[(&str, Arg)]) -> String {
    let mut out = String::new();
    // Writing into a `String` cannot fail, so there is no error here to carry anywhere; the
    // results are dropped for that reason and no other.
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

/// The first day of the week of the translator installed by [`scope`], as
/// [`I18n::first_weekday`] gives it: from the region when one is known, from the language
/// otherwise. Outside a scope it is Monday.
///
/// The runtime installs the translator around `init`, `update` and the other [`App`](crate::runtime::App)
/// methods, so week arithmetic in `update` agrees with the calendars the view draws.
#[must_use]
pub fn first_weekday() -> Weekday {
    ACTIVE.with(|active| active.borrow().as_ref().map_or(Weekday::Monday, |i18n| i18n.first_weekday()))
}

/// What the language of the translator installed by [`scope`] writes between a number's whole
/// part and its decimals, as [`I18n::decimal_separator`] gives it. Outside a scope it is a point.
#[must_use]
pub fn decimal_separator() -> char {
    ACTIVE.with(|active| active.borrow().as_ref().map_or('.', |i18n| i18n.decimal_separator()))
}

/// `value` written with `decimals` decimals in the active language's way: `0.5` in English, `0,5`
/// in French.
///
/// This is what every number the framework draws goes through, and what an application writing a
/// number of its own should use, so one screen never mixes the two ways.
///
/// ```
/// # qframe::i18n::scope(std::sync::Arc::new(qframe::i18n::I18n::builtin()), || {
/// assert_eq!(qframe::i18n::number(1.5, 1), "1.5");
/// # });
/// ```
#[must_use]
pub fn number(value: f64, decimals: usize) -> String {
    localize(format!("{value:.decimals$}"))
}

/// The same number with the point of Rust's own formatting replaced by the active language's
/// separator, for text a caller has already written out.
pub(crate) fn localize(text: String) -> String {
    let separator = decimal_separator();
    if separator == '.' { text } else { text.replace('.', &separator.to_string()) }
}

/// Translates `key` without arguments with the translator installed by [`scope`], or `None` when
/// neither the active locale, its fallbacks nor English define it: for keys only some
/// languages need.
pub(crate) fn translate_active_if_known(key: &str) -> Option<String> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        let i18n = active.as_ref()?;
        i18n.find(key)?;
        Some(i18n.translate(key, &[]))
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
        let names: Vec<(String, String)> =
            i18n.list().into_iter().filter(|(code, _)| code == "en" || code == "tr").collect();
        assert_eq!(names, vec![("en".to_owned(), "English (app)".to_owned()), ("tr".to_owned(), "Türkçe".to_owned())]);
    }

    #[test]
    fn has_looks_at_the_named_language_only() {
        let mut i18n = catalog();
        assert!(i18n.has("en", "files.hello") && i18n.has("tr", "files.hello"));
        assert!(i18n.has("en", "files.only-en"));
        assert!(!i18n.has("tr", "files.only-en"), "borrowed from English, not Turkish's own");
        assert!(i18n.set_active("tr"));
        assert_eq!(i18n.translate("files.only-en", &[]), "English only", "yet the screen shows the fallback");
        assert!(!i18n.has("tr", "files.only-en"), "the active language changes nothing");
        assert!(i18n.has("en", "files.only-en"));
        assert!(!i18n.has("en", "files.nope") && !i18n.has("tr", "files.nope"));
        assert!(!i18n.has("xx", "files.hello"), "an unknown language has no keys");
    }

    #[test]
    fn a_plural_key_counts_as_present() {
        let i18n = catalog();
        assert!(i18n.has("en", "files.count") && i18n.has("tr", "files.count"));
        assert!(!i18n.has("en", "files.count.one"), "a form is not a key of its own");
    }

    #[test]
    fn comparing_a_translation_with_its_key_misses_a_missing_key() {
        let i18n = catalog();
        let key = "files.nope";
        assert_ne!(i18n.translate(key, &[]), key, "the indirect check passes");
        assert!(!i18n.has("en", key), "has reports it missing");
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
        assert_eq!(i18n.detect(env(&[("LANG", "fi_FI.UTF-8")])), None);
        assert_eq!(i18n.detect(env(&[("LANG", "C")])), None);
        assert_eq!(i18n.detect(env(&[("LANG", "POSIX")])), None);
        assert_eq!(i18n.detect(env(&[("LANG", "C.UTF-8")])), None);
    }

    /// A catalog with regional and script locales, as an application adding new languages has.
    fn regional(codes: &[&str]) -> I18n {
        let mut i18n = catalog();
        for code in codes {
            let source = format!("[meta]\nname = \"{code}\"\ncode = \"{code}\"\n[files]\nhello = \"{code}\"\n");
            assert!(i18n.add_source(&format!("{code}.toml"), &source));
        }
        i18n
    }

    fn detected(i18n: &I18n, lang: &str) -> Option<String> {
        i18n.detect(env(&[("LANG", lang)]))
    }

    #[test]
    fn a_region_or_script_code_matches_whole_whatever_its_separator_and_case() {
        let i18n = regional(&["pt-BR", "pt-PT", "zh-Hans", "zh-Hant", "de"]);
        assert_eq!(detected(&i18n, "pt_BR.UTF-8").as_deref(), Some("pt-BR"));
        assert_eq!(detected(&i18n, "pt_PT.UTF-8").as_deref(), Some("pt-PT"));
        assert_eq!(detected(&i18n, "PT-br").as_deref(), Some("pt-BR"));
        assert_eq!(detected(&i18n, "zh-hant").as_deref(), Some("zh-Hant"));
        assert_eq!(detected(&i18n, "tr_TR.UTF-8").as_deref(), Some("tr"));
    }

    #[test]
    fn a_regional_locale_chooses_plural_forms_by_its_language() {
        let mut i18n = catalog();
        assert!(i18n.add_source(
            "pt-BR.toml",
            "[meta]\nname = \"Português\"\ncode = \"pt-BR\"\n[files]\ncount = { one = \"{n} etapa\", other = \"{n} etapas\" }\n",
        ));
        assert!(i18n.set_active("pt-BR"));
        assert_eq!(i18n.translate("files.count", &[("n", Arg::from(0))]), "0 etapa");
        assert_eq!(i18n.translate("files.count", &[("n", Arg::from(2))]), "2 etapas");
    }

    #[test]
    fn a_chinese_region_picks_its_script() {
        let i18n = regional(&["zh-Hans", "zh-Hant"]);
        for lang in ["zh_CN.UTF-8", "zh_SG.UTF-8", "zh-Hans", "zh_Hans_CN"] {
            assert_eq!(detected(&i18n, lang).as_deref(), Some("zh-Hans"), "{lang}");
        }
        for lang in ["zh_TW.UTF-8", "zh_HK.UTF-8", "zh_MO.UTF-8", "zh-Hant"] {
            assert_eq!(detected(&i18n, lang).as_deref(), Some("zh-Hant"), "{lang}");
        }
        assert_eq!(detected(&i18n, "zh"), None, "bare Chinese names no script, and both are known");
    }

    #[test]
    fn a_region_without_a_locale_of_its_own_uses_the_language() {
        let i18n = regional(&["de", "pt-BR", "pt-PT"]);
        assert_eq!(detected(&i18n, "de_AT.UTF-8").as_deref(), Some("de"));
        assert_eq!(detected(&i18n, "de_CH.UTF-8@euro").as_deref(), Some("de"));
        assert_eq!(detected(&i18n, "pt_AO.UTF-8"), None, "two Portuguese locales and no plain one");
    }

    #[test]
    fn the_only_locale_of_a_language_serves_every_region_of_it() {
        let i18n = regional(&["pt-BR", "zh-Hans"]);
        assert_eq!(detected(&i18n, "pt_PT.UTF-8").as_deref(), Some("pt-BR"));
        assert_eq!(detected(&i18n, "pt").as_deref(), Some("pt-BR"));
        assert_eq!(detected(&i18n, "zh").as_deref(), Some("zh-Hans"));
        assert_eq!(detected(&i18n, "zh_TW.UTF-8").as_deref(), Some("zh-Hans"));
        assert_eq!(detected(&i18n, "C"), None);
    }

    /// The languages the framework's own text comes in.
    const BUILT_IN: [&str; 9] = ["de", "en", "es", "fr", "ja", "pt-BR", "ru", "tr", "zh-Hans"];

    #[test]
    fn the_framework_speaks_nine_languages() {
        let codes: Vec<String> = I18n::builtin().list().into_iter().map(|(code, _)| code).collect();
        assert_eq!(codes, BUILT_IN);
    }

    #[test]
    fn every_built_in_plural_gives_each_form_its_language_uses() {
        let i18n = I18n::builtin();
        for (code, locale) in &i18n.locales {
            for (key, message) in &locale.messages {
                let Message::Plural(forms) = message else { continue };
                for n in 0..=200 {
                    let category = PluralCategory::of(code, n);
                    assert!(forms.contains_key(&category), "{code} {key} has no `{}` form for {n}", category.name());
                }
            }
        }
    }

    #[test]
    fn the_system_language_finds_the_built_in_regional_locales() {
        let i18n = I18n::builtin();
        for (lang, code) in [
            ("pt_BR.UTF-8", "pt-BR"),
            ("pt_PT.UTF-8", "pt-BR"),
            ("zh_CN.UTF-8", "zh-Hans"),
            ("zh_TW.UTF-8", "zh-Hans"),
            ("ja_JP.UTF-8", "ja"),
            ("de_AT.UTF-8", "de"),
            ("es_MX.UTF-8", "es"),
            ("fr_CA.UTF-8", "fr"),
            ("ru_RU.UTF-8", "ru"),
            ("tr_TR.UTF-8", "tr"),
        ] {
            assert_eq!(i18n.detect(env(&[("LANG", lang)])).as_deref(), Some(code), "{lang}");
        }
    }

    #[test]
    fn a_week_starts_where_the_language_starts_it() {
        let mut i18n = I18n::builtin();
        for (code, first) in [
            ("en", "7"),
            ("tr", "1"),
            ("de", "1"),
            ("es", "1"),
            ("fr", "1"),
            ("pt-BR", "7"),
            ("ru", "1"),
            ("zh-Hans", "1"),
            ("ja", "7"),
        ] {
            assert!(i18n.set_active(code));
            assert_eq!(i18n.translate("quvyta.date.first-weekday", &[]), first, "{code}");
        }
    }

    #[test]
    fn without_a_region_the_language_gives_the_first_weekday() {
        let mut i18n = I18n::builtin();
        for (code, first) in [
            ("en", Weekday::Sunday),
            ("tr", Weekday::Monday),
            ("de", Weekday::Monday),
            ("pt-BR", Weekday::Sunday),
            ("ja", Weekday::Sunday),
            ("zh-Hans", Weekday::Monday),
        ] {
            assert!(i18n.set_active(code));
            assert_eq!(i18n.first_weekday(), first, "{code}");
        }
    }

    #[test]
    fn a_detected_region_gives_the_first_weekday_over_the_language() {
        let mut i18n = I18n::builtin();
        for (lang, first) in [
            ("en_GB.UTF-8", Weekday::Monday),
            ("en_US.UTF-8", Weekday::Sunday),
            ("pt_BR.UTF-8", Weekday::Sunday),
            ("pt_PT.UTF-8", Weekday::Sunday),
            ("ar_EG.UTF-8", Weekday::Saturday),
            ("en_AU.UTF-8", Weekday::Monday),
        ] {
            let pairs = [("LANG", lang)];
            let lookup = env(&pairs);
            let code = i18n.detect(&lookup).unwrap_or_else(|| ROOT_LOCALE.to_owned());
            assert!(i18n.set_active(&code));
            let region = i18n.detect_region(&lookup);
            assert!(i18n.set_region(region.as_deref()));
            assert_eq!(i18n.first_weekday(), first, "{lang}");
        }
    }

    #[test]
    fn the_region_follows_the_calendar_variables() {
        let i18n = I18n::builtin();
        let region = |pairs: &[(&str, &str)]| i18n.detect_region(env(pairs));
        assert_eq!(region(&[("LANG", "en_GB.UTF-8")]).as_deref(), Some("GB"));
        assert_eq!(region(&[("LC_TIME", "en_GB.UTF-8"), ("LANG", "en_US.UTF-8")]).as_deref(), Some("GB"));
        assert_eq!(region(&[("LC_MESSAGES", "en_GB.UTF-8"), ("LANG", "en_US.UTF-8")]).as_deref(), Some("US"));
        assert_eq!(region(&[("LC_ALL", "de_AT.UTF-8"), ("LC_TIME", "en_GB.UTF-8")]).as_deref(), Some("AT"));
        assert_eq!(region(&[("LANG", "es_419.UTF-8")]).as_deref(), Some("419"));
        assert_eq!(region(&[("LANG", "en")]), None);
        assert_eq!(region(&[("LANG", "C.UTF-8")]), None);
    }

    #[test]
    fn without_a_region_an_unknown_language_starts_on_monday() {
        let mut i18n = I18n::builtin();
        assert!(
            i18n.add_source(
                "fi.toml",
                "[meta]\nname = \"Suomi\"\ncode = \"fi\"\nfallback = \"en\"\n[app]\nx = \"x\"\n"
            )
        );
        assert!(i18n.set_active("fi"));
        assert_eq!(i18n.region(), None);
        assert_eq!(i18n.first_weekday(), Weekday::Monday, "English's Sunday is not borrowed");
    }

    #[test]
    fn a_region_set_by_the_application_decides_until_cleared() {
        let mut i18n = I18n::builtin();
        assert!(i18n.set_region(Some("gb")));
        assert_eq!(i18n.region(), Some("GB"));
        assert_eq!(i18n.first_weekday(), Weekday::Monday);
        assert!(!i18n.set_region(Some("Britain")));
        assert_eq!(i18n.region(), Some("GB"), "a bad code changes nothing");
        assert!(i18n.set_region(None));
        assert_eq!(i18n.first_weekday(), Weekday::Sunday, "English again");
    }

    #[test]
    fn selecting_a_regional_tag_activates_its_language_and_region() {
        let mut i18n = I18n::builtin();
        assert!(i18n.select("en-GB"));
        assert_eq!((i18n.active(), i18n.region()), ("en", Some("GB")));
        assert_eq!(i18n.first_weekday(), Weekday::Monday);
        assert!(i18n.select("tr"));
        assert_eq!((i18n.active(), i18n.region()), ("tr", Some("GB")), "a tag without a region keeps it");
        assert!(i18n.select("pt_BR.UTF-8"));
        assert_eq!((i18n.active(), i18n.region()), ("pt-BR", Some("BR")));
        assert!(!i18n.select("fi-FI"));
        assert_eq!((i18n.active(), i18n.region()), ("pt-BR", Some("BR")), "no Finnish, nothing changes");
        assert!(!i18n.select(""));
    }

    #[test]
    fn the_first_weekday_of_the_active_translator_is_read_without_the_view() {
        assert_eq!(first_weekday(), Weekday::Monday, "outside a scope");
        let mut american = I18n::builtin();
        assert!(american.set_region(Some("US")));
        assert_eq!(scope(Arc::new(american), first_weekday), Weekday::Sunday);
        let mut british = I18n::builtin();
        assert!(british.set_region(Some("GB")));
        assert_eq!(scope(Arc::new(british), first_weekday), Weekday::Monday);
        assert_eq!(first_weekday(), Weekday::Monday, "the scope is gone again");
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
