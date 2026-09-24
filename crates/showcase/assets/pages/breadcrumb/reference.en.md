## Methods

- `Breadcrumb::new(segments)` — the path from the root to the current place.
- `.on_select(|index| msg)` — sent when level `index` is opened; without it the breadcrumb is plain text.
- `.faint(bool)` — draws the path a step quieter, for a place the person cannot open; the levels still open.

## Behaviour

- Each segment is its label with one cell of padding on both sides; separators take one cell.
- When the path is too wide: root, `…`, and the last levels that fit; below that, `…` and the last level.
- `…` opens a list of the hidden levels under it; ↑ ↓ move, typing a letter jumps, Enter opens, Esc closes.
- Focusable only with `on_select` and at least two segments.

## Keys

- `left` `right` move between segments, `home` `end` jump, `enter` or `space` opens.

## Mouse

- Click a segment to open it; click `…` to list the hidden levels.

## Theme keys

- `crumb` — `bg`, `fg`; states `hover`, `focus`, `active` (the open `…`).
- `crumb.current` — `fg`, `bold` of the last level.
- `crumb.faint`, `crumb.faint-current` — the levels and the last level of a faint path; the states of `crumb` still apply.
- `crumb-separator` — `fg`.
- `popup-menu`, `popup-item`, `popup-check` — the list of hidden levels.

## Icons

- `crumb-separator`: a small chevron in Nerd Font and Unicode, `:` in ASCII.
