//! Animation tests: reading files, fallbacks, playback, colours and writing TOML back.

use std::collections::BTreeMap;
use std::time::Duration;

use super::*;
use crate::diagnostics::Diagnostic;
use crate::icons::IconSetRegistry;
use crate::theme::ThemeRegistry;

fn theme() -> Theme {
    ThemeRegistry::builtin().resolve_or_default("monochrome").0
}

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// `(line, column, message)` of every diagnostic.
fn located(diagnostics: &[Diagnostic]) -> Vec<(usize, usize, String)> {
    diagnostics
        .iter()
        .map(|d| d.location.as_ref().map_or((0, 0, d.message.clone()), |l| (l.line, l.column, d.message.clone())))
        .collect()
}

const GOOD: &str = r##"
[animations.blink]
frame = "120ms"
playback = "bounce"
colors = "blend"
rest = 2
frames = [
  { nerd = "\uF111", unicode = "●", ascii = "*" },
  { unicode = "·", ascii = ".", color = "mix($accent, $fg, 40%)" },
  { ascii = "o", color = "#38BDF8", duration = "step" },
]
"##;

#[test]
fn reads_a_good_file_with_every_field() {
    let (found, diagnostics) = parse_animations("blink.toml", GOOD);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let [(name, blink)] = found.as_slice() else { panic!("one animation: {found:?}") };
    assert_eq!(name, "blink");
    assert_eq!(blink.time(), FrameTime::Fixed(ms(120)));
    assert_eq!(blink.play_mode(), Playback::Bounce);
    assert_eq!(blink.color_mode(), ColorMode::Blend);
    assert_eq!(blink.rest_frame(), Some(1), "rest counts from 1 in files");
    assert_eq!(blink.frames().len(), 3);
    assert_eq!(blink.frames()[1].frame_color().map(CellColor::as_str), Some("mix($accent, $fg, 40%)"));
    assert_eq!(blink.frames()[2].frame_duration(), Some(FrameTime::Motion("step")));
}

#[test]
fn glyphs_fall_back_from_nerd_to_unicode_to_ascii() {
    let (found, _) = parse_animations("blink.toml", GOOD);
    let blink = &found[0].1;
    let seen = |mode| (0..3).map(|index| blink.glyph(index, mode)).collect::<Vec<_>>();
    assert_eq!(seen(GlyphMode::Nerd), ["\u{F111}", "·", "o"]);
    assert_eq!(seen(GlyphMode::Unicode), ["●", "·", "o"]);
    assert_eq!(seen(GlyphMode::Ascii), ["*", ".", "o"]);
}

#[test]
fn every_broken_entry_is_located_and_skipped_while_good_ones_stay() {
    let text = r#"[animations.good]
frames = [{ ascii = "-" }]
[animations.no-frames]
frame = "spinner"
[animations.empty]
frames = []
[animations.no-ascii]
frames = [{ unicode = "●" }]
[animations.wide]
frames = [{ ascii = "-", unicode = "中" }]
[animations.bad-color]
frames = [{ ascii = "-", color = "mix($accent)" }]
[animations.nested-pulse]
frames = [{ ascii = "-", color = "mix(pulse($accent, $muted), $fg, 50%)" }]
[animations.bad-time]
frame = "slide"
frames = [{ ascii = "-" }]
[animations.zero]
frames = [{ ascii = "-", duration = "0ms" }]
[animations.bad-playback]
playback = "twice"
frames = [{ ascii = "-" }]
[animations.bad-colors]
colors = "fade"
frames = [{ ascii = "-" }]
[animations.bad-rest]
rest = 3
frames = [{ ascii = "-" }]
[animations.typo]
frmaes = [{ ascii = "-" }]
[animations.frame-typo]
frames = [{ ascii = "-", colour = "$accent" }]
[animations.bracket]
frames = [{ ascii = "[" }]
[animations.not-ascii]
frames = [{ ascii = "é" }]
[animations.two]
frames = [{ ascii = "ab" }]
[animations.Upper]
frames = [{ ascii = "-" }]
[animations.not-a-table]
frames = ["-"]
[other]
x = 1
"#;
    let (found, diagnostics) = parse_animations("app.toml", text);
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["good"]);
    let found = located(&diagnostics);
    let expect = [
        (3, "animation `no-frames` has no `frames`"),
        (6, "animation `empty` needs at least one frame"),
        (8, "animation `no-ascii` frame 1 is missing its `ascii` glyph"),
        (10, "`中` is 2 cells wide"),
        (12, "expected `,` in colour"),
        (14, "pulse() can only be used as a whole value"),
        (16, "`slide` is not a frame time"),
        (19, "`0ms` is too short"),
        (21, "`twice`; use loop, once or bounce"),
        (24, "`fade`; use step or blend"),
        (27, "rest must be a frame number from 1 to 1"),
        (30, "unknown key `frmaes`"),
        (32, "unknown key `colour`"),
        (34, "`[` is a bracket"),
        (36, "`é` is not printable ASCII"),
        (38, "`ab` is 2 characters"),
        (39, "animation name `Upper`"),
        (42, "frame 1 must be a table"),
        (43, "unknown section `other`"),
    ];
    assert_eq!(found.len(), expect.len(), "{found:#?}");
    for ((line, column, message), (want_line, want)) in found.iter().zip(expect) {
        assert_eq!(*line, want_line, "{message}");
        assert!(*column >= 1, "{message}");
        assert!(message.contains(want), "line {line}: {message} should say {want}");
    }
}

