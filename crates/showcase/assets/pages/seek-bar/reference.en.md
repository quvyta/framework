## Methods

- `SeekBar::new(fraction)` — a bar at `fraction`, clamped to 0..1; a value that is not a number rests at the start.
- `.percent(show)` — shows or hides the percentage after the bar; shown by default, as on a progress bar.
- `.variant(name)` — theme variant such as `"success"`, exactly as on a progress bar.
- `.on_seek(|fraction| msg)` — message with the fraction a press, a drag or an arrow key moved to. Without it the bar is drawn but takes no focus, no hover and no pointer.
- `.hover_label(|fraction| text)` — writes the label shown above the pointer, such as the time of a track.

## Behaviour

- Measures the full width it gets and one row, and paints the same cells and tones as `ProgressBar::new(fraction)` does.
- The percentage takes five cells on the right, unless `.percent(false)` hides it; the bar uses the rest, and a press or a hover only counts on those cells.
- The fill steps in eighths of a cell, whole cells in ASCII mode, rounded to the nearest cell.
- A press seeks the centre of the cell it landed on: the fourth cell of a twenty-cell bar is `4.5 / 20`.
- A drag captures the pointer until the release and seeks every cell the pointer moves to; past either end, or off the bar's row, the fraction holds at 0 or 1.
- ← → move a twentieth of the track, Home / End its ends. At an end, where there is nowhere to move, the bar sends nothing.
- Under the pointer the bar steps one tone lighter and the cell the pointer is on takes the accent.
- Without `on_seek` the bar is not hoverable and is not reached by Tab.
- A hover label is drawn as a tooltip of the library over the pointer's own cell and appears at once, without the hover delay.

## Theme keys

- `seek-bar`, `seek-bar.<variant>` — `track`, `fill`; states `hover` and `focus`.
- `progress-label`, `progress-label.<variant>` — `fg`, `bold`; the percentage is the one a progress bar writes.
- `[motion]` — `enter` and `pulse-period` for the label fading in and the fill breathing while focused.