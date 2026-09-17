## When to use

Use a divider to pause between two groups inside the same surface: running and stopped services, today's and older deploys, two figures side by side. When the groups are different things, give each its own panel instead; surfaces separate better than anything drawn between them.

## Step by step

1. Put a plain divider between two groups in a column: `ui.add(Divider::new())`. It is one empty row.
2. Name the group that follows with a caption: `Divider::new().label("STOPPED")`. Write captions the way panel titles are written.
3. Give it more air when the groups are large: `.space(2)`.
4. When empty space alone is too weak, for example on a busy screen, paint it: `.band()`.
5. Between things side by side, use `Divider::new().vertical().band()` and add `.fill_height()` so it spans the row.

## How it works

- **Why there is no line.** A line across the screen is the loudest thing on it, and it says nothing about what it separates. In Quvyta, grouping comes from space and tone, the same way panels and selected rows are shaped.
- **Space first.** A plain divider draws nothing. One empty row is enough for the eye to see two groups, and it never competes with the content.
- **Captions belong below.** The caption sits on its own row after the space, directly above the group it names, so it reads as the start of that group rather than the end of the previous one.
- **Bands are tone.** A band fills the divider with a colour between surface and raised: a gutter of quieter tone, not a stroke. It works on the canvas and inside panels.

## Common mistakes

- **Dividers between everything.** If every row is separated, nothing is grouped. Use them between groups only.
- **A divider next to a panel edge.** Surfaces already separate; a band beside a panel just thickens the edge.
- **Forgetting `fill_height` on vertical dividers.** Without it the band is one row tall.
