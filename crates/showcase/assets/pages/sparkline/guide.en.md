## When to use

Use a sparkline to show the shape of recent history next to a number: CPU over the last minutes, requests per second, queue depth. It answers "is this rising, falling or spiky?" in one row. For exact values or comparisons between things, use a bar chart or a table.

## Step by step

1. Keep a history of samples in your state, oldest first.
2. Draw it: `ui.add(Sparkline::new(values)).width(Length::Fill(1))`.
3. For measures with a known scale, fix it: `.range(0.0, 100.0)`; otherwise columns stretch between the lowest and highest value shown, which exaggerates small changes.
4. Put the current value next to it as text; the sparkline shows the trend, not the number.
5. Add capabilities when they help: `.highlight_extremes()` for the peak and the low, `.baseline(80.0)` for a limit, `.height(Length::Cells(3))` on the node for taller columns.

## How it works

- **Eighth-cell columns.** Each value is a column of `▁▂▃▄▅▆▇█`; taller sparklines stack whole cells and finish with a partial block, so three rows give 24 levels.
- **Every sample is visible.** The lowest value keeps one eighth, so a quiet moment does not look like missing data.
- **Newest on the right.** When there are more values than columns, the oldest are dropped.
- **Extremes and baseline.** The columns are one step below the accent so the peak can light up in the accent itself; the low is muted. The baseline is a tone band across the row at its level, drawn under the columns, not a line.
- **ASCII mode.** Columns fill whole cells with colour over a raised track, and the cell a column ends in takes the share of colour it covers. Even the lowest sample tints one cell, and a one-row sparkline reads as a strip of tones.

## Common mistakes

- **Auto scale for percentages.** A flat 40–42% load fills the whole height and looks alarming; give percentages `.range(0.0, 100.0)`.
- **A sparkline without its number.** Users read the current value first; show it beside the trend.
- **Too few samples.** Five columns are not a trend. Keep at least a few dozen.
