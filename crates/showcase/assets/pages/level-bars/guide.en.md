## When to use

Use level bars to show how loud something is right now: a music player's spectrum, the levels of a
mix, the load of each channel. It answers "which band is loud, and how loud?" at a glance. For a
value over time use a sparkline, for values compared by name use a bar chart, and for one number
with a limit use a gauge.

## Step by step

1. Compute the levels yourself, one value per column between zero and one; the widget draws
   columns and nothing else, so a value can come from a frequency band, an audio channel or a queue
   depth.
2. Draw them: `ui.add(LevelBars::new(levels)).width(Length::Fill(1)).height(Length::Cells(8))`.
3. Give the node the height the precision you want: each row is eight levels, so eight rows show
   sixty-four steps of a band.
4. To hold the high mark a band reached, keep that level beside it and pass it:
   `.peaks(peaks)`. A cap that is not above its column draws nothing.
5. Turn the shape on when it helps: `.bar_width(2)` for a fatter bar, `.gap(0)` for a band graph
   without air, `.mirror(true)` to grow the columns up and down from the middle row,
   `.gradient(true)` to let a column deepen from the theme's base tone to the accent.
6. Write the numbers you want read somewhere else: a level out of a column cannot be read out of a
   block.

## How it works

- **Eighth-cell columns.** Each value is a column of `▁▂▃▄▅▆▇█` from the bottom of the area, so
  three rows already give twenty-four levels.
- **The whole spectrum, however narrow.** Values that do not fit are merged by averaging their
  neighbours, so a forty-column area shows two hundred bands rather than the first forty of them.
  Room to spare instead makes the columns wider, and the one cell that cannot be shared out goes to
  the outermost columns, so the two ends of the row match.
- **Caps, not lines.** A peak is the block that reaches its level, or the thin `▔` line where the
  level is the top of a cell. A cap is drawn in the theme's peak tone, so it is read as a mark and
  never as more of the column.
- **One accent.** Columns take a tone a step below the accent, as a sparkline's do, so a cap and
  the top of a gradient stand out in the accent itself. A gradient steps once per cell and ends at
  the accent.
- **Nothing is a value.** A level below zero is clamped to it and a level that is not a number is
  silence, so a band that has not been measured draws nothing instead of a full column.
- **Sixteen colours and ASCII.** Where only the sixteen standard colours exist, the gradient is
  left out and every column is one tone, because a blend there would round to a palette entry per
  cell and speckle. Where there are no block glyphs, whole cells take the tone of the level and
  the cell a column ends in takes the share of it that fills it, so even a quiet band tints a cell.

## Common mistakes

- **A cap that means nothing.** `.peaks` needs the level a band actually reached. Passing the
  current level again draws a cap on every column at once, which is the same as no caps.
- **One row and a fine scale.** A single row is eight levels; a spectrum that moves a little will
  look frozen. Give it four rows or more.
- **Levels that are not levels.** Percentages from 0 to 100 are not levels: they are clamped to 1
  and every band looks the same. Divide by the full scale first.
- **Reading a number off a column.** A block says a level, not a value. Write the dB figure beside
  the row if someone has to read it.
- **Motion without a clock.** The widget never moves a value on its own; the application sends new
  levels, and a visualiser that keeps running with reduced motion turned on should stop sending.
