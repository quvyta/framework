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

/// The meanings an application's main menu shows. Every icon set must answer all of them, or a
/// family application would have a menu with icons on only some of its rows.
const MENU_ICONS: [&str; 4] = ["project", "profile", "settings", "power"];

/// A launcher's categories, the entries without an icon of their own, the family's mark and the
/// marks on a window's title. Each is drawn in a strip of counted cells — a dock, a menu row, a
/// three-cell title mark — so a glyph two cells wide would push the strip apart.
const LAUNCHER_ICONS: [&str; 10] = [
    "category-system",
    "category-development",
    "category-network",
    "category-office",
    "category-media",
    "category-files",
    "family",
    "window-minimize",
    "window-maximize",
    "window-restore",
];

/// Marks every application in the family shows outside its main menu: the place the work is kept
/// and the button that answers what the keys do. Both sit in strips of counted cells, like the
/// menu's own rows, so both stay one cell wide.
const SHARED_ICONS: [&str; 2] = ["workspace", "help"];

#[test]
fn every_icon_set_answers_the_application_menu_in_every_mode_with_one_cell() {
    let registry = IconSetRegistry::builtin();
    for (id, _) in registry.list() {
        for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
            let icons = registry.icons(&id, &BTreeMap::new(), mode);
            for name in MENU_ICONS.into_iter().chain(LAUNCHER_ICONS).chain(SHARED_ICONS) {
                let glyph = icons.glyph(name);
                assert!(icons.keys().any(|key| key == name), "set {id} has no {name}");
                assert_eq!(qframe::text::width(&glyph), 1, "{name} is one cell in {mode:?} of set {id}: {glyph:?}");
                if mode == GlyphMode::Ascii {
                    assert!(
                        glyph.chars().all(|c| c.is_ascii_graphic()),
                        "{name} stays printable ASCII in set {id}: {glyph:?}"
                    );
                    assert!(!glyph.contains(['[', ']', '(', ')', '{', '}', '|']), "{name} decorates: {glyph:?}");
                }
            }
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
