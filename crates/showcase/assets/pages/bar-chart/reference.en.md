## Methods

- `Bar::new(label, value)` — one bar; negative values count as zero.
- `Bar::value_text(text)` — text shown for the value.
- `Bar::variant(name)` — theme variant of the bar; adds a marker before its value.
- `BarChart::new(bars)` — a horizontal chart.
- `.vertical()` — bars side by side, value above, label below.
- `.max(value)` — the value of a full bar; the largest value by default.
- `.gap(cells)` — rows between horizontal bars or columns between vertical bars; 1 by default. Vertical bars keep at least one column between them, so they never merge.

## Behaviour

- Horizontal charts measure one row per bar plus the gaps; `.gap(0)` packs them for dense lists; vertical charts measure eight rows unless given a height.
- Horizontal labels are dropped below 24 cells of width. Vertical bars are at most six cells wide.
- Eighth-cell precision; whole cells in ASCII mode. Not focusable; sends no messages.

## Theme keys

- `bar-chart`, `bar-chart.<variant>` — `fill`.
- `bar-chart-label` — `fg`.
- `bar-chart-value`, `bar-chart-value.<variant>` — `fg`, `bold`.
