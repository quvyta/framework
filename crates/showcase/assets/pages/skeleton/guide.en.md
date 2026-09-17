## When to use

Use skeletons while content whose shape you know is loading: a list of containers, a card with a chart, a detail pane. The screen keeps its layout, so nothing jumps when the data arrives. For work with no content to wait for, such as a deploy, use a progress bar or a spinner.

## Step by step

1. Build the loading view with the same layout as the loaded view.
2. Put skeletons where content will be: `Skeleton::avatar()` for an icon, `Skeleton::lines(2)` for a name and a detail, `Skeleton::block()` for a chart or image.
3. Size them like the real content: `.width(Length::Cells(36))` for lines, `.height(..)` for blocks.
4. When the data arrives, draw the real rows in the same place.
5. If loading fails, replace the skeletons with an error message; do not leave them sweeping.

## How it works

- **Shapes in a quiet tone.** Placeholders are drawn in the raised tone. Text lines use the upper half of each row, so stacked lines read as separate lines; widths vary and the last line is short, like a real paragraph.
- **The signature sweep.** A band of light passes over the shapes once per `motion.shimmer`, blended cell by cell. The band is placed by screen column, so every skeleton on the screen is part of one sweep instead of each flickering on its own.
- **Reduced motion.** The shapes stand still in their resting tone.
- **ASCII mode.** Half blocks are not used; lines fill whole cells with colour.

## Common mistakes

- **A different layout while loading.** If the skeleton does not match the loaded view, the screen jumps anyway.
- **Skeletons forever.** Empty results need an empty state, failures an error. A skeleton promises content.
- **Too many shapes.** Three placeholder rows say "a list is coming"; fifty only add noise.
