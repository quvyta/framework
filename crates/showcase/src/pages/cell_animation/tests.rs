//! Studio page tests: the built-ins in every mode, editing, copying, remembering and resetting.

use qframe::animation::{CellAnimation, Playback, parse_animations};
use qframe::icons::GlyphMode;
use qframe::prelude::*;
use qframe::storage::Settings;

use super::*;
use crate::app::Showcase;
use crate::tests::showcase_tall;

/// The page, tall enough for the studio and the event log.
fn page(showcase: Showcase) -> Harness<Showcase> {
    showcase_tall(showcase, PAGE, 140)
}

fn studio(h: &Harness<Showcase>) -> &State {
    &h.app().pages.cell_animation
}

fn log(h: &Harness<Showcase>) -> Vec<String> {
    h.app().log.recent(PAGE, 50).iter().map(|entry| format!("{} {}", entry.source, entry.message)).collect()
}

#[test]
fn every_built_in_plays_in_the_three_glyph_modes_side_by_side() {
    let h = page(Showcase::new());
    let screen = h.screen();
    let row = |name: &str| screen.lines().find(|line| line.contains(name)).unwrap_or_default().to_owned();
    for name in ["spinner-arc", "spinner-done", "spinner-dots", "spinner-pulse", "spinner-slices"] {
        assert!(screen.contains(name), "{name} is listed:\n{screen}");
    }
    // A second into the page, the arc is back on its first frame in every mode.
    let arc = row("spinner-arc");
    assert!(arc.contains('◜') && arc.contains('-'), "Unicode and ASCII columns at once: {arc}");
    let slices = row("spinner-slices");
    assert!(
        slices.contains('\u{F0A9E}') || slices.chars().any(|c| ('\u{F0A9E}'..='\u{F0AA5}').contains(&c)),
        "{slices}"
    );
    assert!(slices.contains("loop") && slices.contains("8 frames"), "{slices}");
}

#[test]
fn pressing_a_built_in_opens_it_in_the_studio() {
    let mut h = page(Showcase::new());
    h.click_text("spinner-pulse");
    assert_eq!(studio(&h).draft().name, "spinner-pulse");
    assert_eq!(studio(&h).draft().frames[0].color, "pulse($muted, $fg)");
    assert!(log(&h).iter().any(|line| line == "Select#start open spinner-pulse"), "{:?}", log(&h));
}

#[test]
fn editing_a_frame_updates_the_preview_and_is_logged() {
    let mut h = page(Showcase::new());
    assert!(!h.screen().contains('◈'));
    h.send(send(Msg::Glyph(0, 1, "◈".to_owned())));
    assert_eq!(studio(&h).built.glyph(0, GlyphMode::Unicode), "◈");
    assert_eq!(studio(&h).built.glyph(0, GlyphMode::Nerd), "\u{EE06}", "the frame keeps its own Nerd Font glyph");
    let screen = h.screen();
    let shown = screen.lines().filter(|line| line.contains('◈')).count();
    assert!(shown >= 3, "the large and normal previews and the frame row show it:\n{screen}");
    assert!(screen.contains("Changed from the built-in"), "{screen}");
    assert!(log(&h).iter().any(|line| line.contains("frame 1 unicode = \"◈\"")), "{:?}", log(&h));
}

#[test]
fn broken_fields_are_explained_under_their_frame() {
    let mut h = page(Showcase::new());
    h.send(send(Msg::Glyph(1, 1, "中".to_owned())));
    h.send(send(Msg::Color(2, "mix($accent)".to_owned())));
    h.send(send(Msg::Color(3, "$sparkle".to_owned())));
    h.send(send(Msg::Name("Arc Soft".to_owned())));
    let screen = h.screen();
    assert!(screen.contains("`中` is 2 cells wide"), "{screen}");
    assert!(screen.contains("expected `,` in colour"), "{screen}");
    assert!(screen.contains("unknown colour token `$sparkle`"), "{screen}");
    assert!(screen.contains("may use only lowercase letters"), "{screen}");
    h.click_text("Copy TOML");
    assert_eq!(h.clipboard(), None, "nothing is copied under an invalid name");
}

