## Methods

- `Splitter::columns(width)` — panes side by side; the left pane is `width` columns.
- `Splitter::rows(height)` — stacked panes; the top pane is `height` rows.
- `.limits(min, max)` — bounds of the first pane; `max` is a number or `None` for no upper limit; 1 and `None` by default.
- `.on_resize(|size| msg)` — makes the boundary draggable and focusable.
- `.first(|ui| ...)`, `.second(|ui| ...)` — the panes; `.show(ui)` adds the splitter, filling its area.

## Keys

- On the focused boundary: `left` `right` (columns) or `up` `down` (rows) move one cell, with `shift` five; `home` `end` jump to the limits.

## Mouse

- Drag the boundary. Hovering it brightens it.

## Behaviour

- Without `on_resize` there is no boundary cell and nothing is focusable.
- The second pane always keeps at least one cell, whatever the limits; a `max` below `min` counts as `min`.

## Theme keys

- `split-handle` — `bg`, `fg`; states `hover`, `focus`, `active` (dragging). No state means invisible.
