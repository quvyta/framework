## When to use

Use a sparkline to show the shape of recent history next to a number: CPU over the last minutes, requests per second, queue depth. It answers "is this rising, falling or spiky?" in one row. For exact values or comparisons between things, use a bar chart or a table.

## Step by step

1. Keep a history of samples in your state, oldest first.
2. Draw it: `ui.add(Sparkline::new(values)).width(Length::Fill(1))`.
3. For measures with a known scale, fix it: `.range(0.0, 100.0)`; otherwise columns stretch between the lowest and highest value shown, which exaggerates small changes.
4. Put the current value next to it as text; the sparkline shows the trend, not the number.
5. Add capabilities when they help: `.highlight_extremes()` for the peak and the low, `.baseline(80.0)` for a limit, `.height(Length::Cells(3))` on the node for taller columns.
6. To let someone read a single sample, keep the read sample in your state and wire it up: `.reading(self.reading).on_read(Msg::Read)`. Write the value yourself next to the trend; the sparkline marks the column, you say what it means.

## How it works

- **Eighth-cell columns.** Each value is a column of `▁▂▃▄▅▆▇█`; taller sparklines stack whole cells and finish with a partial block, so three rows give 24 levels.
- **Every sample is visible.** The lowest value keeps one eighth, so a quiet moment does not look like missing data.
- **Newest on the right.** When there are more values than columns, the oldest are dropped.
- **Extremes and baseline.** The columns are one step below the accent so the peak can light up in the accent itself; the low is muted. The baseline is a tone band across the row at its level, drawn under the columns, not a line.
- **Reading a sample.** A press reads the column under the pointer and a drag scrubs along the series, past its ends too, so a value never slips away. While focused, ← and → move one sample, Home and End jump to the oldest and the newest shown, and Esc stops reading. Both ways send the same message, so mouse and keyboard reach the same values.
- **Marked by tone, not by movement.** The read column stands on the active surface and is drawn in the accent; a hovered column's ground is one hover step brighter. Nothing slides: a sparkline is not a row structure, and the column a pointer left stays where it was.
- **ASCII mode.** Columns fill whole cells with colour over a raised track, and the cell a column ends in takes the share of colour it covers. Even the lowest sample tints one cell, and a one-row sparkline reads as a strip of tones.

## Common mistakes

- **Auto scale for percentages.** A flat 40–42% load fills the whole height and looks alarming; give percentages `.range(0.0, 100.0)`.
- **A sparkline without its number.** Users read the current value first; show it beside the trend.
- **Too few samples.** Five columns are not a trend. Keep at least a few dozen.
- **A reading with nowhere to read it.** `.on_read` only tells you which sample it is; without a line of text beside the trend the mark says nothing.
