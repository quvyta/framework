## When to use

Use a segmented control for two to five short, mutually exclusive choices that change how something is shown: list, grid or tree; the last hour, day or week. It reads faster than a radio group and takes one row.

## Step by step

1. Keep the choice in your application: `view: usize`.
2. Draw it: `Segmented::new(["List", "Grid", "Tree"]).selected(state.view)`.
3. Handle choices: `.on_select(|index| Msg::View(index))` and redraw the content for the new choice.
4. Keep labels to a word or two; translate them like any text.

## How it works

- **One surface, one filled segment.** Segments sit side by side without gaps or dividers; the chosen one is filled with the accent and bold, the others share the raised tone.
- **Hover lifts a segment** and shows the pillar `▌` in that segment's first cell, never at the far left of the control, so the click target is clear. Nothing slides.
- **Keyboard.** Tab reaches the control once; Left and Right choose the neighbour at once, Home and End the ends. While the keyboard focuses the control, the chosen segment breathes with its pillar.

## Common mistakes

- **Actions in segments.** Segments choose a state; "Refresh" is a button.
- **Too many or too long segments.** Past five, or with sentences, use a select or tabs.
