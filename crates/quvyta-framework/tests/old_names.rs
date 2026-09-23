//! The names the storage API had before the Quvyta ecosystem got its name still work, without a
//! warning, so an application whose gate denies warnings builds against the new version before it
//! moves over.

#![deny(warnings)]

use qframe::storage::{Ecosystem, Family, Scope, Source};

#[test]
fn the_old_type_name_is_the_ecosystem() {
    assert_eq!(Family::QUVYTA, Ecosystem::QUVYTA);
    let tools = Family::new("tools", "Tools");
    assert_eq!((tools.id(), tools.title()), ("tools", "Tools"));
    assert_eq!(tools, Ecosystem::new("tools", "Tools"));
}

#[test]
fn the_old_scope_and_source_names_stand_for_the_ecosystem() {
    assert_eq!(Scope::Family, Scope::Ecosystem);
    assert_eq!(Source::Family, Source::Ecosystem);
    // The old name works as a pattern too, and a match over it is still exhaustive.
    let said = match Source::Ecosystem {
        Source::Family => "follows",
        Source::App | Source::Detected => "own",
    };
    assert_eq!(said, "follows");
}

#[test]
fn the_old_icon_key_draws_the_ecosystem_s_mark() {
    use qframe::icons::GlyphMode;
    let mut icons = qframe::env::Env::builtin().icons().clone();
    assert!(icons.contains("family"), "an application asking by the old name finds the icon");
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        icons.set_mode(mode);
        assert_eq!(icons.glyph("family"), icons.glyph("ecosystem"), "{mode:?}: the same mark");
    }
    assert_eq!(icons.glyphs("family"), icons.glyphs("ecosystem"));
    assert!(!icons.keys().any(|key| key == "family"), "the set lists only the new name");
}
