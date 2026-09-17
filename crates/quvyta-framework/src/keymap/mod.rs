//! Keymaps: named actions bound to key chords, overridable from files.
//!
//! ```toml
//! [global]
//! quit = "ctrl+q"
//!
//! [app]
//! save = ["ctrl+s", "f2"]
//! ```
//!
//! A chord is modifiers and a key joined with `+`. Modifier and key names are case-insensitive
//! (`Ctrl+Enter` is `ctrl+enter`), but a letter's case counts: `"S"` means `shift+s`, the
//! chord a terminal reports for a typed capital S, while `"s"` is the plain key.
//!
//! `[global]` holds framework actions; `[app]` holds the application's own. An application
//! binding wins over a global one on the same chord. Hint bars take their labels from the
//! locale key `quvyta.keys.<action>` for global actions and `keys.<action>` for app actions.

mod chord;

use std::collections::BTreeMap;
use std::ops::Range;

pub use chord::{Key, KeyChord, Modifiers};

use crate::assets;
use crate::diagnostics::Diagnostic;
use crate::doc::{Doc, Value};

/// Which table an action belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    /// Framework actions, in `[global]`.
    Global,
    /// Application actions, in `[app]`.
    App,
}

impl Scope {
    fn table(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::App => "app",
        }
    }

    /// The locale key holding the hint label of `action`.
    #[must_use]
    pub fn label_key(self, action: &str) -> String {
        match self {
            Self::Global => format!("quvyta.keys.{action}"),
            Self::App => format!("keys.{action}"),
        }
    }
}

/// Actions and the chords that trigger them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keymap {
    bindings: BTreeMap<(Scope, String), Vec<KeyChord>>,
}

impl Keymap {
    /// The built-in keymap.
    #[must_use]
    pub fn builtin() -> Self {
        let mut report = Vec::new();
        let keymap = Self::parse("default.toml", assets::KEYMAP, &mut report);
        debug_assert!(report.is_empty(), "built-in keymap must be valid: {report:?}");
        keymap
    }

    /// Parses a keymap file. Broken entries are reported and skipped.
    #[must_use]
    pub fn parse(file: &str, text: &str, report: &mut Vec<Diagnostic>) -> Self {
        let doc = Doc::new(file, text);
        let mut keymap = Self::default();
        let root = match doc.parse() {
            Ok(root) => root,
            Err(diagnostic) => {
                report.push(diagnostic);
                return keymap;
            }
        };
        for (section, value) in &root {
            let scope = match section.get_ref().as_ref() {
                "global" => Scope::Global,
                "app" => Scope::App,
                other => {
                    report.push(doc.error(&value.span(), format!("unknown section `{other}`; use [global] or [app]")));
                    continue;
                }
            };
            let table = match doc.table(value, scope.table()) {
                Ok(table) => table,
                Err(diagnostic) => {
                    report.push(diagnostic);
                    continue;
                }
            };
            for (action, binding) in table {
                let action = action.get_ref().to_string();
                if let Some(chords) = parse_binding(&doc, scope, &action, binding, report) {
                    keymap.bindings.insert((scope, action), chords);
                }
            }
        }
        keymap
    }

    /// Replaces every action that `other` defines. An action bound to an empty list in
    /// `other` becomes unbound.
    pub fn overlay(&mut self, other: &Self) {
        for (key, chords) in &other.bindings {
            self.bindings.insert(key.clone(), chords.clone());
        }
    }

    /// Binds `action` in code, replacing any earlier binding.
    pub fn bind(&mut self, scope: Scope, action: &str, chords: &[KeyChord]) {
        self.bindings.insert((scope, action.to_owned()), chords.to_vec());
    }

    /// The action `chord` triggers; application actions win over global ones.
    #[must_use]
    pub fn action_for(&self, chord: KeyChord) -> Option<(Scope, &str)> {
        [Scope::App, Scope::Global].into_iter().find_map(|scope| {
            self.bindings
                .iter()
                .find(|((s, _), chords)| *s == scope && chords.contains(&chord))
                .map(|((s, action), _)| (*s, action.as_str()))
        })
    }

    /// The chords bound to `action`.
    #[must_use]
    pub fn chords_for(&self, scope: Scope, action: &str) -> &[KeyChord] {
        self.bindings.get(&(scope, action.to_owned())).map_or(&[], Vec::as_slice)
    }

    /// Every binding, sorted by scope and action.
    pub fn iter(&self) -> impl Iterator<Item = (Scope, &str, &[KeyChord])> {
        self.bindings.iter().map(|((scope, action), chords)| (*scope, action.as_str(), chords.as_slice()))
    }

    /// Warnings for chords bound to more than one action within the same scope.
    #[must_use]
    pub fn conflicts(&self) -> Vec<Diagnostic> {
        let mut owners: BTreeMap<(Scope, KeyChord), Vec<&str>> = BTreeMap::new();
        for ((scope, action), chords) in &self.bindings {
            for chord in chords {
                owners.entry((*scope, *chord)).or_default().push(action);
            }
        }
        owners
            .into_iter()
            .filter(|(_, actions)| actions.len() > 1)
            .map(|((scope, chord), actions)| {
                Diagnostic::warning(
                    None,
                    format!("`{chord}` is bound to several [{}] actions: {}", scope.table(), actions.join(", ")),
                )
            })
            .collect()
    }
}

