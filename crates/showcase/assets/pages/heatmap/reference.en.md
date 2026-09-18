## Methods

- `Heatmap::new(values)` — a grid of `f32` values, oldest first, filling column by column, seven rows tall.
- `.rows(n)` — rows of the grid; at least one.
- `.max(value)` — the value the top step stands for; the largest value by default.
- `.starts_at(row)` — blank cells before the first value, kept within one column.
- `.series(index)` — builds the tones from the theme's n-th series tone instead of the accent.
- `.selected(Some(index))` — the value the cursor rests on.
- `.on_select(|index| msg)` — message for a cell chosen with a click or Enter; also what makes the heatmap interactive.
- `.columns(width)` — how many columns fit in `width` cells, newest kept.
- `Legend::new(names)` — names in the order a chart draws its series; the n-th takes the theme's n-th series tone.
- `.tones(indices)` — the palette index of every name, in order; a name past the end keeps its position's tone. Give it the numbers the chart's series were pinned to.
- `.vertical()` — one name per row.

## Behaviour

- Measures one column per week and `rows` rows; no values measure nothing at all.
- A value at or below zero takes the empty tone, any other value one of four steps; values above the scale stay at the top step.
- Narrow areas drop the oldest whole columns; short areas keep the rows that fit from the top.
- A 16-colour terminal drops the steps it cannot tell apart and spreads the levels over the tones left.
- Focusable and clickable only with `on_select`: ←/→ move a column, ↑/↓ a day, Home/End go to the ends, Enter or a click reports the cell.
- A legend takes no focus and sends no messages; long names are cut with `…`.

## Theme keys

- `heatmap` — `empty` (a day that holds nothing), `fill` (the full tone), `cursor` (the tone a lit cell steps towards).
- `heatmap:focus` — `cursor` while the keyboard moved the cursor last.
- `legend` — `fg` (the names).
- Colour tokens `series-1` to `series-5`, through `Theme::series_color(index)`, which wraps around after five.