#[test]
fn width_check_accepts_nerd_glyphs_and_rejects_wide_and_combined_ones() {
    assert_eq!(check_glyph("\u{F0A9E}", GlyphMode::Nerd), Ok(()));
    assert_eq!(check_glyph("◜", GlyphMode::Unicode), Ok(()));
    assert_eq!(check_glyph(" ", GlyphMode::Ascii), Ok(()), "a blank frame is a real frame");
    for (glyph, mode) in [("中", GlyphMode::Unicode), ("🚀", GlyphMode::Nerd), ("日", GlyphMode::Nerd)] {
        assert!(check_glyph(glyph, mode).is_err_and(|m| m.contains("2 cells wide")), "{glyph}");
    }
    assert!(check_glyph("👍🏽", GlyphMode::Unicode).is_err(), "an emoji with a modifier is wide");
    assert!(check_glyph("", GlyphMode::Ascii).is_err_and(|m| m.contains("empty")));
    assert!(check_glyph("◜◠", GlyphMode::Unicode).is_err_and(|m| m.contains("2 characters")));
    assert!(check_glyph("✓", GlyphMode::Ascii).is_err_and(|m| m.contains("ASCII")));
    assert!(check_glyph("(", GlyphMode::Unicode).is_err_and(|m| m.contains("bracket")));
}

/// A two-frame loop of 100ms each, the second frame in the accent.
fn two() -> CellAnimation {
    CellAnimation::new()
        .frame_time(FrameTime::Fixed(ms(100)))
        .frame(AnimationFrame::new("a"))
        .frame(AnimationFrame::new("b").color(CellColor::parse("$accent").expect("colour")))
}

#[test]
fn loop_once_and_bounce_order_the_frames() {
    let theme = theme();
    let indices = |animation: &CellAnimation| -> Vec<usize> {
        (0..8).map(|step| animation.sample(&theme, None, ms(step * 100), Some(ms(0))).index).collect()
    };
    let three = two().frame(AnimationFrame::new("c"));
    assert_eq!(indices(&three), [0, 1, 2, 0, 1, 2, 0, 1]);
    assert_eq!(indices(&three.clone().playback(Playback::Bounce)), [0, 1, 2, 1, 0, 1, 2, 1]);
    assert_eq!(indices(&three.clone().playback(Playback::Once)), [0, 1, 2, 2, 2, 2, 2, 2]);
    let once = three.playback(Playback::Once);
    assert!(!once.sample(&theme, None, ms(299), Some(ms(0))).finished);
    assert!(once.sample(&theme, None, ms(300), Some(ms(0))).finished);
    assert_eq!(once.sample(&theme, None, ms(300), Some(ms(0))).next, None, "a finished animation stands still");
    let late = once.sample(&theme, None, ms(1150), Some(ms(1000)));
    assert_eq!(late.index, 1, "time counts from `since`");
}

#[test]
fn frames_change_exactly_on_their_boundary_and_per_frame_durations_count() {
    let theme = theme();
    let animation = two().frame(AnimationFrame::new("c").duration(FrameTime::Fixed(ms(300))));
    let at = |millis| animation.sample(&theme, None, ms(millis), Some(ms(0)));
    assert_eq!((at(99).index, at(100).index, at(199).index, at(200).index), (0, 1, 1, 2));
    assert_eq!((at(499).index, at(500).index), (2, 0), "the long frame lasts 300ms");
    assert_eq!(at(130).next, Some(ms(70)), "the next change is scheduled exactly");
    let stepped = CellAnimation::new().frame_time(FrameTime::Motion("step")).frame(AnimationFrame::new("a"));
    let step = theme.motion().step;
    assert_eq!(stepped.sample(&theme, None, ms(0), Some(ms(0))).next, Some(step), "motion keys follow the theme");
}

