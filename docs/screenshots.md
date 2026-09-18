# Screenshots for a README

`quvyta-framework-shots` (library `qshots`) turns a `Harness` screen into an SVG and a PNG. Every
glyph is an outline from the embedded JetBrains Mono Nerd Font Mono, so nothing depends on the
machine: the same scene gives byte-identical files everywhere, and the picture shows only what the
test drew.
Every application follows the same convention: a version-less path dev-dependency and one
ignored test named `readme_shots`.

## 1. Add the dev-dependency

```toml
[dev-dependencies]
# By path and without a version, so publishing the application leaves it out.
quvyta-framework-shots = { path = "../framework/crates/shots" }
```

## 2. Write one ignored test named `readme_shots`

```rust,ignore
#[test]
#[ignore = "writes docs/screenshots; run on purpose to refresh the README pictures"]
fn readme_shots() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/screenshots/");
    let mut harness = Harness::with_env(App::with_sample_data(), env(), 120, 36);
    harness.set_theme("iris").set_locale("en").set_glyph_mode(GlyphMode::Nerd);
    harness.advance(Duration::from_secs(2)); // let transitions finish
    let shot = qshots::Shot::of(&harness).title("qcode");
    assert!(shot.missing().is_empty(), "the font lacks {:?}", shot.missing());
    shot.save(format!("{dir}home")).unwrap(); // home.svg and home.png
}
```

`save` creates the folder and writes `<name>.svg` and `<name>.png` (twice the size). `to_svg()`
and `to_png()` return the same content for tests that check it without writing.

## 3. Run it

```sh
cargo test readme_shots -- --ignored
```

The normal test run skips the test, so the gate never writes a file. Commit the pictures, then
reference them from the README with absolute URLs, so they also show on crates.io:

```markdown
![qcode on its home screen](https://raw.githubusercontent.com/quvyta/<repo>/main/docs/screenshots/home.svg)
```

The SVG is small and sharp at any size; use the PNG where an SVG cannot be shown.

## Keep personal data out

- Build every scene from fixed sample data: made-up projects, hosts and names, never the real
  configuration, history or files of the machine running the test.
- Never show real paths. Use a fixed sample folder with a neutral name, or a path like
  `~/projects`; no user name, no home folder, no temporary folder.
- Fix everything that changes between runs: the theme, the language (`set_locale("en")`), the
  glyph mode, the clock (the harness clock starts at zero; move it with `advance`), sizes, and any
  count or size read from disk. If two runs give different files, something is not fixed.
- Look at every picture before committing it.
