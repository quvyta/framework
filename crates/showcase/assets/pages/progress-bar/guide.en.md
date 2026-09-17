## When to use

Use a progress bar when you know how much of the work is done: an upload, a migration, a test run. It also shows how full something is, like a disk or a quota. When the size of the work is unknown, use the indeterminate bar or a spinner.

## Step by step

1. Add a bar with a value from 0 to 1: `ui.add(ProgressBar::new(0.45)).width(Length::Fill(1))`.
2. Update the value from your state as work reports progress; the bar redraws with the new value.
3. Give it a tone that tells the story: `.variant("success")` when finished, `"warning"` when a disk is nearly full, `"danger"` when a quota is exhausted.
4. Hide the percentage when a label elsewhere already says it: `.percent(false)`.
5. When you cannot measure progress, use `ProgressBar::indeterminate()`.

## How it works

- **Eighth-cell precision.** Whole cells are filled with colour and the last cell uses a partial block, so a 40-cell bar has 320 visible steps. In ASCII mode it fills whole cells, rounded to the nearest one like the charts.
- **No frame, no brackets.** The track is a quiet surface and the fill is a colour; there is nothing around them.
- **Indeterminate sweep.** A band of light travels along the track once per `motion.shimmer`, each cell blended by its distance from the band.
- **Reduced motion.** The indeterminate bar rests in a faint even tint.

## Common mistakes

- **Going backwards.** Progress that shrinks breaks trust; if the estimate changes, move to indeterminate.
- **Staying at 100%.** Replace a finished bar with the result or turn it to the success tone.
- **Tiny bars.** Below about ten cells the steps are hard to read; give it room.