#[test]
fn step_colours_hold_per_frame_and_blend_colours_move_towards_the_next_frame() {
    let theme = theme();
    let fg = Rgb::new(0, 0, 0);
    let accent = theme.color("accent").expect("accent");
    let step = two();
    let color = |animation: &CellAnimation, millis| animation.sample(&theme, Some(fg), ms(millis), Some(ms(0))).color;
    assert_eq!(color(&step, 0), Some(fg), "a frame without colour takes the widget's");
    assert_eq!(color(&step, 50), Some(fg));
    assert_eq!(color(&step, 100), Some(accent));
    assert_eq!(color(&step, 150), Some(accent));
    assert!(!step.sample(&theme, Some(fg), ms(50), Some(ms(0))).smooth);

    let blend = two().colors(ColorMode::Blend);
    assert_eq!(color(&blend, 0), Some(fg), "a boundary shows the frame's own colour");
    assert_eq!(color(&blend, 50), Some(fg.mix(accent, 0.5)), "halfway to the next frame");
    assert_eq!(color(&blend, 100), Some(accent));
    assert_eq!(color(&blend, 175), Some(accent.mix(fg, 0.75)), "the loop blends back to the first");
    assert!(blend.sample(&theme, Some(fg), ms(50), Some(ms(0))).smooth);
    let once = two().colors(ColorMode::Blend).playback(Playback::Once);
    assert_eq!(color(&once, 150), Some(accent), "the last frame of a single play has nothing to blend to");
}

#[test]
fn colours_resolve_fg_hex_mix_and_pulse_and_unknown_tokens_take_the_widget_colour() {
    let theme = theme();
    let fg = Rgb::new(200, 100, 0);
    let muted = theme.color("muted").expect("muted");
    let single = |color: &str| {
        CellAnimation::new().frame(AnimationFrame::new("*").color(CellColor::parse(color).expect("parses")))
    };
    let at = |animation: &CellAnimation, millis| animation.sample(&theme, Some(fg), ms(millis), Some(ms(0)));
    assert_eq!(at(&single("#102030"), 0).color, Some(Rgb::new(16, 32, 48)));
    assert_eq!(at(&single("mix($muted, $fg, 25%)"), 0).color, Some(fg.mix(muted, 0.25)));
    assert_eq!(at(&single("$nope"), 0).color, Some(fg));
    let pulse = single("pulse($muted, $fg)");
    let period = theme.motion().pulse_period.as_millis();
    let half = u64::try_from(period / 2).expect("short");
    assert_eq!(at(&pulse, 0).color, Some(muted));
    assert_eq!(at(&pulse, half).color, Some(fg));
    assert!(at(&pulse, 0).smooth, "a pulse needs smooth frames");
    assert_eq!(CellColor::from(Rgb::new(255, 0, 16)).as_str(), "#FF0010");
}

#[test]
fn standing_still_shows_the_rest_frame_and_a_pulse_its_second_colour() {
    let theme = theme();
    let still = |animation: &CellAnimation| animation.sample(&theme, None, ms(900), None);
    let three = two().frame(AnimationFrame::new("c"));
    assert_eq!(still(&three).index, 0, "a loop rests on its first frame");
    assert_eq!(still(&three.clone().playback(Playback::Once)).index, 2, "a single play rests on its last");
    assert_eq!(still(&three.clone().rest(1)).index, 1);
    assert_eq!(still(&three.rest(9)).index, 2, "a rest past the end rests on the last frame");
    let still_frame = still(&two());
    assert!(still_frame.finished && still_frame.next.is_none() && !still_frame.smooth);
    let pulse = CellAnimation::new()
        .frame(AnimationFrame::new("*").color(CellColor::parse("pulse($muted, $accent)").expect("parses")));
    assert_eq!(still(&pulse).color, theme.color("accent"));
}

#[test]
fn frame_times_read_motion_keys_and_durations_and_print_back() {
    assert_eq!(FrameTime::parse("spinner"), Ok(FrameTime::Motion("spinner")));
    assert_eq!(FrameTime::parse(" 80ms "), Ok(FrameTime::Fixed(ms(80))));
    assert_eq!(FrameTime::parse("0.25s"), Ok(FrameTime::Fixed(ms(250))));
    assert!(FrameTime::parse("slide").is_err());
    assert!(FrameTime::parse("fast").is_err_and(|m| m.contains("motion key")));
    for text in ["spinner", "80ms", "250ms"] {
        assert_eq!(FrameTime::parse(text).map(|time| time.to_string()).as_deref(), Ok(text));
    }
}