#[test]
fn copy_toml_puts_the_animation_block_on_the_clipboard() {
    let mut h = page(Showcase::new());
    h.send(send(Msg::Color(0, "#38BDF8".to_owned())));
    h.click_text("Copy TOML");
    let copied = h.clipboard().expect("copied").to_owned();
    assert!(copied.starts_with("[animations.spinner-arc]\nframe = \"spinner\"\n"), "{copied}");
    assert!(
        copied.contains("{ nerd = \"\\uEE06\", unicode = \"◜\", ascii = \"-\", color = \"#38BDF8\" },"),
        "{copied}"
    );
    let (read, diagnostics) = parse_animations("copied.toml", &copied);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(read[0].1, *studio(&h).built, "the copy reads back as the preview");
    assert!(log(&h).iter().any(|line| line.starts_with("Button#copy-toml copied [animations.spinner-arc]")));
}

#[test]
fn working_animations_survive_a_restart_and_reset_returns_to_the_built_in() {
    let mut h = page(Showcase::new());
    h.send(send(Msg::Glyph(0, 1, "◐".to_owned())));
    h.send(send(Msg::Open(0)));
    h.send(send(Msg::Add)).send(send(Msg::Glyph(1, 2, "o".to_owned())));
    h.send(send(Msg::Playback(1)));
    let settings = h.app().pages.storage.settings.clone();
    let saved = settings.get::<String>(STUDIO_KEY).expect("remembered");
    assert!(saved.contains("[animations.spinner-arc]") && saved.contains("[animations.my-animation]"), "{saved}");
    let written = settings.to_toml();
    let reread = Settings::parse_str("settings.toml", &written).schema(storage::schema(h.env())).self_heal(true);
    assert_eq!(reread.diagnostics(), &[], "the open studio prefix keeps the key");

    let mut again = page(Showcase::with_settings(reread));
    let restored = studio(&again);
    let names: Vec<&str> = restored.drafts.iter().map(|draft| draft.name.as_str()).collect();
    assert_eq!(names, ["spinner-arc", "my-animation"]);
    assert_eq!(restored.draft().frames[0].glyphs[1], "◐");
    assert!(restored.restored.is_empty());
    assert_eq!(restored.drafts[1].playback, Playback::Once);

    again.click_text("Reset to built-in");
    let arc: &CellAnimation = &studio(&again).builtin("spinner-arc").expect("built in").clone();
    assert_eq!(*studio(&again).built, *arc);
    let saved = again.app().pages.storage.settings.get::<String>(STUDIO_KEY).expect("the new one stays");
    assert!(!saved.contains("spinner-arc") && saved.contains("[animations.my-animation]"), "{saved}");
    assert!(log(&again).iter().any(|line| line == "Button#reset spinner-arc reset to the built-in"));
}

#[test]
fn frames_are_added_moved_and_removed_and_a_broken_save_is_reported() {
    let mut h = page(Showcase::new());
    h.send(send(Msg::Open(0)));
    h.send(send(Msg::Add)).send(send(Msg::Glyph(1, 2, "o".to_owned())));
    h.send(send(Msg::Up(1)));
    let ascii: Vec<&str> = studio(&h).draft().frames.iter().map(|frame| frame.glyphs[2].as_str()).collect();
    assert_eq!(ascii, ["o", "*"]);
    h.send(send(Msg::Rest(2))).send(send(Msg::Remove(1)));
    assert_eq!(studio(&h).draft().frames.len(), 1);
    assert_eq!(studio(&h).draft().rest, None, "a rest frame that is gone rests where the playback does");
    h.send(send(Msg::Remove(0)));
    assert_eq!(studio(&h).draft().frames.len(), 1, "the last frame stays");
    assert!(log(&h).iter().any(|line| line.ends_with("frame 2 moved to 1")), "{:?}", log(&h));

    let mut settings = Settings::in_memory();
    settings
        .set(STUDIO_KEY, "[animations.ok]\nframes = [{ ascii = \"-\" }]\n[animations.bad]\nframes = []\n".to_owned());
    let h = page(Showcase::with_settings(settings));
    assert_eq!(studio(&h).drafts.len(), 1);
    assert!(h.screen().contains("studio:4:10: error: animation `bad` needs at least one frame"), "{}", h.screen());
}
