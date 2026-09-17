## When to use

Use a gauge for how full a resource is right now when there is a point where it becomes a problem: memory, disk, a connection pool, a rate limit. A progress bar says how far work has come; a gauge says how close a resource is to its limit.

## Step by step

1. Add a gauge with its value: `Gauge::new(81.0)`. The range is 0 to 100 by default.
2. Give it the real range when the unit is not percent: `.range(0.0, 8.0)` for 8 GiB.
3. Name it: `.label("Memory")`.
4. Set the limits: `.thresholds(6.0, 7.2)`. From the first value it turns to the warning tone, from the second to danger.
5. Show the value the way people think about it: `.value_text("6.4 of 8 GiB")`.
6. Stack several gauges with the same width and `.label_width(8)` so their meters line up.

## How it works

- **One row.** Label on the left, meter in the middle, value on the right. The meter fills in eighths of a cell.
- **Limits you can see.** With thresholds, the part of the track past each limit is tinted faintly with the tone waiting there, so you see how much room is left before anything is wrong.
- **Never colour alone.** With thresholds the value carries a marker: a dot when fine, `▲` for warning, `✕` for danger.
- **Narrow areas.** When fewer than four cells remain for the meter, it is left out and the label and value stay; the label is cut with `…` first.

## Common mistakes

- **Thresholds for things without limits.** A request counter has no danger zone; use a number or a sparkline.
- **Warning colours without meaning.** If 90% disk is normal for your system, move the thresholds instead of teaching people to ignore the warning colour.
- **Different widths.** Gauges of different widths are hard to compare; give a group the same width.