#[test]
fn toml_written_back_reads_as_the_same_animation() {
    let (found, _) = parse_animations("blink.toml", GOOD);
    let blink = &found[0].1;
    let written = blink.to_toml("blink");
    assert!(written.contains(r#"{ nerd = "\uF111", unicode = "●", ascii = "*" }"#), "{written}");
    assert!(written.contains("rest = 2"), "{written}");
    let (again, diagnostics) = parse_animations("again.toml", &written);
    assert!(diagnostics.is_empty(), "{diagnostics:?}\n{written}");
    assert_eq!(again, vec![("blink".to_owned(), blink.clone())]);

    let quoted = CellAnimation::new().frame(AnimationFrame::new("\\").unicode("\""));
    let (again, _) = parse_animations("q.toml", &quoted.to_toml("q"));
    assert_eq!(again[0].1, quoted);
}

#[test]
fn every_built_in_animation_round_trips_through_toml() {
    let icons = IconSetRegistry::builtin().icons("default", &BTreeMap::new(), GlyphMode::Unicode);
    let names: Vec<&str> = icons.animation_names().collect();
    assert_eq!(
        names,
        [
            "spinner-arc",
            "spinner-done",
            "spinner-dots",
            "spinner-orbit",
            "spinner-pop",
            "spinner-pulse",
            "spinner-quarters",
            "spinner-slices"
        ]
    );
    for name in names {
        let animation = icons.animation(name).expect("listed");
        let (again, diagnostics) = parse_animations("built-in.toml", &animation.to_toml(name));
        assert!(diagnostics.is_empty(), "{name}: {diagnostics:?}");
        assert_eq!(again[0].1, **animation, "{name}");
    }
}

#[test]
fn a_theme_animation_replaces_a_built_in_and_an_icon_set_can_add_one() {
    let theme = "[meta]\nname = \"Soft\"\nextends = \"monochrome\"\n\n[animations.spinner-arc]\nframe = \"step\"\n\
                 frames = [{ unicode = \"◐\", ascii = \"o\" }, { unicode = \"◑\", ascii = \"O\" }]\n";
    let mut themes = ThemeRegistry::builtin();
    themes.add_source("soft", "soft.toml", theme);
    let resolved = themes.resolve("soft");
    assert!(resolved.diagnostics.is_empty(), "{:?}", resolved.diagnostics);
    let soft = resolved.theme.expect("resolves");
    let mut sets = IconSetRegistry::builtin();
    sets.add_source(
        "extra",
        "extra.toml",
        "[meta]\nname = \"Extra\"\n[icons]\n[animations.beacon]\nframes = [{ ascii = \"!\" }]\n",
    );
    let icons = sets.icons_with_animations("extra", &BTreeMap::new(), soft.animation_overrides(), GlyphMode::Unicode);
    let arc = icons.animation("spinner-arc").expect("replaced, not removed");
    assert_eq!(arc.frames().len(), 2);
    assert_eq!(arc.glyph(1, GlyphMode::Unicode), "◑");
    assert!(icons.animation("beacon").is_some(), "the set's own animation");
    assert!(icons.animation("spinner-done").is_some(), "a set without built-ins still has them");
}

#[test]
fn a_former_spinner_icon_replaces_the_glyphs_and_keeps_colours_at_the_same_place() {
    let mut sets = IconSetRegistry::builtin();
    let set = "[meta]\nname = \"Old\"\n[icons]\nspinner-done = { nerd = \"○◐●\", unicode = \"○◐●\", ascii = \"oO@\" }\n\
               spinner = { nerd = \"ab\", unicode = \"ab\", ascii = \"abc\" }\nspinner-arc = { nerd = \"中\", unicode = \"-\", ascii = \"-\" }\n";
    sets.add_source("old", "old.toml", set);
    let messages: Vec<String> = sets.diagnostics().iter().map(ToString::to_string).collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].starts_with("old.toml:6:") && messages[0].contains("2 cells wide"), "{messages:?}");
    let icons = sets.icons("old", &BTreeMap::new(), GlyphMode::Ascii);
    assert!(!icons.contains("spinner-done"), "former spinner keys are not icons any more");
    let done = icons.animation("spinner-done").expect("built in");
    assert_eq!(done.play_mode(), Playback::Once, "timing and playback stay");
    let colors: Vec<Option<&str>> =
        done.frames().iter().map(|frame| frame.frame_color().map(CellColor::as_str)).collect();
    assert_eq!(colors, [None, Some("mix($success, $fg, 50%)"), Some("$success")]);
    let dots = icons.animation("spinner-dots").expect("the old `spinner` key");
    let glyphs: String = (0..6).map(|index| dots.glyph(index, GlyphMode::Ascii)).collect();
    assert_eq!(glyphs, "abcabc", "six frames play two Unicode and three ASCII glyphs in step");
    assert_eq!(icons.animation("spinner-arc").map(|arc| arc.frames().len()), Some(12), "the broken icon is skipped");
}
