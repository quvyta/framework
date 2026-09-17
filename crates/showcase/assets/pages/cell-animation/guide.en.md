## When to use

Use a cell animation when one cell should move: a spinner style, a finishing tick, a blinking status dot, a recording light. Every spinner style is one, so the same format redesigns the built-ins and adds your own. Use the studio on this page to try glyphs and colours in all three glyph modes before you write them into a file.

## Step by step

1. **Pick a start.** In the studio choose a built-in from *Start from*, or *New animation*. Pressing a name in the built-in list opens it too.
2. **Write the frames.** Each frame has three glyph fields. Fill ASCII first: every frame needs it. Leave Unicode empty to use the ASCII glyph there, and Nerd Font empty to use the Unicode glyph. A field also takes a code point such as `U+F0995` or `\uF0995`, handy for Nerd Font glyphs you cannot type.
3. **Add colour where it means something.** Leave *Colour* empty to take the widget's colour. Write `$success` for a theme colour, `mix($success, $fg, 50%)` for halfway between success and the widget's colour, `pulse($muted, $fg)` to breathe, or `#38BDF8` for a fixed colour.
4. **Set the pace.** *Frame time* follows a theme motion key (`spinner`, `step`, …) so every theme sets the speed, or a fixed number of milliseconds. A frame's *Duration* overrides it for that frame.
5. **Choose playback and colours.** `loop` repeats, `once` stops on the last frame, `bounce` goes forward and back. `step` shows each frame in its own colour; `blend` slides the colour towards the next frame.
6. **Copy TOML** and paste the `[animations.<name>]` block into an icon set or theme file, or send it to whoever maintains them.
7. **Use it.** `Spinner::new().animation("record-light")`, `Toast::info("Recording").icon_motion("record-light")`, or `cx.animation("record-light", style, Some(Duration::ZERO))` in your own widget.

## How it works

- **The format.** An `[animations.<name>]` table has `frame` (a motion key or a duration like `"80ms"`, default `"spinner"`), `playback` (`loop`, `once`, `bounce`), `colors` (`step`, `blend`), an optional `rest` frame number counted from 1, and `frames`, a list of `{ nerd, unicode, ascii, color, duration }`.
- **Fallbacks.** Nerd Font falls back to Unicode, Unicode to ASCII. Every glyph must be exactly one cell: a CJK character or an emoji is two cells and is refused with the file, line and column. Brackets are refused in every mode.
- **Where animations live.** Built-ins sit in the default icon set, next to the icons. A theme's `[animations.<name>]` replaces one animation, exactly like `[icons]` replaces one icon, and an icon set of your own can add more. A broken animation is reported and skipped; the one it tried to replace keeps working.
- **Older themes.** A theme that still writes a spinner icon such as `spinner-arc = { nerd = "…", unicode = "…", ascii = "…" }` keeps working: the glyphs replace the frames of that animation, and each new frame takes the colour of the frame at the same place.
- **`$fg`** is the colour of the widget drawing the animation: a spinner's tone, a button's text. The finish blends `mix($success, $fg, N%)` so it starts from whatever colour the spinner had.
- **Reduced motion** shows the rest frame standing still: the first frame, or the last for `once`. A `pulse()` then shows its second colour, so a resting dot keeps its strong colour.
- **The studio remembers** the animations you changed or created in the showcase settings, under `studio.animations`. *Reset to built-in* discards your changes to a built-in.

## Common mistakes

- **Wide glyphs.** Emoji and CJK characters take two cells; the studio says so under the frame and leaves the glyph out.
- **Hard-coded colours everywhere.** `#hex` ignores the theme; prefer `$tokens` and `mix()` so the animation fits every theme.
- **Nerd Font only.** Always give a meaningful ASCII glyph; many terminals have no Nerd Font.
- **Frame counts that fight.** One frame list serves all modes. If the ASCII sequence is shorter, repeat it so every mode keeps its rhythm, as the built-in Slices animation does with 40 frames.
