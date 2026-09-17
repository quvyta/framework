//! The shape of an application's settings: which keys exist, what each may hold and what it
//! falls back to. Loading checks a file against it and, with self-healing on, repairs the file.

use std::fmt;
use std::sync::Arc;

use super::value::{Setting, SettingValue};
use crate::icons::{IconMode, PillarStyle};

/// Decides whether a stored value is acceptable for one key.
type Valid = Arc<dyn Fn(&SettingValue) -> bool + Send + Sync>;

/// What one key may hold.
#[derive(Clone)]
enum Allowed {
    /// `true` or `false`.
    Flag,
    /// Any text.
    Text,
    /// Text from a fixed list.
    Choice(Vec<String>),
    /// Whatever the application's check accepts.
    Check(Valid),
}

impl Allowed {
    fn accepts(&self, value: &SettingValue) -> bool {
        match (self, value) {
            (Self::Flag, SettingValue::Bool(_)) | (Self::Text, SettingValue::Text(_)) => true,
            (Self::Choice(choices), SettingValue::Text(text)) => choices.contains(text),
            (Self::Check(valid), value) => valid(value),
            _ => false,
        }
    }

    /// The expectation in words, for diagnostics.
    fn describe(&self) -> String {
        match self {
            Self::Flag => "a boolean".to_owned(),
            Self::Text => "a string".to_owned(),
            Self::Choice(choices) => format!("one of {}", choices.join(", ")),
            Self::Check(_) => "a value this application accepts".to_owned(),
        }
    }
}

impl PartialEq for Allowed {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Flag, Self::Flag) | (Self::Text, Self::Text) => true,
            (Self::Choice(a), Self::Choice(b)) => a == b,
            // Two checks are the same check only when they are the same closure.
            (Self::Check(a), Self::Check(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// What an optional key may hold, for [`Schema::optional`]: the same kinds as the builders with
/// a default ([`Schema::flag`], [`Schema::text`], [`Schema::choice`], [`Schema::check`]), without
/// the default.
///
/// A plain value rather than one builder per kind (`optional_flag`, `optional_text`, …): the kinds
/// stay listed once, and any kind can be optional.
///
/// ```
/// use qframe::storage::{Schema, SettingKind};
///
/// let schema = Schema::builtin()
///     .optional("deploy.note", SettingKind::text())
///     .optional("deploy.retries", SettingKind::check(|retries: &u8| (1..=10).contains(retries)));
/// ```
#[derive(Clone, PartialEq)]
pub struct SettingKind(Allowed);

impl SettingKind {
    /// `true` or `false`.
    #[must_use]
    pub fn flag() -> Self {
        Self(Allowed::Flag)
    }

    /// Any text.
    #[must_use]
    pub fn text() -> Self {
        Self(Allowed::Text)
    }

    /// One text of `choices`.
    #[must_use]
    pub fn choice(choices: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(Allowed::Choice(choices.into_iter().map(Into::into).collect()))
    }

    /// A value that reads as `T` and passes `valid`.
    #[must_use]
    pub fn check<T: Setting + 'static>(valid: impl Fn(&T) -> bool + Send + Sync + 'static) -> Self {
        Self(Allowed::Check(Arc::new(move |value| T::from_setting(value).is_some_and(|value| valid(&value)))))
    }
}

impl fmt::Debug for SettingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.describe())
    }
}

/// One key of a [`Schema`].
#[derive(Clone, PartialEq)]
pub(crate) struct Rule {
    key: String,
    allowed: Allowed,
    /// `None` for an optional key: an invalid value is removed instead of replaced.
    default: Option<SettingValue>,
}

impl Rule {
    /// Whether `value` is valid for this key.
    pub(crate) fn accepts(&self, value: &SettingValue) -> bool {
        self.allowed.accepts(value)
    }

    /// What the key must hold, in words.
    pub(crate) fn describe(&self) -> String {
        self.allowed.describe()
    }

    /// The value written in place of an invalid one; `None` for an optional key.
    pub(crate) fn default_value(&self) -> Option<&SettingValue> {
        self.default.as_ref()
    }
}

