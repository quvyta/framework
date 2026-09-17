//! The files compiled into the framework must be complete and warning-free: applications
//! fall back to them when their own files are broken.

use std::collections::BTreeMap;

use qframe::i18n::I18n;
use qframe::icons::{GlyphMode, IconSetRegistry};
use qframe::keymap::Keymap;
use qframe::theme::{REQUIRED_COLORS, ThemeRegistry};

#[test]
fn every_builtin_theme_resolves_without_diagnostics() {
    let registry = ThemeRegistry::builtin();
    assert!(registry.diagnostics().is_empty(), "{:?}", registry.diagnostics());
    for (id, _) in registry.list() {
        let resolved = registry.resolve(&id);
        let messages: Vec<String> = resolved.diagnostics.iter().map(ToString::to_string).collect();
        assert!(messages.is_empty(), "theme {id}: {messages:#?}");
        let theme = resolved.theme.expect("resolves");
        for token in REQUIRED_COLORS {
            assert!(theme.color(token).is_some(), "theme {id} lacks {token}");
        }
        assert_eq!(theme.icon_set(), "default");
        assert!(theme.typography("title").is_some_and(|t| t.flag("bold")));
    }
}

#[test]
fn default_icon_set_is_valid_in_every_mode() {
    let registry = IconSetRegistry::builtin();
    assert!(registry.diagnostics().is_empty(), "{:?}", registry.diagnostics());
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        let icons = registry.icons("default", &BTreeMap::new(), mode);
        assert!(icons.keys().count() >= 20);
        for key in icons.keys() {
            assert!(!icons.glyph(key).is_empty(), "{key} empty in {mode:?}");
        }
    }
}

#[test]
fn builtin_locales_are_complete() {
    let i18n = I18n::builtin();
    assert!(i18n.diagnostics().is_empty(), "{:?}", i18n.diagnostics());
    for (code, _) in i18n.list() {
        assert_eq!(i18n.missing_keys(&code, "en"), Vec::<String>::new(), "locale {code}");
    }
}

#[test]
fn builtin_keymap_is_valid_and_labelled() {
    let keymap = Keymap::builtin();
    assert!(keymap.conflicts().is_empty());
    let i18n = I18n::builtin();
    for (scope, action, chords) in keymap.iter() {
        assert!(!chords.is_empty(), "{action} unbound");
        let label = i18n.translate(&scope.label_key(action), &[]);
        assert!(!label.starts_with('⟦'), "{action} has no label");
    }
}
