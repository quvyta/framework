## Methods

- `Tooltip::new(text)` — wraps the widgets added with `ui.add_with(…)` and explains them with `text`.
- `.placement(Placement)` — `Below` (default), `Above`, `Right` or `Left`; flips when there is no room.
- `.on_focus(bool)` — also shows, at once, while keyboard focus is inside. Default: `false`.

## Behaviour

- Shows after the pointer rests for `motion.hover-delay`; hides when the pointer leaves.
- One row, shortened with `…` to the screen width; never covers the pointer cell.
- Draws no hit area of its own in the layer, so clicks reach what is under it.

## Theme keys

- `tooltip` — `bg` (default `$overlay`), `fg` (default `$text`), `padding` (default `[0, 1]`).
- `[motion] hover-delay` — how long the pointer rests before the tooltip appears.
- `[motion] enter` — how long the text takes to fade in.
