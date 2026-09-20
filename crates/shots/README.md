# quvyta-framework-shots

Screenshots of [Quvyta framework](https://github.com/quvyta/framework) test screens, as SVG and
PNG, for READMEs. Build a scene with the framework's `Harness`, then:

```rust,ignore
qshots::Shot::of(&harness).title("qcode").save("docs/screenshots/home")?;
```

writes `docs/screenshots/home.svg` and `docs/screenshots/home.png` (twice the size). Nothing is
read from the machine: every glyph is drawn from the embedded JetBrains Mono Nerd Font Mono, with
Chinese and Japanese from an embedded Noto Sans Mono CJK SC, so the same scene gives
byte-identical files everywhere, and the picture never shows more than the test screen.

## The picture a shared link shows

`qshots::Card` puts a screenshot, an application's name and one sentence on a canvas of a fixed
size: the card GitHub, Mastodon and the chat applications show when a repository's link is
shared.

```rust,ignore
qshots::Card::new(qshots::Shot::of(&harness))
    .name("qtools")
    .promise("Every tool you keep reaching for, in one window.")
    .save("docs/screenshots/social")?;
```

writes `docs/screenshots/social.svg` and a PNG of exactly 1280 x 640, the size those sites ask
for; `size(width, height)` asks for another. The ground is the theme's `canvas` with no
transparency anywhere, the name is drawn in the theme's accent colour and the sentence under it
in its text colour, both from the same embedded font as the screenshot, so the card is the same
file on every machine and needs no font on the one that shows it.

The screenshot keeps its shape and is scaled to fit the right of the card, whole: a compact
scene, around 80 columns, stays readable, a wide one turns to texture. Text is never cut: the
sentence wraps to its column, and a word or a sentence with no room left is an error naming what
did not fit. `missing()` reports every character the fonts have no glyph for, in the screenshot
as well as in the name and the sentence, so a Turkish or Chinese promise never comes out blank.

## Moving pictures

`qshots::Reel` records a scripted visit as a GIF and an MP4. It runs the harness's fake clock a
frame every 50 ms, so animations play and the same script gives the same frames everywhere; a
screen that did not change only holds the frame before longer. A drawn pointer glides cell by
cell, hovering every cell it crosses, and clicks where the script says, with the same glide and
the same settling pause whichever button or modifier the click carries
(`qframe::event::MouseButton`, `qframe::keymap::Modifiers`):

```rust,ignore
let mut reel = qshots::Reel::new(harness, "target/readme-reel")
    .title("qtools")
    .pointer_at(90, 20)
    // Runs on every frame's screen: a recording must never show a real folder.
    .check(|screen| assert!(!screen.contains("/home/"), "a real folder is on screen"));
reel.hold(Duration::from_secs(2));
reel.click_on("Apply").hold(Duration::from_secs(1));
reel.right_click_on("src").hold(Duration::from_secs(1)); // a context menu
// Another button or a held modifier, e.g. a ctrl click that adds to a selection:
reel.click_with(MouseButton::Left, Modifiers { ctrl: true, ..Modifiers::default() }, (12, 7));
reel.press("ctrl+p", Duration::from_millis(600)).type_slowly("theme", Duration::from_millis(180));
reel.harness_mut().send(Msg::Refresh); // anything the recorder has no word for
reel.skip(Duration::from_secs(5)); // time passes off camera: no frame, no length
// An application with a clock of its own moves it along with the recording's:
reel.hold_with(Duration::from_secs(3), |harness, step| {
    harness.send(Msg::Tick(step));
});
reel.glide((90, 20)).hold(Duration::from_secs(1));
let recording = reel.finish()?; // numbered PNG frames and frames.txt, ffmpeg's concat list
recording.encode("docs/screenshots/qtools.gif", "docs/screenshots/qtools.mp4", 1000)?;
```

`Reel::new` empties the frame folder, so no frame of an earlier run is left beside the new ones;
give it a folder of its own under `target/`.

`encode` needs ffmpeg on the `PATH` and returns an error naming the cause when it is missing or
fails. The frames are drawn at twice the terminal's size with square corners and scaled to the
given width; the GIF shares one palette without dithering, the MP4 is H.264 in `yuv420p`. Both
last exactly as long as `recording.total()`, so a looping GIF returns to its first frame the
moment the story ends instead of resting on the last one.

The code is MIT licensed; the embedded fonts are under the SIL Open Font License 1.1
(`fonts/OFL.txt`, `fonts/OFL-NotoSansMonoCJK.txt`).
