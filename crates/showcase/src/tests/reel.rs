//! The showcase recording for the README and launch posts: a scripted visit through the showcase
//! drawn frame by frame from the test harness, with a fake clock, so the same frames come out on
//! every machine and no terminal or screen recorder is involved.
//!
//! `docs/screenshots/make-showcase-gif.sh` runs it and encodes the frames; run by hand with
//! `QUVYTA_REEL_DIR=<folder> cargo test --release -p quvyta-framework-showcase reel -- --ignored`.
//! The folder receives numbered PNG frames and `frames.txt`, a list for ffmpeg's concat reader
//! that gives every frame its duration.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use qframe::icons::GlyphMode;
use qframe::runtime::Harness;

use super::showcase_tall;
use crate::app::{Msg, Showcase};
use crate::pages::{PageMsg, example_dashboard};

/// Time between two frames: smooth enough for the cell-step animations, which move no faster.
const STEP: Duration = Duration::from_millis(50);

/// Terminal size of the recording.
const SIZE: (u16, u16) = (120, 36);

/// How long the pointer takes to cross one cell when it glides: slow enough to follow.
const GLIDE_PER_CELL: Duration = Duration::from_millis(12);

/// Pause between typed characters, as a calm typist.
const TYPE_GAP: Duration = Duration::from_millis(180);

/// A harness, a pointer on it and the frames drawn so far.
struct Reel {
    harness: Harness<Showcase>,
    pointer: (i32, i32),
    dir: PathBuf,
    /// Each distinct picture written, with how long it stays.
    frames: Vec<(String, Duration)>,
    last_svg: String,
}

impl Reel {
    fn new(dir: PathBuf) -> Self {
        let mut harness = showcase_tall(Showcase::new(), "example-dashboard", SIZE.1);
        harness.resize(SIZE.0, SIZE.1).set_theme("nordic").set_glyph_mode(GlyphMode::Nerd);
        harness.advance(Duration::from_secs(2));
        // The pointer starts at rest over the dashboard's empty right side, out of the way.
        let pointer = (104, 22);
        harness.hover(pointer.0, pointer.1);
        Self { harness, pointer, dir, frames: Vec::new(), last_svg: String::new() }
    }

    /// Draws the current screen for `duration`. A picture equal to the last one lengthens it,
    /// so pauses cost no frames.
    fn capture(&mut self, duration: Duration) {
        let (x, y) = self.pointer;
        let shot = qshots::Shot::of(&self.harness)
            .title("qframe")
            .square()
            .pointer(u16::try_from(x).unwrap_or(u16::MAX), u16::try_from(y).unwrap_or(u16::MAX));
        assert!(shot.missing().is_empty(), "the font lacks {:?}", shot.missing());
        let svg = shot.to_svg();
        if svg == self.last_svg
            && let Some((_, last)) = self.frames.last_mut()
        {
            *last += duration;
            return;
        }
        let name = format!("{:04}.png", self.frames.len());
        let png = shot.to_png().expect("the frame rasterises");
        std::fs::write(self.dir.join(&name), png).expect("the frame folder is writable");
        self.frames.push((name, duration));
        self.last_svg = svg;
    }

    /// Lets `duration` pass, one frame per step, so animations play.
    fn hold(&mut self, duration: Duration) {
        let mut left = duration;
        while !left.is_zero() {
            let step = left.min(STEP);
            self.harness.advance(step);
            self.capture(step);
            left -= step;
        }
    }

    /// Lets `duration` pass on the dashboard while its clock ticks and its chart moves on every
    /// `every`.
    fn live(&mut self, duration: Duration, every: Duration) {
        let mut left = duration;
        while !left.is_zero() {
            let slice = left.min(every);
            self.hold(slice);
            left -= slice;
            self.harness.send(Msg::Page(PageMsg::ExampleDashboard(example_dashboard::Msg::Refresh)));
        }
    }

    /// Moves the pointer to `to` in a straight line, easing out as it arrives, telling the
    /// showcase of every cell it enters.
    fn glide(&mut self, to: (i32, i32)) {
        let from = self.pointer;
        let cells = (to.0 - from.0).abs().max((to.1 - from.1).abs()).max(1);
        let total = (GLIDE_PER_CELL * u32::try_from(cells).unwrap_or(1)).clamp(STEP * 4, Duration::from_millis(900));
        let steps = u32::try_from(total.as_millis() / STEP.as_millis()).unwrap_or(1).max(1);
        for step in 1..=steps {
            let t = f64::from(step) / f64::from(steps);
            let eased = 1.0 - (1.0 - t).powi(3);
            let at = |a: i32, b: i32| a + (f64::from(b - a) * eased).round() as i32;
            let next = (at(from.0, to.0), at(from.1, to.1));
            if next != self.pointer {
                self.pointer = next;
                self.harness.hover(next.0, next.1);
            }
            self.harness.advance(STEP);
            self.capture(STEP);
        }
    }

