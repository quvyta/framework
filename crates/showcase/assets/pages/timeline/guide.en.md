## When to use

Use a timeline when the question is "when, during this day": the hours of focus, meetings, builds, sleep. It shows where things fell and where the gaps were, which a total cannot. For how much per day over weeks use a heatmap; for comparing totals use a bar chart.

Use an axis wherever a chart needs its edge named: hours under a strip, weekdays or months under standing bars. It never cuts a label and never lets two collide, so it can be given any width.

## Step by step

1. Keep the day's blocks in your state: `TimeBlock::new("Rust", start, end)` with two `TimeOfDay` values. A block whose end is before its start runs past midnight.
2. Pin each category to a tone of the palette: `.tone(0)` for Rust, `.tone(1)` for Docs, everywhere they appear, and name them with `Legend::new(names).tones([0, 1])`.
3. Draw the day: `ui.add(Timeline::new(blocks).axis())`. The strip grows a row only for blocks that run at the same time.
4. Let a reader read a block: `.readout()`, with `.selected(state.picked).on_select(Msg::Pick)` so the keyboard can walk the blocks too.
5. Let them zoom: keep a range in your state, pass it with `.range(from, to)`, and store the new one from `.on_zoom(|from, to| Msg::Zoom(from, to))`. When `from` equals `to` the whole day is back.
6. For a night, start the day in the evening: `.day_starts_at(TimeOfDay::new(18, 0, 0))`.
7. For a session split at midnight, mark where it goes on: `.open_end()` on its part in the first day, `.faint().open_start()` on its part in the next. A counter still running is `.open_end()` too.
8. For an axis of its own: `Axis::weekdays(Weekday::Monday, 7)`, `Axis::months(1, 12)`, `Axis::hours(from, to)` or `Axis::labels(names)`; under standing bars give it the chart's gap, `.gap(1)`.

## How it works

- **Placed by time.** A block covers the cells its hours fall in, and one shorter than a cell still takes a cell, so a five-minute review is seen on a whole-day strip. The hours of the axis are placed with the same arithmetic, so a label stands over the cell its time is drawn in.
- **Gaps are the track.** Nothing recorded is the empty track tone. Two blocks of one tone that touch are parted by a quieter first cell on the later one: a seam of tone, never a line.
- **Overlaps take lanes.** Each block takes the first lane from the top where nothing overlaps it. A block that ends where another starts shares its lane. An area too short for every lane folds the rest into its last row and draws the selected block on top.
- **Midnight.** The day is 24 hours from `day_starts_at`, midnight by default. In a midnight day a block from 23:00 to 01:30 is cut at the end; its rest belongs to tomorrow's strip, from 00:00. A day that starts at 18:00 holds it whole.
- **Open edges.** `open_end` and `open_start` mark an edge where the block does not really stop. The last or first two cells fade towards the track, two thirds and one third of the way, so the block runs out into the day instead of stopping square; there is no glyph, and never the `▌` pillar, which means focus. The fading cells are always kept apart from the track, so a block never looks shorter than it is. A block too short for a fade keeps a cell of its own tone, and a block open at both ends shares its cells between the two. The readout writes what the edge means: "continues next day" for an end at the day's end, "running" for one before it, "from the previous day" for a start at the day's start, "from earlier" for one after it.
- **Only the block's own edge is open.** Where a zoomed range cuts a block the edge stays square: the range is where the view stops, the axis already says so, and a fade there would claim the block goes on when it may not. The readout still gives the whole block's times.
- **Faint blocks.** `faint` puts a block halfway between its tone and the track: still its category's colour, plainly not a gap. Hover and selection still step it, and never onto the track.
- **Zoom is yours.** `+` and `-` step through 24, 12, 6, 3 and 1 hours around the selected block, `0` goes back to the whole day, and the wheel zooms around the pointer once the timeline holds the focus, so scrolling a page past it never gets caught. Selecting a block outside the range moves the range to it.
- **Mouse and keyboard are equal.** The block under the pointer and the selected block step towards the text colour; ← and → (or h and l) walk the blocks in time order, Home and End jump to the ends, a click selects. The readout writes the block under the pointer, else the selected one, as its name, its times and its length: both read the same words.
- **Nothing moves.** Hover and selection change tones only. A timeline is a narrow strip, not a list, so it never slides.
- **Short and narrow.** A short area gives up the axis first, then the readout, then lanes. A narrow readout drops the length, then the times, and only then cuts the name.
- **Axis thinning.** Weekdays and months write every name in full while all fit, then every name short, then every second, third, … short name. Hours take the finest round step that fits in the long form `09:30`, and go to the short `09` only when the long one would leave fewer than two labels. A label that would run past the edge is left out.
- **Every state.** An empty day is the whole track and the readout says so; a disabled timeline goes muted and answers nothing. The strip is made of colour, so every glyph mode draws it the same, and on a 16-colour terminal a tone that would merge with the track or with its own hover step is pushed until it differs.

## Common mistakes

- **Colours by position.** Categories that change colour from day to day cannot be learned. Pin them with `.tone(n)` and give the legend the same numbers.
- **Splitting a night by hand.** Start the day in the evening instead, and sleep stays one block.
- **A continuation that looks new.** The next day's part of a session is the same work; draw it `.faint().open_start()` so it does not read as a fresh start.
- **An open end on a zoom cut.** Do not add `open_end` because the range ends there; only the block's real end is open.
- **A readout left out.** A tone is not a time; without the readout or text of your own nobody can say when a block started.
- **Zooming without a range.** `on_zoom` only asks. Store the range it sends and pass it back with `.range()`.
