## When to use

Use a bar chart to compare a few values side by side: memory per service, deploys per day, errors per endpoint. Horizontal bars suit long names and ranked lists; vertical bars suit a sequence such as days or hours. For a trend over many samples, use a sparkline.

## Step by step

1. Build the bars: `Bar::new("postgres-16", 1536.0)`.
2. Show values in their unit: `.value_text("1536 MiB")`.
3. Add them to a chart: `ui.add(BarChart::new(bars)).width(Length::Cells(72))`.
4. Mark only the bars that mean something: `.variant("danger")` for a service over its limit. Leave the rest in the accent.
5. For a sequence, stand them up: `.vertical()` and give the node a height.
6. When bars should be read against a known full value, set it: `.max(100.0)`.

## How it works

- **Eighth-cell bars.** Horizontal bars end with `▏`…`▉`, vertical bars with `▁`…`▇`, so close values still look different.
- **Room between bars.** One empty row separates horizontal bars by default, so each reads as its own shape; `.gap(0)` packs them.
- **Labels and values.** Horizontal: labels on the left (at most two fifths of the width, cut with `…`), values right-aligned. Vertical: the value sits just above each bar and the label below it.
- **Meaning with a marker.** A bar with a variant carries a `●` before its value, so the danger bar reads as special without colour.
- **Narrow areas.** Below 24 cells horizontal charts drop their labels; vertical bars get thinner, down to one cell, and labels are cut.
- **ASCII mode.** Bars round to whole cells.

## Common mistakes

- **A different colour per bar.** Colour is for meaning; a rainbow of services makes the danger bar invisible.
- **Unsorted horizontal bars.** Ranked data reads best largest first.
- **Too many bars.** Past a dozen, consider a table with a small bar per row.