/// The settings an application knows: every key with what it may hold and its default.
///
/// Give it to [`Settings::schema`](super::Settings::schema) to check a loaded file, and turn on
/// [`Settings::self_heal`](super::Settings::self_heal) to repair it: unknown keys are removed and
/// invalid values are replaced by their default. Values known only while running, such as the
/// installed themes and languages, are passed in when the schema is built.
///
/// Two capabilities are opt-in, each on its own:
///
/// - [`Schema::optional`] declares a key without a default. A valid value is kept, an invalid one
///   is removed, and a missing one stays missing.
/// - [`Schema::open`] keeps every key under a table as it is, for keys the application does not
///   own, such as plugins' settings.
///
/// A key missing from the file is never written into it: reading it gives `None` and the
/// application uses its default.
///
/// ```
/// use qframe::storage::{Schema, Settings};
///
/// let schema = Schema::builtin()
///     .choice(Settings::LANGUAGE, ["en", "tr"], "en")
///     .choice("deploy.region", ["eu-west", "us-east"], "eu-west")
///     .check("editor.tab-width", 4u16, |width| (1..=16).contains(width));
/// let settings = Settings::parse_str("settings.toml", "language = \"sjds\"\ncolor = \"red\"\n")
///     .schema(schema)
///     .self_heal(true);
/// assert_eq!(settings.language().as_deref(), Some("en"));
/// assert!(settings.value("color").is_none());
/// ```
#[derive(Clone, Default, PartialEq)]
pub struct Schema {
    rules: Vec<Rule>,
    /// Tables whose keys are kept unchecked, without a trailing dot.
    open: Vec<String>,
}

impl Schema {
    /// The keys the framework itself reads: `theme` and `language` (any string; `monochrome`
    /// and `en`), `icons` (`auto`, `nerd`, `unicode`, `ascii`; `auto`), `reduced-motion`
    /// (`false`), `pillar` (`thick`, `thin`; `thick`) and `slide` (`true`). Declare `theme` and
    /// `language` again with [`Schema::choice`] to accept only what is installed.
    #[must_use]
    pub fn builtin() -> Self {
        use super::Settings;
        Self::default()
            .text(Settings::THEME, "monochrome")
            .text(Settings::LANGUAGE, "en")
            .choice(Settings::ICONS, IconMode::ALL.map(IconMode::name), IconMode::Auto.name())
            .flag(Settings::REDUCED_MOTION, false)
            .choice(Settings::PILLAR, PillarStyle::ALL.map(PillarStyle::name), PillarStyle::Thick.name())
            .flag(Settings::SLIDE, true)
    }

    /// A `true`/`false` key. Declaring a key again replaces its earlier rule.
    #[must_use]
    pub fn flag(self, key: &str, default: bool) -> Self {
        self.rule(key, Allowed::Flag, Some(SettingValue::Bool(default)))
    }

    /// A key holding any text.
    #[must_use]
    pub fn text(self, key: &str, default: impl Into<String>) -> Self {
        self.rule(key, Allowed::Text, Some(SettingValue::Text(default.into())))
    }

    /// A key holding one text of `choices`, e.g. the installed theme ids.
    #[must_use]
    pub fn choice(self, key: &str, choices: impl IntoIterator<Item = impl Into<String>>, default: &str) -> Self {
        let SettingKind(allowed) = SettingKind::choice(choices);
        self.rule(key, allowed, Some(SettingValue::Text(default.to_owned())))
    }

    /// A key whose value must read as `T` and pass `valid`, e.g. a number in a range or a name
    /// without spaces.
    #[must_use]
    pub fn check<T: Setting + 'static>(
        self,
        key: &str,
        default: T,
        valid: impl Fn(&T) -> bool + Send + Sync + 'static,
    ) -> Self {
        let SettingKind(allowed) = SettingKind::check(valid);
        self.rule(key, allowed, Some(default.to_setting()))
    }

    /// A key without a default, holding `kind`, e.g. a note an application stores only once the
    /// user writes one. With self-healing on, a valid value is kept, an invalid one is removed
    /// (there is nothing to replace it with) and a missing one is not added; reading a missing
    /// key gives `None`.
    ///
    /// ```
    /// use qframe::storage::{Schema, SettingKind, Settings};
    ///
    /// let schema = Schema::default().optional("deploy.note", SettingKind::text());
    /// let healed = Settings::parse_str("settings.toml", "[deploy]\nnote = 42\n").schema(schema).self_heal(true);
    /// assert_eq!(healed.get::<String>("deploy.note"), None);
    /// assert_eq!(healed.to_toml(), "");
    /// ```
    #[must_use]
    pub fn optional(self, key: &str, kind: SettingKind) -> Self {
        let SettingKind(allowed) = kind;
        self.rule(key, allowed, None)
    }

    /// Keeps every key under the dotted table `prefix` as it is: `open("plugins")` keeps
    /// `[plugins]` and every table below it, unchecked and never removed, for settings the
    /// application does not own. Keys declared under the prefix are still checked by their rule.
    /// The prefix names a table, so a plain `plugins = …` key is not under it; `""` opens nothing.
    ///
    /// ```
    /// use qframe::storage::{Schema, Settings};
    ///
    /// let schema = Schema::default().open("plugins").flag("plugins.enabled", true);
    /// let text = "[plugins]\nenabled = \"yes\"\n\n[plugins.git]\nsign = true\n";
    /// let healed = Settings::parse_str("settings.toml", text).schema(schema).self_heal(true);
    /// assert_eq!(healed.to_toml(), "[plugins]\nenabled = true\n\n[plugins.git]\nsign = true\n");
    /// ```
    #[must_use]
    pub fn open(mut self, prefix: &str) -> Self {
        let prefix = prefix.trim_end_matches('.');
        if !prefix.is_empty() && !self.open.iter().any(|open| open == prefix) {
            self.open.push(prefix.to_owned());
        }
        self
    }

    fn rule(mut self, key: &str, allowed: Allowed, default: Option<SettingValue>) -> Self {
        self.rules.retain(|rule| rule.key != key);
        self.rules.push(Rule { key: key.to_owned(), allowed, default });
        self
    }

    /// The rule of `key`, if the schema knows it.
    pub(crate) fn get(&self, key: &str) -> Option<&Rule> {
        self.rules.iter().find(|rule| rule.key == key)
    }

    /// Whether `key` lies under an [open](Self::open) prefix.
    pub(crate) fn is_open(&self, key: &str) -> bool {
        self.open.iter().any(|prefix| key.strip_prefix(prefix.as_str()).is_some_and(|rest| rest.starts_with('.')))
    }
}

