//! Migration guard: every spinner style, tone, theme and glyph mode draws exactly the frames and
//! colours it drew before spinners became cell animations. The fixture was recorded from the
//! icon-based spinner (commit d1f72e3) and must never be regenerated to make this test pass.
//! Deliberate design changes are edited into it by hand: the Ring style was removed, Arc uses the
//! Nerd Font progress spinner frames, and the Unicode Slices frames are the half circles ◐◓◑◒.

use std::fmt::Write as _;
use std::time::Duration;

use qframe::color::Rgb;
use qframe::icons::GlyphMode;
use qframe::runtime::{App, Command, Harness};
use qframe::widget::View;
use qframe::widgets::{Spinner, SpinnerStyle};

const THEMES: [&str; 4] = ["monochrome", "iris", "nordic", "amber"];
const MODES: [GlyphMode; 3] = [GlyphMode::Nerd, GlyphMode::Unicode, GlyphMode::Ascii];

/// One spinner the recorder finishes by sending `true`.
struct One {
    style: SpinnerStyle,
    variant: Option<&'static str>,
    done: bool,
}

impl App for One {
    type Msg = bool;
    fn update(&mut self, done: bool) -> Command<bool> {
        self.done = done;
        Command::none()
    }
    fn view(&self, ui: &mut View<'_, bool>) {
        let mut spinner = Spinner::new().style(self.style).done(self.done);
        if let Some(variant) = self.variant {
            spinner = spinner.variant(variant);
        }
        ui.add(spinner);
    }
}

fn hex(color: Option<Rgb>) -> String {
    color.map_or_else(|| "none".to_owned(), |c| format!("{:02x}{:02x}{:02x}", c.r, c.g, c.b))
}

/// FNV-1a, so long colour sequences stay one line in the fixture.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

/// The glyph and colour of the spinner cell every 20ms for `samples` samples.
fn sample(h: &mut Harness<One>, samples: usize) -> (String, String) {
    let (mut glyphs, mut colors) = (String::new(), String::new());
    for _ in 0..samples {
        glyphs.push(h.screen().chars().next().unwrap_or(' '));
        colors.push_str(&hex(h.fg(0, 0)));
        colors.push(' ');
        h.advance(Duration::from_millis(20));
    }
    (glyphs, colors)
}

/// One fixture line: the glyphs, the first three colours and a hash of all of them.
fn line(out: &mut String, case: &str, (glyphs, colors): &(String, String)) {
    let first: Vec<&str> = colors.split(' ').take(3).collect();
    let _ = writeln!(out, "{case}: {glyphs} | {} | {:016x}", first.join(" "), fnv(colors));
}

/// A harness drawing one spinner in `theme` and `mode`.
fn harness(one: One, theme: &str, mode: GlyphMode) -> Harness<One> {
    let mut h = Harness::new(one, 4, 1);
    h.set_theme(theme).set_glyph_mode(mode);
    h
}

/// Records every case in the fixture's format.
fn record() -> String {
    let mut out = String::new();
    for theme in THEMES {
        for mode in MODES {
            for style in SpinnerStyle::ALL {
                for variant in [None, Some("success")] {
                    let case = format!("{theme} {mode:?} {} {}", style.name(), variant.unwrap_or("-"));
                    // 3200ms is a whole cycle of every style (40 frames of 80ms) and two pulses.
                    let mut h = harness(One { style, variant, done: false }, theme, mode);
                    line(&mut out, &format!("{case} turning"), &sample(&mut h, 161));

                    let mut h = harness(One { style, variant, done: false }, theme, mode);
                    h.set_reduced_motion(true);
                    line(&mut out, &format!("{case} reduced"), &sample(&mut h, 3));
                    h.send(true);
                    line(&mut out, &format!("{case} reduced-done"), &sample(&mut h, 3));

                    let mut h = harness(One { style, variant, done: false }, theme, mode);
                    h.advance(Duration::from_millis(310));
                    h.send(true);
                    line(&mut out, &format!("{case} done"), &sample(&mut h, 20));

                    let mut h = harness(One { style, variant, done: true }, theme, mode);
                    line(&mut out, &format!("{case} drawn-done"), &sample(&mut h, 3));
                }
            }
        }
    }
    out
}

#[test]
fn every_spinner_draws_the_frames_and_colours_it_drew_before_animations() {
    let recorded = record();
    let pinned = include_str!("fixtures/spinner-sequences.txt");
    for (now, before) in recorded.lines().zip(pinned.lines()) {
        assert_eq!(now, before);
    }
    assert_eq!(recorded.lines().count(), pinned.lines().count());
}
