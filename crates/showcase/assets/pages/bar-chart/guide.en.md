## When to use

Use a bar chart to compare a few values side by side: memory per service, deploys per day, errors per endpoint. Horizontal bars suit long names and ranked lists; vertical bars suit a sequence such as days or hours. When a category is made of several parts, give the chart series: stacked when the total matters, grouped when the parts are to be compared. For a trend over many samples, use a sparkline.

## Step by step

1. Build the bars: `Bar::new("postgres-16", 1536.0)`.
2. Show values in their unit: `.value_text("1536 MiB")`, or give the whole chart a unit with `.unit("h")`.
3. Add them to a chart: `ui.add(BarChart::new(bars)).width(Length::Cells(72))`.
4. Mark only the bars that mean something: `.variant("danger")` for a service over its limit. Leave the rest in the accent.
5. For a sequence, stand them up: `.vertical()` and give the node a height.
6. When bars should be read against a known full value, set it: `.max(100.0)`.
7. For several values per category, name the categories and the series: `BarChart::series(days, [Series::new("Rust", hours)])`. Bars stand next to each other; `.stacked()` puts them in one bar.
8. To let people point at a category, keep the selection in your state and pass it back: `.selected(state.day).on_select(Msg::Day)`.
9. To keep a category in one colour from week to week, pin it to a tone of the palette: `Series::new("Docs", hours).tone(1)`. Give the legend the same numbers with `Legend::new(names).tones([1, 2])`.

## How it works

- **Eighth-cell bars.** Horizontal bars end with `▏`…`▉`, vertical bars with `▁`…`▇`, so close values still look different. A value that is there at all reaches at least one eighth, so a small share next to a much larger one is seen rather than read as nothing.
- **Room between bars.** One empty row separates horizontal bars by default, so each reads as its own shape; `.gap(0)` packs them. The bars of one group stand right next to each other, so a group reads as one shape.
- **Labels and values.** Horizontal: labels on the left (at most two fifths of the width, cut with `…`), values right-aligned. Vertical: the value sits just above each bar and the label below it. A group writes the series name before each value; a stack writes the total.
- **Series tones.** The theme decides, through its `series-<n>` colour tokens. Without them the tone walks a ramp from the accent towards the faint end of the theme, so a chart stays inside the theme's one accent. The ramp repeats after four series.
- **A stack names itself.** A horizontal segment writes its series name inside it when the name fits with a cell of air on each side, so a stack is never read by colour alone. Where the names do not fit — a narrow bar, a standing bar — name the series in a line of text beside the chart.
- **Meaning with a marker.** A bar with a variant carries a `●` before its value, so the danger bar reads as special without colour.
- **Hover and selection.** Given `on_select`, the category under the pointer rises to the raised surface and the selected one to the active surface; a horizontal chart also stands the accent pillar in the row's own lead cell. Those lead cells are kept free from the first frame, so nothing moves when the pointer arrives: a chart is not a list, and it never slides.
- **The keyboard.** The arrow keys follow the axis the bars run along: ↑↓ or k and j in a horizontal chart, ←→ or h and l in a vertical one. Home and End jump to the ends.
- **Narrow areas.** Below 24 cells horizontal charts drop their labels and the series names before their values; vertical bars get thinner, down to one cell, and labels are cut. A group of standing bars too thin to give every series a column stacks them instead, so no series is dropped.
- **Short areas.** A vertical chart gives up the value row first and then the labels, keeping the bars.
- **ASCII mode.** Bars round to whole cells.

## Common mistakes

- **A different colour per bar.** Colour is for meaning; a rainbow of services makes the danger bar invisible. Series tones are the one exception, and they come from the theme.
- **Unsorted horizontal bars.** Ranked data reads best largest first.
- **A category that changes colour.** When this week shows three kinds of work and last week two, the n-th series is a different kind each week. Give every category its own `Series::tone(n)` and the same numbers to the `Legend` through `.tones(...)`.
- **Too many series.** Past four the tones start over; three or four shares are as much as a stack can say.
- **A stack where the parts matter more than the total.** Group them instead, or the reader is left comparing lengths from different starting points.
- **Too many bars.** Past a dozen, consider a table with a small bar per row.
