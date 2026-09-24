use super::*;
use crate::diagnostics::Severity;

const SET: &str = r#"
[meta]
name = "Test"

[icons]
check = { nerd = "N", unicode = "✓", ascii = "v" }
spin = { nerd = "ab", unicode = "◐◓", ascii = "-/" }
bad = { nerd = "x", unicode = "x", ascii = "[x]" }
half = { nerd = "x" }
"#;

fn registry() -> IconSetRegistry {
    let mut registry = IconSetRegistry::builtin();
    assert!(registry.add_source("test", "test.toml", SET));
    registry
}

#[test]
fn picks_glyph_for_mode() {
    let mut icons = registry().icons("test", &BTreeMap::new(), GlyphMode::Nerd);
    assert_eq!(icons.glyph("check"), "N");
    icons.set_mode(GlyphMode::Unicode);
    assert_eq!(icons.glyph("check"), "✓");
    icons.set_mode(GlyphMode::Ascii);
    assert_eq!(icons.glyph("check"), "v");
    assert_eq!(icons.glyph("nope"), "⟦nope⟧");
}

#[test]
fn splits_frames_by_grapheme() {
    let icons = registry().icons("test", &BTreeMap::new(), GlyphMode::Unicode);
    assert_eq!(icons.frames("spin"), vec!["◐", "◓"]);
}

#[test]
fn broken_entries_are_reported_and_skipped() {
    let registry = registry();
    let messages: Vec<String> = registry.diagnostics().iter().map(ToString::to_string).collect();
    assert!(messages.iter().any(|m| m.contains("brackets are not allowed")), "{messages:?}");
    assert!(messages.iter().any(|m| m.contains("missing its `ascii` glyph")), "{messages:?}");
    let icons = registry.icons("test", &BTreeMap::new(), GlyphMode::Ascii);
    assert!(!icons.contains("bad"));
    assert!(!icons.contains("half"));
}

#[test]
fn unknown_sections_and_meta_keys_are_reported_with_their_line() {
    let mut registry = IconSetRegistry::builtin();
    let set = "[meta]\nname = 7\nauthor = \"x\"\n[icon]\ncheck = 1\n[icons]\n";
    assert!(registry.add_source("typo", "typo.toml", set));
    let lines: Vec<(usize, &str)> = registry
        .diagnostics()
        .iter()
        .map(|d| (d.location.as_ref().map_or(0, |l| l.line), d.message.as_str()))
        .collect();
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines.iter().any(|(line, m)| *line == 2 && m.contains("meta.name must be a string")), "{lines:?}");
    assert!(lines.iter().any(|(line, m)| *line == 3 && m.contains("unknown key `meta.author`")), "{lines:?}");
    assert!(lines.iter().any(|(line, m)| *line == 4 && m.contains("unknown section `icon`")), "{lines:?}");
    assert_eq!(registry.list().iter().find(|(id, _)| id == "typo").map(|(_, name)| name.as_str()), Some("typo"));
}

#[test]
fn pillar_has_a_short_form_thick_thin_or_one_cell() {
    for (text, glyph) in [("thick", "▌"), ("thin", "▎"), ("▍", "▍")] {
        let mut registry = IconSetRegistry::builtin();
        let set = format!("[meta]\nname = \"P\"\n[icons]\npillar = \"{text}\"\n");
        assert!(registry.add_source("p", "p.toml", &set));
        let mut icons = registry.icons("p", &BTreeMap::new(), GlyphMode::Unicode);
        assert_eq!(icons.glyph(PILLAR), glyph);
        icons.set_mode(GlyphMode::Ascii);
        assert_eq!(icons.glyph(PILLAR), " ", "ASCII shows the pillar as a coloured cell");
    }
    let mut registry = IconSetRegistry::builtin();
    assert!(registry.add_source("p", "p.toml", "[meta]\nname = \"P\"\n[icons]\npillar = \"wide\"\n"));
    let messages: Vec<String> = registry.diagnostics().iter().map(ToString::to_string).collect();
    assert!(messages.iter().any(|m| m.contains("single one-cell character")), "{messages:?}");
}

