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

## Moving pictures

`qshots::Reel` records a scripted visit as a GIF and an MP4. It runs the harness's fake clock a
frame every 50 ms, so animations play and the same script gives the same frames everywhere; a
screen that did not change only holds the frame before longer. A drawn pointer glides cell by
cell, hovering every cell it crosses, and clicks where the script says:

```rust,ignore
let mut reel = qshots::Reel::new(harness, "target/readme-reel")
    .title("qtools")
    .pointer_at(90, 20)
    // Runs on every frame's screen: a recording must never show a real folder.
    .check(|screen| assert!(!screen.contains("/home/"), "a real folder is on screen"));
reel.hold(Duration::from_secs(2));
reel.click_on("Apply").hold(Duration::from_secs(1));
reel.press("ctrl+p", Duration::from_millis(600)).type_slowly("theme", Duration::from_millis(180));
reel.harness_mut().send(Msg::Refresh); // anything the recorder has no word for
reel.glide((90, 20)).hold(Duration::from_secs(1));
let recording = reel.finish()?; // numbered PNG frames and frames.txt, ffmpeg's concat list
recording.encode("docs/screenshots/qtools.gif", "docs/screenshots/qtools.mp4", 1000)?;
```

`encode` needs ffmpeg on the `PATH` and returns an error naming the cause when it is missing or
fails. The frames are drawn at twice the terminal's size with square corners and scaled to the
given width; the GIF shares one palette without dithering, the MP4 is H.264 in `yuv420p`.

The code is MIT licensed; the embedded fonts are under the SIL Open Font License 1.1
(`fonts/OFL.txt`, `fonts/OFL-NotoSansMonoCJK.txt`).
