## Methods

- `TimeBlock::new(label, start, end)` — a block of the day; an end before the start runs past midnight, an end equal to the start is a moment.
- `TimeBlock::tone(index)` — the theme's `index`-th series tone instead of the accent.
- `TimeBlock::open_end()` — the block goes on after its end (into the next day, or still running): its last two cells fade towards the track and the readout says "continues next day" or "running".
- `TimeBlock::open_start()` — the block began before its start: its first two cells fade in from the track and the readout says "from the previous day" or "from earlier".
- `TimeBlock::faint()` — halfway between the block's tone and the track, for time shown in full elsewhere.
- `Timeline::new(blocks)` — a whole day from midnight, one row tall for consecutive blocks.
- `.day_starts_at(time)` — starts the day at `time`, so blocks across midnight stay whole.
- `.range(from, to)` — shows only that stretch; into the next day when `to` is not after `from`, the whole day when they are equal; kept inside the day.
- `.axis()` — a row of hours under the strip.
- `.readout()` — a row writing the block being read: name, times, length.
- `.selected(Some(index))` — the selected block.
- `.on_select(|index| msg)` — message for moving the selection; turns hover, clicks and the arrow keys on.
- `.on_zoom(|from, to| msg)` — message asking for a new range; turns `+`, `-`, `0` and the wheel on.
- `.disabled(disabled)` — muted, no focus, no messages.
- `Axis::hours(from, to)` — hours placed by time, the same arithmetic as a timeline.
- `Axis::weekdays(first, count)` — weekday names from the language files, wrapping after Sunday.
- `Axis::months(first, count)` — month names, 1 for January, wrapping after December.
- `Axis::labels(names)` — names of your own, one slot each.
- `.gap(cells)` — empty cells between slots, to match a chart's gap; hours ignore it.

## Behaviour

- Measures one row per lane, plus one for the axis and one for the readout; an empty day is one lane.
- A block shorter than a cell takes one cell. Touching blocks of one tone get a seam: the later one's first cell a step towards the track.
- Overlapping blocks take the first free lane from the top; a short area folds extra lanes into its last row, the selected block drawn on top.
- Short areas give up the axis, then the readout, then lanes. A narrow readout drops the length, then the times, then cuts the name with `…`.
- Focusable with `on_select` (and blocks to select) or `on_zoom`. Keys: ←/→ or h/l walk blocks in time order, Home/End jump; `+` or `=` zoom in, `-` out, `0` the whole day. The wheel zooms around the pointer only while focused. A click selects the block under it; a click on a gap selects nothing.
- Zoom steps: 24, 12, 6, 3 and 1 hours, anchored on the selected block (keys) or the pointer (wheel), on whole minutes, kept inside the day. Selecting a block outside a zoomed range asks for the range to move to it.
- Hover and selection change tones only; a timeline never slides.
- An open edge fades over up to two cells, two thirds then one third of the way to the track, always leaving one cell in the block's tone; a block open at both ends shares its cells. Only an edge inside the visible range fades: a zoom cut stays square. A name sits clear of the fading cells or is left out.
- The readout writes an open edge's meaning after the times; a narrow row drops the length, then the times, then the meaning, then cuts the name.
- A faint block, its fading cells and its hover and selection steps are always kept apart from the track on 256 and 16 colours; a fading cell that finds no colour between the tone and the track keeps the tone.
- On 256 and 16 colours a block tone that would merge with the track takes the text colour, and a hover or selection step that would not show is pushed further.
- An axis measures one row, takes no focus, and writes nothing where no label fits; labels keep a cell between them and are never cut.

## Theme keys

- `timeline` — `track` (the empty day), `fill` (a block without a tone), `hover`, `selected` (the tones a block steps towards).
- `timeline:focus` — `selected` while the keyboard is on the timeline.
- `timeline-readout` — `fg` (the name), `detail` (times and length).
- `axis` — `fg` (the labels).
- Colour tokens `series-1` to `series-5`, through `Theme::series_color(index)`.
