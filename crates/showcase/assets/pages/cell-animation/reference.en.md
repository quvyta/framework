## Methods

- `CellAnimation::new()` — no frames, `FrameTime::Motion("spinner")`, `Playback::Loop`, `ColorMode::Step`.
- `.frame(AnimationFrame)`, `.frame_time(FrameTime)`, `.playback(Playback)`, `.colors(ColorMode)`, `.rest(index)` — build one in code; `rest` counts from 0.
- `.sample(theme, fg, now, since) -> CellFrame` — the frame index, colour, whether a single play finished, when the frame changes and whether the colour moves; `since = None` stands still on the rest frame.
- `.glyph(index, GlyphMode)` — a frame's glyph after fallbacks. `.to_toml(name)` — the `[animations.<name>]` block.
- `AnimationFrame::new(ascii)` then `.unicode(glyph)`, `.nerd(glyph)`, `.color(CellColor)`, `.duration(FrameTime)`.
- `CellColor::parse("mix($success, $fg, 50%)")`, `CellColor::from(Rgb)`; `FrameTime::parse("step" | "80ms")`.
- `check_glyph(glyph, mode)` — the one-cell check the loader runs. `parse_animations(file, text)` — reads a file of animations with located diagnostics.
- `PaintCx::animation(name, style, since) -> AnimatedCell` — the glyph and style of a registered animation now, in the terminal's glyph mode; schedules the next frame.
- `Icons::animation(name)`, `Icons::animation_names()` — the registered animations.
- `Spinner::animation(name)`, `SpinnerStyle::animation()`, `Toast::icon_motion(style or name)`.

## Behaviour

- **File format:** `[animations.<name>]` with `frame`, `playback`, `colors`, `rest` (1-based) and `frames = [{ nerd, unicode, ascii, color, duration }]`. Names use lowercase letters, digits and `-`. At most 256 frames.
- **Fallback:** nerd → unicode → ascii; `ascii` required; every glyph one grapheme, one cell, no brackets; ASCII printable only.
- **Colours:** `$token`, `#RRGGBB`, `mix(a, b, N%)` (N% of a), `pulse(a, b)` only as a whole value; `$fg` is the widget's colour; an unknown token takes the widget's colour.
- **Timing:** frames change exactly on whole-millisecond boundaries counted from `since`; looping spinners count from zero so they turn in step. Pulses follow `motion.pulse-period`.
- **Blend:** at a frame's start its own colour, then a straight mix towards the next frame in play order; the last frame of `once` does not blend.
- **Layering:** built-in default icon set, then the chosen icon set, then the theme chain. In one layer, former spinner icon keys (`spinner`, `spinner-arc`, `spinner-done`, `spinner-orbit`, `spinner-pop`, `spinner-quarters`, `spinner-slices`) replace their animation's glyphs first, then `[animations]` apply.
- **Loader:** never panics; each problem is a diagnostic with file, line and column, and the whole animation is skipped.
- **Studio storage:** `studio.animations` in the showcase settings, a TOML text of the changed and new animations, under the open `studio` prefix.

## Theme keys

- `[animations.<name>]` in icon set and theme files.
- Built-in names: `spinner-arc`, `spinner-done`, `spinner-dots`, `spinner-orbit`, `spinner-pop`, `spinner-pulse`, `spinner-quarters`, `spinner-slices`.
- `[motion]` keys usable as frame times: `spinner`, `step`, `flash`, `enter`, `shimmer`, `pulse-period`, `cursor-blink`, `page`, `hover-delay`.
- Previews here draw in the `spinner` and `spinner-label` styles.
