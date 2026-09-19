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

The code is MIT licensed; the embedded fonts are under the SIL Open Font License 1.1
(`fonts/OFL.txt`, `fonts/OFL-NotoSansMonoCJK.txt`).