#[test]
fn overrides_replace_single_icons() {
    let overrides = BTreeMap::from([(
        "check".to_owned(),
        IconGlyphs { nerd: "C".to_owned(), unicode: "C".to_owned(), ascii: "C".to_owned() },
    )]);
    let icons = registry().icons("test", &overrides, GlyphMode::Nerd);
    assert_eq!(icons.glyph("check"), "C");
    assert_eq!(icons.glyph("spin"), "ab");
}

#[test]
fn unknown_set_falls_back_to_default() {
    let icons = IconSetRegistry::builtin().icons("missing", &BTreeMap::new(), GlyphMode::Unicode);
    assert_eq!(icons.glyph("check"), "✓");
}

#[test]
fn icon_mode_names_round_trip() {
    for mode in IconMode::ALL {
        assert_eq!(IconMode::from_name(mode.name()), Some(mode));
    }
    assert_eq!(IconMode::from_name(" NERD "), Some(IconMode::Nerd));
    assert_eq!(IconMode::from_name("emoji"), None);
}

/// An application's own icons: two keys the built-in set lacks and one it has.
const APP: &str = r#"[meta]
name = "App"

[icons]
"category.internet" = { nerd = "I", unicode = "◎", ascii = "@" }
"source.aur" = { nerd = "A", unicode = "◇", ascii = "a" }
check = { nerd = "!", unicode = "!", ascii = "!" }
"#;

fn with_app() -> IconSetRegistry {
    let mut registry = IconSetRegistry::builtin();
    assert!(registry.add_source("app", "app.toml", APP));
    assert!(registry.diagnostics().is_empty(), "{:?}", registry.diagnostics());
    registry
}

#[test]
fn an_application_key_is_drawn_whatever_set_is_chosen_in_all_three_modes() {
    let registry = with_app();
    for set in ["default", "missing"] {
        let mut icons = registry.icons(set, &BTreeMap::new(), GlyphMode::Nerd);
        assert_eq!(icons.glyph("category.internet"), "I", "{set}");
        icons.set_mode(GlyphMode::Unicode);
        assert_eq!(icons.glyph("category.internet"), "◎", "{set}");
        icons.set_mode(GlyphMode::Ascii);
        assert_eq!(icons.glyph("category.internet"), "@", "{set}");
        assert_eq!(icons.glyph("source.aur"), "a", "{set}");
    }
}

#[test]
fn a_built_in_key_keeps_its_glyph_unless_the_application_set_is_the_chosen_one() {
    let registry = with_app();
    let icons = registry.icons("default", &BTreeMap::new(), GlyphMode::Unicode);
    assert_eq!(icons.glyph("check"), "✓", "an application set does not restyle the framework's icons everywhere");
    let chosen = registry.icons("app", &BTreeMap::new(), GlyphMode::Unicode);
    assert_eq!(chosen.glyph("check"), "!", "named by a theme, the set restyles them");
    assert_eq!(chosen.glyph("category.internet"), "◎");
}

#[test]
fn the_chosen_set_and_a_theme_override_win_over_an_application_key() {
    let mut registry = with_app();
    let restyled =
        "[meta]\nname = \"R\"\n[icons]\n\"category.internet\" = { nerd = \"R\", unicode = \"R\", ascii = \"R\" }\n";
    assert!(registry.add_source("restyled", "restyled.toml", restyled));
    let icons = registry.icons("restyled", &BTreeMap::new(), GlyphMode::Ascii);
    assert_eq!(icons.glyph("category.internet"), "R", "the set the theme chose");
    assert_eq!(icons.glyph("source.aur"), "a", "keys the chosen set lacks still come from the application");
    let overrides = BTreeMap::from([(
        "source.aur".to_owned(),
        IconGlyphs { nerd: "T".to_owned(), unicode: "T".to_owned(), ascii: "T".to_owned() },
    )]);
    let icons = registry.icons("default", &overrides, GlyphMode::Ascii);
    assert_eq!(icons.glyph("source.aur"), "T", "a theme's single icon");
}

