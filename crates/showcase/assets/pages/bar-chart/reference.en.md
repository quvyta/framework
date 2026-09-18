## Methods

- `Bar::new(label, value)` — one bar; negative values count as zero.
- `Bar::value_text(text)` — text shown for the value.
- `Bar::variant(name)` — theme variant of the bar; adds a marker before its value.
- `Series::new(name, values)` — one series, with one value per category in the order of the categories; a shorter series counts as zero from there on.
- `Series::tone(index)` — takes the theme's `index`-th series tone instead of the tone of the series' position, so a category keeps its colour in every chart. Without it nothing changes.
- `BarChart::new(bars)` — a horizontal chart of plain bars.
- `BarChart::series(labels, series)` — a horizontal chart of the categories `labels` with one value per category in every series; the bars of a category stand next to each other.
- `.stacked()` — draws the series as segments of one bar per category.
- `.vertical()` — bars side by side, value above, label below.
- `.max(value)` — the value of a full bar; the largest value by default, or the largest category total when stacked.
- `.gap(cells)` — rows between horizontal categories or columns between vertical ones; 1 by default. Vertical charts keep at least one column between categories, so they never merge; the bars inside a group have no gap.
- `.unit(text)` — written after every value the chart formats itself; a bar with its own `value_text` keeps that text.
- `.selected(category)` — the category drawn on a raised ground.
- `.on_select(message)` — message for moving the selection to a category, which turns the pointer and keyboard handling on.
- `.disabled(disabled)` — greys the chart out; it takes no focus, answers no key and no pointer.

## Behaviour

- Horizontal charts measure one row per bar of a category plus the gaps; `.gap(0)` packs them for dense lists; vertical charts measure eight rows unless given a height.
- Horizontal labels and the series names before values are dropped below 24 cells of width. Vertical bars are at most six cells wide; a group too thin for a column per series is stacked instead.
- A vertical area of two rows drops the value row and one of a single row drops the labels as well.
- Eighth-cell precision, with whole cells in ASCII mode; a value above zero always reaches at least one eighth. Segments of a stack end on cell edges and the bar's eighth-cell tail belongs to the last series in it.
- Focusable only with `on_select`. Keys: ↑↓ or k and j in a horizontal chart, ←→ or h and l in a vertical one, Home and End for the ends. A left click picks the category under the pointer; the gap between two categories belongs to neither. Selecting the category that is already selected sends nothing.
- Hover and selection change no width, no height and no column: the lead cells of a chart that can show a selection are kept free from the first frame, and a chart never slides.

## Theme keys

- `bar-chart`, `bar-chart.<variant>` — `fill`.
- `bar-chart-bar` with `hover`, `selected`, `focus` — `bg`, `pillar`. Without them a hovered category takes the raised surface, a selected one the active surface, and the pillar the accent.
- `bar-chart-label` — `fg`.
- `bar-chart-value`, `bar-chart-value.<variant>` — `fg`, `bold`.
- `series-<n>` colour tokens — the tone of series `n`. Without them the tones walk a ramp from `accent` towards `muted`, repeating after four series; a disabled chart walks a ramp from `muted` towards `dim`.
