## Methods

- `Sparkline::new(values)` — a sparkline of `f32` values, oldest first.
- `.range(min, max)` — scales over a fixed range instead of the values shown.
- `.highlight_extremes()` — colours the latest highest and lowest column.
- `.baseline(value)` — tints the row at the level of `value`.
- `.reading(index)` — the sample being read, as a position in the values given; `None` for none.
- `.on_read(message)` — lets a sample be read with the pointer and the keyboard; the message carries the position read, or `None` when reading stops.

## Behaviour

- Measures one column per value and one row; give the node a width to fill and a height for taller columns.
- Shows the newest values that fit; the lowest value keeps one eighth, the highest fills the column.
- Values outside a fixed range are clamped.
- Without `on_read` it is not focusable and sends no messages.
- With `on_read`: a left press reads the column pressed, a drag scrubs and clamps to the ends, and while focused ←/→ move one sample, Home and End go to the oldest and newest shown, and Esc sends `None`. The first key reads the newest sample. A message is only sent when the sample read changes.
- A `reading` outside the samples shown is not marked, and the keys carry on inside the window that is shown.
- The read column is drawn in the accent on the active surface, a hovered column on a ground one hover step brighter; no cell moves for either.

## Theme keys

- `sparkline` — `fg` (columns), `peak`, `low`, `reading` (the column read, the accent by default), `baseline` (band), `track` (ASCII ground).