impl fmt::Debug for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Schema")
            .field("rules", &self.rules.iter().map(|rule| (&rule.key, rule.describe())).collect::<Vec<_>>())
            .field("open", &self.open)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_accept_only_their_values() {
        let schema = Schema::builtin().check("editor.tab-width", 4u16, |width| (1..=16).contains(width));
        let icons = schema.get("icons").expect("built in");
        assert!(icons.accepts(&SettingValue::Text("ascii".into())));
        assert!(!icons.accepts(&SettingValue::Text("sparkly".into())));
        assert!(!icons.accepts(&SettingValue::Bool(true)));
        assert_eq!(icons.describe(), "one of auto, nerd, unicode, ascii");
        let slide = schema.get("slide").expect("built in");
        assert!(slide.accepts(&SettingValue::Bool(false)));
        assert!(!slide.accepts(&SettingValue::Text("true".into())));
        let width = schema.get("editor.tab-width").expect("declared");
        assert!(width.accepts(&SettingValue::Integer(8)));
        assert!(!width.accepts(&SettingValue::Integer(40)));
        assert!(!width.accepts(&SettingValue::Text("8".into())));
        assert_eq!(width.default_value(), Some(&SettingValue::Integer(4)));
        assert!(schema.get("color").is_none());
    }

    #[test]
    fn declaring_a_key_again_replaces_it() {
        let schema = Schema::builtin().choice("language", ["en", "tr"], "tr");
        assert_eq!(schema.clone().choice("language", ["en", "tr"], "tr"), schema, "one rule per key");
        let language = schema.get("language").expect("declared");
        assert!(!language.accepts(&SettingValue::Text("sjds".into())));
        assert_eq!(language.default_value(), Some(&SettingValue::Text("tr".into())));
        assert_eq!(Schema::builtin(), Schema::builtin());
        assert_ne!(schema, Schema::builtin());
    }

    #[test]
    fn optional_rules_have_no_default_and_open_prefixes_name_tables() {
        let schema = Schema::default()
            .optional("deploy.note", SettingKind::text())
            .optional("deploy.retries", SettingKind::check(|retries: &u8| (1..=10).contains(retries)))
            .open("plugins")
            .open("plugins.")
            .open("");
        let note = schema.get("deploy.note").expect("declared");
        assert_eq!(note.default_value(), None);
        assert!(note.accepts(&SettingValue::Text("freeze".into())) && !note.accepts(&SettingValue::Integer(1)));
        let retries = schema.get("deploy.retries").expect("declared");
        assert!(retries.accepts(&SettingValue::Integer(3)) && !retries.accepts(&SettingValue::Integer(30)));
        assert!(schema.is_open("plugins.git") && schema.is_open("plugins.git.sign") && schema.is_open("plugins."));
        assert!(!schema.is_open("plugins") && !schema.is_open("plugins-extra.x") && !schema.is_open("deploy.note"));
        assert_eq!(format!("{schema:?}"), format!("{:?}", schema.clone().open("plugins")), "each prefix once");
        let declared = Schema::default().text("deploy.note", "").optional("deploy.note", SettingKind::text());
        assert_eq!(declared.get("deploy.note").and_then(Rule::default_value), None, "declaring again replaces");
        assert_eq!(format!("{:?}", SettingKind::choice(["a", "b"])), "one of a, b");
    }
}
