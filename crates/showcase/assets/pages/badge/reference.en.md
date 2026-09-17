## Methods

- `Badge::new(label)` — a neutral badge reading `label`.
- `.variant(name)` — tone: `"success"`, `"warning"`, `"danger"`, `"info"`, `"accent"`, or any variant your theme defines.
- `.count(n)` — adds a count segment after the label; above 99 it reads `99+`.

## Behaviour

- Measures one row: padding, marker, a space, the label, padding, then the count segment.
- When given less width, the label is cut with `…`; the marker and count stay.
- The marker is the `dot` icon, so it follows the glyph mode.
- It is not focusable and sends no messages.

## Theme keys

- `badge`, `badge.<variant>` — `bg`, `fg`, `dot`.
- `badge-count`, `badge-count.<variant>` — `bg`, `fg`, `bold`.
- Icon `dot` — the marker.
