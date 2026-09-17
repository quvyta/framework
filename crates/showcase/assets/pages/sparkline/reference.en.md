## Methods

- `Sparkline::new(values)` — a sparkline of `f32` values, oldest first.
- `.range(min, max)` — scales over a fixed range instead of the values shown.
- `.highlight_extremes()` — colours the latest highest and lowest column.
- `.baseline(value)` — tints the row at the level of `value`.

## Behaviour

- Measures one column per value and one row; give the node a width to fill and a height for taller columns.
- Shows the newest values that fit; the lowest value keeps one eighth, the highest fills the column.
- Values outside a fixed range are clamped.
- Not focusable; sends no messages.

## Theme keys

- `sparkline` — `fg` (columns), `peak`, `low`, `baseline` (band), `track` (ASCII ground).
