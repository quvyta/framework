## When to use

Use a splitter when two areas are worked with side by side and people want to decide how much room each gets: a file list next to an editor, a source view above a build log. When the layout is fixed, a row or column is enough.

## Step by step

1. Keep the first pane's size in your application: `files: u16`.
2. Pick the direction: `Splitter::columns(state.files)` for panes side by side, `Splitter::rows(state.log)` for stacked panes.
3. Fill the panes: `.first(|ui| ...)` and `.second(|ui| ...)`, then `.show(ui)`.
4. Make it resizable: `.on_resize(|size| Msg::Files(size))` and store the size in `update`.
5. Set limits so neither pane becomes useless: `.limits(16, 60)` for a range, or `.limits(16, None)` for only a minimum.
6. Give the splitter room: it fills what it gets, so place it in a sized column or a filled area.

## How it works

- **No line, ever.** The boundary is one empty cell that looks like the ground around it. Under the pointer it brightens one step, while dragged it takes the accent, and focused it carries a faint accent tint.
- **Drag or keyboard.** Tab reaches the boundary; ← → (or ↑ ↓ for stacked panes) move it one cell, Shift moves five, Home and End jump to the limits.
- **Your state decides.** Dragging only sends sizes, already within the limits; the panes move when you store them.
- **Limits are yours.** By default the first pane is at least 1 cell and has no upper limit. `.limits(min, max)` takes a number or `None` for `max`, exactly like the side panel; the playground switches the files pane between no limit, 16–40 and 24–48.
- **The second pane always keeps a cell,** however small the area gets, and the first never goes past the limits.
- **Splitters nest.** A pane can hold another splitter, each with its own size.

## Common mistakes

- **Ratios in state.** The size is in cells; if the window changes a lot, clamp or recompute it in `update`.
- **An unsized splitter inside a scroll view.** It would try to take the whole scroll height; give its container a height.
- **Drawing a divider in a pane.** The boundary already shows itself when needed; a drawn line breaks the look.