    /// Glides to the first cell of `text` at or below `row` and clicks it.
    fn click_on(&mut self, text: &str, row: i32) {
        let target = self.find_below(text, row);
        // Resting a cell inside the word reads better than touching its first letter.
        self.glide((target.0 + 1, target.1));
        self.hold(Duration::from_millis(100));
        self.harness.click(self.pointer.0, self.pointer.1);
    }

    fn find_below(&self, text: &str, row: i32) -> (i32, i32) {
        let screen = self.harness.screen();
        let skip = usize::try_from(row).unwrap_or(0);
        let found = screen.lines().enumerate().skip(skip).find_map(|(y, line)| {
            let column = line[..line.find(text)?].chars().count();
            Some((i32::try_from(column).ok()?, i32::try_from(y).ok()?))
        });
        found.unwrap_or_else(|| panic!("`{text}` is not on screen below row {row}:\n{screen}"))
    }

    /// Opens the theme list from the top bar, where `current` names the theme in use, and picks
    /// `theme`; it then stays for `stay`.
    fn pick_theme(&mut self, current: &str, theme: &str, stay: Duration) {
        self.click_on(current, 0);
        self.hold(Duration::from_millis(350));
        self.click_on(theme, 1);
        self.hold(stay);
    }

    fn type_slowly(&mut self, text: &str) {
        for c in text.chars() {
            self.harness.type_text(&c.to_string());
            self.hold(TYPE_GAP);
        }
    }

    /// The list for ffmpeg's concat reader. Its last entry is named twice, as the reader wants,
    /// or the final pause would be cut.
    fn list(&self) -> String {
        let mut list = String::new();
        for (name, duration) in &self.frames {
            let _ = writeln!(list, "file '{name}'\nduration {:.3}", duration.as_secs_f64());
        }
        if let Some((name, _)) = self.frames.last() {
            let _ = writeln!(list, "file '{name}'");
        }
        list
    }

    fn total(&self) -> Duration {
        self.frames.iter().map(|(_, duration)| *duration).sum()
    }
}

#[test]
#[ignore = "writes a folder of frames; run through docs/screenshots/make-showcase-gif.sh"]
fn reel() {
    let dir = PathBuf::from(std::env::var("QUVYTA_REEL_DIR").expect("QUVYTA_REEL_DIR names the frame folder"));
    std::fs::create_dir_all(&dir).expect("the frame folder can be made");
    let mut reel = Reel::new(dir);
    let second = Duration::from_secs(1);

    // The dashboard, alive: the clock and the CPU chart move on.
    reel.live(Duration::from_millis(3000), Duration::from_millis(750));

    // Themes from the top bar, a second each.
    reel.pick_theme("Nordic", "Amber", second);
    reel.pick_theme("Amber", "Iris", second);
    reel.pick_theme("Iris", "Monochrome", second);

    // The command palette narrows as it is typed into, then opens a page.
    // The pointer steps aside to a quiet corner the palette leaves clear.
    reel.glide((113, 19));
    reel.harness.press("ctrl+p");
    reel.hold(Duration::from_millis(600));
    reel.type_slowly("tab");
    reel.hold(Duration::from_millis(1400));
    reel.harness.press("enter");
    reel.hold(Duration::from_millis(1500));

    // The page's code, and back to its demo.
    reel.click_on("Code", 3);
    reel.hold(Duration::from_millis(2200));
    reel.click_on("Demo", 3);
    reel.hold(Duration::from_millis(800));

    // The pointer runs down the menu: the highlight follows it and slides in.
    let first = reel.find_below("Table", 4);
    reel.glide((first.0 + 2, first.1));
    reel.hold(Duration::from_millis(450));
    for row in 1..5 {
        reel.glide((first.0 + 2, first.1 + row));
        reel.hold(Duration::from_millis(450));
    }

    // Back where it started, so the loop joins: Nordic, then the dashboard again.
    reel.pick_theme("Monochrome", "Nordic", Duration::from_millis(300));
    reel.glide((104, 22));
    reel.harness.press("esc");
    reel.hold(Duration::from_millis(1200));

    std::fs::write(reel.dir.join("frames.txt"), reel.list()).expect("the frame folder is writable");
    let total = reel.total();
    assert!((Duration::from_secs(20)..=Duration::from_secs(25)).contains(&total), "the loop lasts {total:?}");
    println!("{} frames, {total:?}", reel.frames.len());
}
