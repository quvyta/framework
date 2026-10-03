## Methods

- `LevelBars::new(values)` — one column per value, each a level between zero and one, lowest band
  first.
- `.gap(cells)` — empty cells between the columns; one by default.
- `.bar_width(cells)` — cells a column is drawn in; one by default, wider when the values leave
  room, cut to the area when it is narrower than one.
- `.peaks(peaks)` — a cap on each column at the level given, one value per value; `peaks([])` for
  none.
- `.mirror(on)` — grows the columns up and down from the middle row.
- `.gradient(on)` — blends the column from the theme's base tone to the accent as it deepens.

## Behaviour

- Measures one column per value in eighths of a cell; give the node a width to fill and a height for
  the precision you want, one row being eight levels.
- A level below zero is clamped to it and a level that is not a number is silence.
- Values that do not fit are merged by averaging their neighbours, so the whole spectrum shows in
  a narrow area; a merged column's cap is the mean of the caps it covers.
- Room to spare widens every column by the same share, the cells that cannot be shared out going to
  the outermost columns, so the two ends of a row match.
- `.mirror(true)` needs two rows: in a one-row area both halves would be thinner than a cell, so the
  column draws as a plain one. A level that cannot be shared evenly grows upwards first.
- A cap is drawn when its level is above the level of its column and has a cell of its own: the
  block that reaches it, or the thin `▔` line where the level is the top of a cell.
- Nothing is drawn outside the area, and a value list that is empty draws nothing at all.
- In ASCII glyphs whole cells take the tone of the level over the track and the cell a column ends
  in takes the share of it that fills it, so even a quiet level tints a cell.
- In the sixteen standard colours the gradient is left out and every column is one tone.
- A level bar answers no key and no click: it takes no focus and sends no messages.

## Theme keys

- `level-bars` — `base` (the columns, and the bottom of a gradient), `peak` (a cap, and the top of
  a gradient), `track` (the ground in ASCII mode).