#[test]
fn a_later_application_set_wins_a_key_two_of_them_give() {
    let mut registry = with_app();
    let later = "[icons]\n\"source.aur\" = { nerd = \"L\", unicode = \"L\", ascii = \"L\" }\n";
    assert!(registry.add_source("later", "later.toml", later));
    let icons = registry.icons("default", &BTreeMap::new(), GlyphMode::Nerd);
    assert_eq!(icons.glyph("source.aur"), "L");
}

#[test]
fn a_missing_column_is_reported_where_the_icon_is_and_a_plainer_glyph_stands_in() {
    let mut registry = IconSetRegistry::builtin();
    let set = "[icons]\nno-nerd = { unicode = \"◎\", ascii = \"@\" }\nonly-ascii = { ascii = \"a\" }\n\
               no-ascii = { nerd = \"N\", unicode = \"◎\" }\n";
    assert!(registry.add_source("gaps", "gaps.toml", set));
    let places: Vec<(String, Severity)> = registry
        .diagnostics()
        .iter()
        .map(|d| (d.location.as_ref().map(ToString::to_string).unwrap_or_default(), d.severity))
        .collect();
    assert_eq!(
        places,
        [
            ("gaps.toml:2:11".to_owned(), Severity::Warning),
            ("gaps.toml:3:14".to_owned(), Severity::Warning),
            ("gaps.toml:3:14".to_owned(), Severity::Warning),
            ("gaps.toml:4:12".to_owned(), Severity::Error),
        ],
        "{:?}",
        registry.diagnostics()
    );
    let mut icons = registry.icons("default", &BTreeMap::new(), GlyphMode::Nerd);
    assert_eq!(icons.glyph("no-nerd"), "◎", "the Unicode glyph stands in: a Nerd Font draws it too");
    assert_eq!(icons.glyph("only-ascii"), "a");
    icons.set_mode(GlyphMode::Unicode);
    assert_eq!(icons.glyph("only-ascii"), "a");
    assert!(!icons.contains("no-ascii"), "nothing plainer can stand in for ASCII, so the icon is skipped");
}

#[test]
fn a_status_strip_finds_its_keys_in_every_mode() {
    let keys = [
        "cpu",
        "memory",
        "battery-full",
        "battery-half",
        "battery-empty",
        "battery-charging",
        "terminal",
        "session",
        "network-down",
        "network-up",
    ];
    let registry = IconSetRegistry::builtin();
    assert!(registry.diagnostics().is_empty(), "{:?}", registry.diagnostics());
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        let icons = registry.icons("default", &BTreeMap::new(), mode);
        let glyphs: Vec<String> = keys.iter().map(|key| icons.glyph(key).into_owned()).collect();
        for (key, glyph) in keys.iter().zip(&glyphs) {
            assert!(icons.contains(key), "{key} is missing in {mode:?}");
            assert_eq!(crate::text::width(glyph), 1, "{key} in {mode:?} is {glyph:?}");
            let private = glyph.chars().all(|c| matches!(u32::from(c), 0xE000..=0xF8FF | 0xF0000..=0xFFFFD));
            assert_eq!(private, mode == GlyphMode::Nerd, "{key} in {mode:?} is {glyph:?}");
        }
        if mode != GlyphMode::Ascii {
            // The four charges of the battery and the two directions are told apart by shape alone.
            let battery = &glyphs[2..6];
            assert!(battery.iter().enumerate().all(|(i, a)| battery[i + 1..].iter().all(|b| a != b)), "{battery:?}");
            assert_ne!(glyphs[8], glyphs[9]);
        }
    }
}
