## Methods

- `Divider::new()` — one empty row.
- `.space(cells)` — rows of space, or columns when vertical; 1 by default.
- `.label(text)` — a caption on its own row after the space; horizontal dividers only.
- `.band()` — paints the divider in the band tone.
- `.vertical()` — separates side by side content; combine with `.fill_height()`.

## Behaviour

- Horizontal: measures the full width and `space` rows, plus one row for a caption.
- Vertical: measures `space` columns and one row; `fill_height` stretches it across the row.
- The caption is cut with `…` when it does not fit.
- Never draws a line character in any glyph mode. It is not focusable.

## Theme keys

- `divider` — `band`.
- `divider-label` — `fg`, `bold`.
