## Methods

- `Skeleton::lines(count)` — `count` text lines of varied widths, the last one short.
- `Skeleton::avatar()` — a two-cell placeholder for an icon or avatar.
- `Skeleton::block()` — a block filling its area; three rows unless the node has a height.

## Behaviour

- Lines measure the full width and `count` rows; blocks the full width and three rows; avatars two cells and one row.
- Requests animation frames while motion is not reduced; with reduced motion nothing moves.
- The sweep position depends on the screen column, so neighbouring skeletons are lit in one pass.
- Not focusable; sends no messages.

## Theme keys

- `skeleton` — `bg` for the shapes, `highlight` for the light.
- `[motion]` — `shimmer` for one pass of the sweep.
