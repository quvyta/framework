//! The showcase recording for the README and launch posts: a scripted visit through the showcase
//! drawn frame by frame from the test harness with [`qshots::Reel`], on a fake clock, so the
//! same frames come out on every machine and no terminal or screen recorder is involved.
//!
//! `docs/screenshots/make-showcase-gif.sh` runs it; it draws the frames into
//! `target/showcase-reel/` and has ffmpeg encode them as `docs/screenshots/showcase.gif` and
//! `showcase.mp4`.

use std::path::Path;
use std::time::Duration;

use qframe::icons::GlyphMode;
use qshots::Reel;

use super::showcase_tall;
use crate::app::{Msg, Showcase};
use crate::pages::{PageMsg, example_dashboard};

/// Terminal size of the recording.
const SIZE: (u16, u16) = (120, 36);

/// Width of the GIF and the MP4: half the frames' drawn size, which keeps the GIF small while
/// every cell stays readable.
const WIDTH: u32 = 1112;

/// Pause between typed characters, as a calm typist.
const TYPE_GAP: Duration = Duration::from_millis(180);

/// Lets `duration` pass on the dashboard while its clock ticks and its chart moves on every
/// `every`.
fn live(reel: &mut Reel<Showcase>, duration: Duration, every: Duration) {
    let mut left = duration;
    while !left.is_zero() {
        let slice = left.min(every);
        reel.hold(slice);
        left -= slice;
        reel.harness_mut().send(Msg::Page(PageMsg::ExampleDashboard(example_dashboard::Msg::Refresh)));
    }
}

/// Opens the theme list from the top bar, where `current` names the theme in use, and picks
/// `theme`; it then stays for `stay`.
fn pick_theme(reel: &mut Reel<Showcase>, current: &str, theme: &str, stay: Duration) {
    reel.click_on(current).hold(Duration::from_millis(350));
    reel.click_on_below(theme, 1).hold(stay);
}

#[test]
#[ignore = "draws the frames and encodes them with ffmpeg; run through docs/screenshots/make-showcase-gif.sh"]
fn reel() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut harness = showcase_tall(Showcase::new(), "example-dashboard", SIZE.1);
    harness.resize(SIZE.0, SIZE.1).set_theme("nordic").set_glyph_mode(GlyphMode::Nerd);
    harness.advance(Duration::from_secs(2));
    // The pointer starts at rest over the dashboard's empty right side, out of the way.
    let mut reel = Reel::new(harness, root.join("target/showcase-reel")).title("qframe").pointer_at(104, 22);
    let second = Duration::from_secs(1);

    // The dashboard, alive: the clock and the CPU chart move on.
    live(&mut reel, Duration::from_millis(3000), Duration::from_millis(750));

    // Themes from the top bar, a second each.
    pick_theme(&mut reel, "Nordic", "Amber", second);
    pick_theme(&mut reel, "Amber", "Iris", second);
    pick_theme(&mut reel, "Iris", "Monochrome", second);

    // The command palette narrows as it is typed into, then opens a page.
    // The pointer steps aside to a quiet corner the palette leaves clear.
    reel.glide((113, 19));
    reel.press("ctrl+p", Duration::from_millis(600));
    reel.type_slowly("tab", TYPE_GAP).hold(Duration::from_millis(1400));
    reel.press("enter", Duration::from_millis(1500));

    // The page's code, and back to its demo.
    reel.click_on_below("Code", 3).hold(Duration::from_millis(2200));
    reel.click_on_below("Demo", 3).hold(Duration::from_millis(800));

    // The pointer runs down the menu: the highlight follows it and slides in.
    let first = reel.find_below("Table", 4).expect("the menu lists Table");
    for row in 0..5 {
        reel.glide((first.0 + 2, first.1 + row)).hold(Duration::from_millis(450));
    }

    // Back where it started, so the loop joins: Nordic, then the dashboard again.
    pick_theme(&mut reel, "Monochrome", "Nordic", Duration::from_millis(300));
    reel.glide((104, 22));
    reel.press("esc", Duration::from_millis(1200));

    let recording = reel.finish().expect("the frame list is written");
    let total = recording.total();
    assert!((Duration::from_secs(20)..=Duration::from_secs(25)).contains(&total), "the loop lasts {total:?}");
    let shots = root.join("docs/screenshots");
    recording
        .encode(shots.join("showcase.gif"), shots.join("showcase.mp4"), WIDTH)
        .expect("ffmpeg encodes the recording");
    println!("{} frames, {total:?}", recording.frames());
}
