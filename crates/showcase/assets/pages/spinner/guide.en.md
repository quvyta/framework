## When to use

Show a spinner when something is working and you cannot say how far along it is: pulling an image, waiting for a server, starting a container. If you know the progress, use a progress bar instead.

## Step by step

1. Add it where the work is shown: `ui.add(Spinner::new())`.
2. Say what is happening: `.label(t!("pulling"))`. A spinner alone tells people *that* something runs, the label tells them *what*.
3. The default `Arc` suits most places. Pick another style when the place asks for it: `.style(SpinnerStyle::Pulse)` for calm background health, `Dots` for busy work, `Slices` for a filling pie.
4. Give it a tone when the state matters: `.variant("warning")` while retrying, `"danger"` while a connection is lost.
5. When the work succeeds, finish it in place: `.done(true)`, and change the label to the result ("Image deploy-api built"). Give the spinner an `.id(..)` so it is the same widget before and after. When the work fails, remove the spinner and say what went wrong.

## How it works

- **One cell.** A spinner never changes the layout around it; the label starts two cells later. Every frame of every style, and of the finish, is exactly one cell wide; `Quarters` turns a quadrant block `▖▘▝▗` that terminals draw themselves, so it never spills over in any font.
- **Styles in alphabetical order.** `SpinnerStyle::ALL` lists Arc, Dots, Orbit, Pop, Pulse, Quarters and Slices; Arc comes first and is the default.
- **Every style is a cell animation.** Its frames have Nerd Font, Unicode and ASCII glyphs, so it works in any terminal. A theme replaces one with `[animations.spinner-arc]`; the Animation studio page shows them all in every glyph mode and edits copies. `Spinner::animation(name)` plays any other animation.
- **Nerd Font glyphs.** In Nerd Font mode `Arc` turns `nf-extra-progress_spinner_1..6` (U+EE06 to U+EE0B) and `Slices` fills a pie with `nf-md-circle_slice_1..8` (U+F0A9E to U+F0AA5); both need Nerd Font v3. In Unicode mode `Arc` sweeps `◜◠◝◞◡◟` and `Slices` turns the half circles `◐◓◑◒`; in ASCII mode they use `-\|/` and `.oO@`.
- **Speed comes from the theme.** Frames change every `motion.spinner`; the pulse style breathes over `motion.pulse-period`, blending from the faint colour to the tone.
- **The finish.** When `done` turns on, the spinner stops turning and plays the five frames of the animation `spinner-done` once, one `motion.step` each, then rests on the last. The tick grows at every step and its colour blends in equal steps from the spinner's colour (a pulse starts from the colour it had at that moment) to `$success`, the theme's success colour: the middle frame is the middle colour. Turning `done` off spins again.
- **The finish glyphs.** Nerd Font: `nf-md-progress_check`, `nf-fa-check_circle_o`, `nf-oct-check_circle`, `nf-md-check_circle_outline`, `nf-md-checkbox_marked_circle` (U+F0995, U+F05D, U+F49E, U+F05E1, U+F0133), which need Nerd Font v3. Unicode: `·∙✓✔✔`. ASCII: `..vvv`. Where two frames share a glyph, the colour still moves on.
- **Reduced motion.** A turning spinner shows its first frame and stands still; a done spinner shows the final tick in the final colour at once. A spinner that is already done when it first appears also rests on the tick without playing.

## Common mistakes

- **Spinners forever.** If work fails, stop the spinner and say so.
- **A tick for a failure.** `done` means the work succeeded; the tick is the success colour. Show an error instead.
- **Many spinners at once.** A screen full of motion is noise; group work under one spinner with a count.
- **A spinner for known progress.** People want to know how long; use `ProgressBar::new(value)`.
