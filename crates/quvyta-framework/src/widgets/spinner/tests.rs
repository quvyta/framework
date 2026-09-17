//! Spinner tests: styles, frames, tones, the finish and reduced motion.

use std::time::Duration;

use super::*;
use crate::runtime::{App, Command, Harness};
use crate::widget::View;

struct Demo(SpinnerStyle);

impl App for Demo {
    type Msg = ();
    fn update(&mut self, _: ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Spinner::new().style(self.0).label("Working")).fill_width();
    }
}

#[test]
fn frames_advance_with_time() {
    let mut h = Harness::new(Demo(SpinnerStyle::Dots), 20, 1);
    assert_eq!(h.screen(), "⠋ Working\n");
    h.advance(Duration::from_millis(80));
    assert_eq!(h.screen(), "⠙ Working\n");
}

#[test]
fn styles_are_alphabetical_and_arc_is_the_default() {
    let names: Vec<&str> = SpinnerStyle::ALL.iter().map(|style| style.name()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    assert_eq!(SpinnerStyle::ALL[0], SpinnerStyle::default());
    assert_eq!(Spinner::new(), Spinner::new().style(SpinnerStyle::Arc));
    let h = Harness::new(Demo(SpinnerStyle::default()), 20, 1);
    assert_eq!(h.screen(), "◜ Working\n");
}

#[test]
fn every_frame_of_every_style_is_one_cell_in_every_glyph_mode() {
    use crate::icons::GlyphMode;
    let mut h = Harness::new(Demo(SpinnerStyle::default()), 20, 1);
    for mode in [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii] {
        h.set_glyph_mode(mode);
        let icons = h.env().icons();
        let keys = SpinnerStyle::ALL.map(SpinnerStyle::animation);
        for key in keys.iter().copied().chain([DONE_ANIMATION]) {
            let animation = icons.animation(key).unwrap_or_else(|| panic!("{key} is built in"));
            assert!(!animation.frames().is_empty(), "{key} has frames in {mode:?}");
            for frame in animation.frames() {
                let glyph = frame.glyph(mode);
                assert_eq!(crate::text::width(glyph), 1, "{key} frame {glyph:?} in {mode:?}");
            }
        }
        let finish = icons.animation(DONE_ANIMATION).map(|animation| animation.frames().len());
        assert_eq!(finish, Some(5), "the finish has five frames in {mode:?}");
    }
}

#[test]
fn quarters_turn_clockwise_with_quadrant_blocks() {
    let mut h = Harness::new(Demo(SpinnerStyle::Quarters), 20, 1);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    let mut seen = String::new();
    for _ in 0..4 {
        seen.push(h.screen().chars().next().unwrap_or(' '));
        h.advance(Duration::from_millis(80));
    }
    assert_eq!(seen, "▖▘▝▗");
}

#[test]
fn slices_fill_a_pie_and_the_arc_turns_the_nerd_progress_spinner() {
    use crate::icons::GlyphMode;
    let seen = |style: SpinnerStyle, mode: GlyphMode, count: usize| {
        let mut h = Harness::new(Demo(style), 20, 1);
        h.set_glyph_mode(mode);
        let mut seen = String::new();
        for _ in 0..count {
            seen.push(h.screen().chars().next().unwrap_or(' '));
            h.advance(Duration::from_millis(80));
        }
        seen
    };
    let nerd_slices: String = (0xF0A9E..=0xF0AA5).filter_map(char::from_u32).collect();
    assert_eq!(seen(SpinnerStyle::Slices, GlyphMode::Nerd, 9), format!("{nerd_slices}\u{F0A9E}"));
    assert_eq!(seen(SpinnerStyle::Slices, GlyphMode::Unicode, 6), "◐◓◑◒◐◓");
    assert_eq!(seen(SpinnerStyle::Slices, GlyphMode::Ascii, 5), ".oO@.");
    let nerd_arc: String = (0xEE06..=0xEE0B).filter_map(char::from_u32).collect();
    assert_eq!(seen(SpinnerStyle::Arc, GlyphMode::Nerd, 6), nerd_arc);
    assert_eq!(seen(SpinnerStyle::Arc, GlyphMode::Unicode, 6), "◜◠◝◞◡◟");
    assert_eq!(seen(SpinnerStyle::Arc, GlyphMode::Ascii, 4), "-\\|/");
}

#[test]
fn reduced_motion_stands_still_and_ascii_has_frames() {
    let mut h = Harness::new(Demo(SpinnerStyle::Arc), 20, 1);
    h.set_reduced_motion(true);
    let still = h.screen();
    h.advance(Duration::from_millis(400));
    assert_eq!(h.screen(), still);
    h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
    assert!(h.screen().is_ascii());
}

/// A spinner the test finishes and restarts by sending `true` or `false`.
struct Finishing {
    style: SpinnerStyle,
    done: bool,
}

impl App for Finishing {
    type Msg = bool;
    fn update(&mut self, done: bool) -> Command<bool> {
        self.done = done;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, bool>) {
        ui.add(Spinner::new().style(self.style).label("Working").done(self.done)).fill_width();
    }
}

fn finishing(style: SpinnerStyle, done: bool) -> Harness<Finishing> {
    Harness::new(Finishing { style, done }, 20, 1)
}

fn tone(h: &Harness<Finishing>, token: &str) -> Rgb {
    h.env().theme().color(token).expect("token")
}

#[test]
fn done_plays_the_tick_once_blending_into_success_in_every_glyph_mode() {
    use crate::icons::GlyphMode;
    let modes = [
        (GlyphMode::Nerd, ["\u{F0995}", "\u{F05D}", "\u{F49E}", "\u{F05E1}", "\u{F0133}"]),
        (GlyphMode::Unicode, ["·", "∙", "✓", "✔", "✔"]),
        (GlyphMode::Ascii, [".", ".", "v", "v", "v"]),
    ];
    for (mode, glyphs) in modes {
        let mut h = finishing(SpinnerStyle::Arc, false);
        h.set_glyph_mode(mode);
        let step = h.env().theme().motion().step;
        let (accent, success) = (tone(&h, "accent"), tone(&h, "success"));
        h.advance(Duration::from_millis(130));
        h.send(true);
        let colours = [0.0, 0.25, 0.5, 0.75, 1.0].map(|t| accent.mix(success, t));
        assert_eq!(colours[4], success);
        for (frame, (glyph, colour)) in glyphs.iter().zip(colours).enumerate() {
            assert_eq!(h.screen(), format!("{glyph} Working\n"), "frame {frame} in {mode:?}");
            assert_eq!(h.fg(0, 0), Some(colour), "frame {frame} colour in {mode:?}");
            assert_eq!(h.fg(2, 0), Some(tone(&h, "dim")), "the label keeps its colour");
            h.advance(step);
        }
        h.advance(step * 10);
        assert_eq!(h.screen(), format!("{} Working\n", glyphs[4]), "rests on the tick in {mode:?}");
        assert_eq!(h.fg(0, 0), Some(success));
    }
}

#[test]
fn the_frame_changes_exactly_on_each_step() {
    let mut h = finishing(SpinnerStyle::Arc, false);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    let step = h.env().theme().motion().step;
    h.send(true);
    h.advance(step - Duration::from_millis(1));
    assert_eq!(h.screen(), "· Working\n");
    h.advance(Duration::from_millis(1));
    assert_eq!(h.screen(), "∙ Working\n");
}

#[test]
fn undone_spins_again_and_a_second_finish_replays() {
    let mut h = finishing(SpinnerStyle::Dots, false);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    let step = h.env().theme().motion().step;
    h.send(true);
    h.advance(step * 4);
    assert_eq!(h.screen(), "✔ Working\n");
    h.send(false);
    let turning = h.screen();
    assert!("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".contains(turning.chars().next().unwrap_or(' ')), "{turning}");
    assert_eq!(h.fg(0, 0), Some(tone(&h, "accent")));
    h.advance(Duration::from_millis(80));
    assert_ne!(h.screen(), turning, "it turns again");
    h.send(true);
    assert_eq!(h.screen(), "· Working\n", "the finish plays from the start");
}

#[test]
fn a_spinner_drawn_done_or_with_reduced_motion_rests_on_the_tick() {
    let mut h = finishing(SpinnerStyle::Arc, true);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    assert_eq!(h.screen(), "✔ Working\n", "already done when first drawn");
    assert_eq!(h.fg(0, 0), Some(tone(&h, "success")));

    let mut h = finishing(SpinnerStyle::Arc, false);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    h.set_reduced_motion(true);
    h.send(true);
    assert_eq!(h.screen(), "✔ Working\n");
    assert_eq!(h.fg(0, 0), Some(tone(&h, "success")));
}

#[test]
fn the_tick_takes_each_themes_success_colour_and_starts_from_the_tone() {
    for theme in ["monochrome", "iris", "nordic", "amber"] {
        let mut h = Harness::new(Finishing { style: SpinnerStyle::Pop, done: false }, 20, 1);
        h.set_theme(theme);
        let step = h.env().theme().motion().step;
        h.send(true);
        assert_eq!(h.fg(0, 0), Some(tone(&h, "accent")), "{theme} starts from the spinner colour");
        h.advance(step * 4);
        assert_eq!(h.fg(0, 0), Some(tone(&h, "success")), "{theme} ends on success");
    }
}

#[test]
fn a_pulse_finishes_from_the_colour_on_screen() {
    let mut h = finishing(SpinnerStyle::Pulse, false);
    h.advance(Duration::from_millis(300));
    let on_screen = h.fg(0, 0).expect("pulse colour");
    assert_ne!(Some(on_screen), h.env().theme().color("accent"), "caught mid-breath");
    h.send(true);
    assert_eq!(h.fg(0, 0), Some(on_screen));
    h.advance(h.env().theme().motion().step);
    assert_eq!(h.fg(0, 0), Some(on_screen.mix(tone(&h, "success"), 0.25)));
}

#[test]
fn a_theme_can_replace_the_finish_frames() {
    let dir = std::env::temp_dir().join(format!("quvyta-spinner-done-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let theme = "[meta]\nname = \"Round ticks\"\nextends = \"monochrome\"\n\n[icons]\n\
                 spinner-done = { nerd = \"○●\", unicode = \"○●\", ascii = \"o*\" }\n";
    std::fs::write(dir.join("round-ticks.toml"), theme).expect("theme file");
    let dirs = crate::env::AssetDirs { themes: Some(dir.clone()), ..Default::default() };
    let env = crate::env::Env::load(&dirs).expect("loads");
    std::fs::remove_dir_all(&dir).ok();
    assert!(env.diagnostics().is_empty(), "{:?}", env.diagnostics());
    let mut h = Harness::with_env(Finishing { style: SpinnerStyle::Arc, done: false }, env, 20, 1);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    h.set_theme("round-ticks");
    let step = h.env().theme().motion().step;
    h.send(true);
    assert_eq!(h.screen(), "○ Working\n");
    assert_eq!(h.fg(0, 0), Some(tone(&h, "accent")));
    h.advance(step);
    assert_eq!(h.screen(), "● Working\n", "two frames finish in one step");
    assert_eq!(h.fg(0, 0), Some(tone(&h, "success")));
    h.set_glyph_mode(crate::icons::GlyphMode::Ascii);
    assert_eq!(h.screen(), "* Working\n");
}

#[test]
fn pulse_breathes_between_faint_and_accent() {
    let mut h = Harness::new(Demo(SpinnerStyle::Pulse), 20, 1);
    let start = h.fg(0, 0);
    h.advance(Duration::from_millis(700));
    assert_ne!(h.fg(0, 0), start);
}

/// A spinner playing an animation by name.
struct Named(&'static str);

impl App for Named {
    type Msg = ();
    fn update(&mut self, _: ()) -> Command<()> {
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, ()>) {
        ui.add(Spinner::new().animation(self.0).label("Working")).fill_width();
    }
}

#[test]
fn a_spinner_plays_any_animation_by_name_and_shows_a_missing_one() {
    let mut named = Harness::new(Named("spinner-pop"), 20, 1);
    let mut styled = Harness::new(Demo(SpinnerStyle::Pop), 20, 1);
    for _ in 0..5 {
        assert_eq!(named.screen(), styled.screen());
        named.advance(Duration::from_millis(80));
        styled.advance(Duration::from_millis(80));
    }
    assert_eq!(Spinner::new().animation("spinner-slices"), Spinner::new().style(SpinnerStyle::Slices));
    let missing = Harness::new(Named("nope"), 20, 1);
    assert_eq!(missing.screen(), "⟦ Working\n", "noticed like a missing icon, in one cell");
}

/// A theme file in a temporary directory, loaded into an environment.
fn env_with_theme(id: &str, text: &str) -> crate::env::Env {
    let dir = std::env::temp_dir().join(format!("quvyta-{id}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join(format!("{id}.toml")), text).expect("theme file");
    let dirs = crate::env::AssetDirs { themes: Some(dir.clone()), ..Default::default() };
    let env = crate::env::Env::load(&dirs).expect("loads");
    std::fs::remove_dir_all(&dir).ok();
    assert!(env.diagnostics().is_empty(), "{:?}", env.diagnostics());
    env
}

#[test]
fn a_theme_animation_replaces_the_finish_with_its_own_colours_and_timing() {
    let theme = "[meta]\nname = \"Flash\"\nextends = \"monochrome\"\n\n[animations.spinner-done]\n\
                 frame = \"100ms\"\nplayback = \"once\"\nframes = [\n\
                 { unicode = \"!\", ascii = \"!\", color = \"$warning\" },\n\
                 { unicode = \"✓\", ascii = \"v\", color = \"#123456\" },\n]\n";
    let env = env_with_theme("flash", theme);
    let mut h = Harness::with_env(Finishing { style: SpinnerStyle::Arc, done: false }, env, 20, 1);
    h.set_glyph_mode(crate::icons::GlyphMode::Unicode);
    h.set_theme("flash");
    h.send(true);
    assert_eq!(h.screen(), "! Working\n");
    assert_eq!(h.fg(0, 0), Some(tone(&h, "warning")));
    h.advance(Duration::from_millis(99));
    assert_eq!(h.screen(), "! Working\n");
    h.advance(Duration::from_millis(1));
    assert_eq!(h.screen(), "✓ Working\n");
    assert_eq!(h.fg(0, 0), Some(Rgb::new(0x12, 0x34, 0x56)), "a hard-coded colour");
    h.advance(Duration::from_secs(5));
    assert_eq!(h.screen(), "✓ Working\n", "rests on the last frame");
}
