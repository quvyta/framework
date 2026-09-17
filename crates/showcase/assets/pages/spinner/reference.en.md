## Methods

- `Spinner::new()` — an arc spinner in the accent colour.
- `.style(SpinnerStyle)` — how it moves.
- `.animation(name)` — plays any cell animation by name instead, such as one your theme defines; an unknown name draws `⟦`.
- `.label(text)` — text two cells after the spinner, truncated to fit.
- `.variant(name)` — theme variant such as `"success"`, `"warning"`, `"danger"`.
- `.done(bool)` — the work is finished: play the tick once and rest on it. Off by default; turning it off spins again.

## Styles

- `Arc` — an arc sweeping round (animation `spinner-arc`). The default. Nerd Font `nf-extra-progress_spinner_1..6`, U+EE06 to U+EE0B (Nerd Font v3); Unicode `◜◠◝◞◡◟`; ASCII `-\|/`.
- `Dots` — braille dots turning (animation `spinner-dots`).
- `Orbit` — a dot circling a cell (animation `spinner-orbit`).
- `Pop` — a dot growing and shrinking (animation `spinner-pop`).
- `Pulse` — a dot breathing between the faint colour and the tone (animation `spinner-pulse`, colour `pulse($muted, $fg)`).
- `Quarters` — a filled quadrant of the cell turning clockwise, `▖▘▝▗` (animation `spinner-quarters`).
- `Slices` — a pie filling slice by slice, then starting again (animation `spinner-slices`): Nerd Font `nf-md-circle_slice_1..8`, U+F0A9E to U+F0AA5, needs Nerd Font v3; Unicode `◐◓◑◒`; ASCII `.oO@`.

`SpinnerStyle::ALL` lists them in this alphabetical order and `.name()` gives the short name; `.animation()` gives the animation name.

## The finish

The animation `spinner-done` holds five frames, played once (`playback = "once"`), one `motion.step` each:

1. `nf-md-progress_check` U+F0995, Unicode `·`, ASCII `.` — the spinner's colour.
2. `nf-fa-check_circle_o` U+F05D, Unicode `∙`, ASCII `.` — `mix($success, $fg, 25%)`.
3. `nf-oct-check_circle` U+F49E, Unicode `✓`, ASCII `v` — `mix($success, $fg, 50%)`.
4. `nf-md-check_circle_outline` U+F05E1, Unicode `✔`, ASCII `v` — `mix($success, $fg, 75%)`.
5. `nf-md-checkbox_marked_circle` U+F0133, Unicode `✔`, ASCII `v` — `$success`; the spinner rests here.

The Nerd Font glyphs need Nerd Font v3. The spinner's colour is `spinner` (or its variant); a pulse starts from the colour it showed when `done` turned on.

## Behaviour

- Measures one cell, plus two and the label's width when labelled. Every frame of every style and of the finish is one cell wide in every glyph mode, so finishing never moves the label.
- Requests a frame exactly when the next frame is due; a resting tick requests nothing.
- `done(true)` from the start (the first time the spinner is drawn) rests on the tick without playing.
- With reduced motion shows the first frame, or the full tone for `Pulse`, and requests nothing; a done spinner shows the last frame in its colour at once.

## Theme keys

- `spinner`, `spinner.<variant>` — `fg`.
- `spinner-label`, `spinner-label.<variant>` — `fg`, `bold`.
- `[motion]` — `spinner`, `pulse-period`, `step` (one finish frame).
- `[animations]` — `spinner-arc`, `spinner-dots`, `spinner-orbit`, `spinner-pop`, `spinner-pulse`, `spinner-quarters`, `spinner-slices`, `spinner-done`. The former icon keys (`spinner`, `spinner-arc`, …, `spinner-done`) still work in `[icons]`: they replace the glyphs of their animation and keep its timing and colours.