/// The chords of `binding`, one key or a list of keys, for `action`. Broken keys are reported and
/// left out; a binding of another type is reported and gives `None`.
fn parse_binding(
    doc: &Doc<'_>,
    scope: Scope,
    action: &str,
    binding: &Value<'_>,
    report: &mut Vec<Diagnostic>,
) -> Option<Vec<KeyChord>> {
    let texts: Vec<(&str, Range<usize>)> = if let Some(text) = binding.get_ref().as_str() {
        vec![(text, binding.span())]
    } else if let Some(items) = binding.get_ref().as_array() {
        items
            .iter()
            .filter_map(|item| match doc.string(item, &format!("{}.{action}", scope.table())) {
                Ok(text) => Some((text, item.span())),
                Err(diagnostic) => {
                    report.push(diagnostic);
                    None
                }
            })
            .collect()
    } else {
        report.push(doc.error(&binding.span(), format!("`{action}` must be a key like \"ctrl+s\" or a list of keys")));
        return None;
    };
    let mut chords = Vec::new();
    for (text, span) in texts {
        match text.parse::<KeyChord>() {
            Ok(chord) => chords.push(chord),
            Err(message) => report.push(doc.error(&span, message)),
        }
    }
    Some(chords)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(text: &str) -> KeyChord {
        text.parse().expect("valid chord")
    }

    #[test]
    fn builtin_binds_quit() {
        let keymap = Keymap::builtin();
        assert_eq!(keymap.action_for(chord("ctrl+q")), Some((Scope::Global, "quit")));
        assert_eq!(keymap.chords_for(Scope::Global, "debug"), &[chord("f12")]);
    }

    #[test]
    fn builtin_chords_keep_their_meaning_with_uppercase_letters_as_shift() {
        let keymap = Keymap::builtin();
        let shifted: Vec<String> = keymap
            .iter()
            .flat_map(|(_, action, chords)| {
                chords.iter().filter(|c| c.mods.shift).map(move |c| format!("{action} {c}"))
            })
            .collect();
        assert_eq!(shifted, ["focus-prev shift+tab"]);
        assert_eq!(keymap.action_for(chord("?")), Some((Scope::Global, "help")));
        let mut report = Vec::new();
        let user = Keymap::parse("user.toml", "[app]\nsave = \"S\"\nsearch = \"s\"\n", &mut report);
        assert!(report.is_empty(), "{report:?}");
        assert_eq!(user.action_for(chord("shift+s")), Some((Scope::App, "save")));
        assert_eq!(user.action_for(chord("s")), Some((Scope::App, "search")));
    }

    #[test]
    fn parses_lists_and_reports_bad_entries() {
        let mut report = Vec::new();
        let keymap = Keymap::parse(
            "app.toml",
            "[app]\nsave = [\"ctrl+s\", \"f2\"]\nbroken = \"ctrl+banana\"\nweird = 5\n[extra]\n",
            &mut report,
        );
        assert_eq!(keymap.chords_for(Scope::App, "save"), &[chord("ctrl+s"), chord("f2")]);
        assert_eq!(report.len(), 3, "{report:?}");
        assert_eq!(report[0].location.as_ref().map(|l| l.line), Some(3));
    }

    #[test]
    fn a_list_keeps_its_good_keys() {
        let mut report = Vec::new();
        let keymap = Keymap::parse("app.toml", "[app]\nsave = [\"ctrl+s\", 3, \"ctrl+banana\"]\n", &mut report);
        assert_eq!(keymap.chords_for(Scope::App, "save"), &[chord("ctrl+s")]);
        let messages: Vec<&str> = report.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert_eq!(messages[0], "app.save must be a string, found integer");
    }

    #[test]
    fn app_bindings_win_and_overlay_replaces_actions() {
        let mut keymap = Keymap::builtin();
        let mut report = Vec::new();
        let user = Keymap::parse("user.toml", "[global]\nquit = \"ctrl+w\"\n[app]\nclose = \"ctrl+q\"\n", &mut report);
        keymap.overlay(&user);
        assert_eq!(keymap.action_for(chord("ctrl+q")), Some((Scope::App, "close")));
        assert_eq!(keymap.action_for(chord("ctrl+w")), Some((Scope::Global, "quit")));
        assert_eq!(Scope::App.label_key("close"), "keys.close");
        assert_eq!(Scope::Global.label_key("quit"), "quvyta.keys.quit");
    }

    #[test]
    fn reports_conflicts_within_a_scope() {
        let mut keymap = Keymap::default();
        keymap.bind(Scope::App, "save", &[chord("ctrl+s")]);
        keymap.bind(Scope::App, "search", &[chord("ctrl+s")]);
        keymap.bind(Scope::Global, "other", &[chord("ctrl+s")]);
        let conflicts = keymap.conflicts();
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].message.contains("save, search"));
    }
}
