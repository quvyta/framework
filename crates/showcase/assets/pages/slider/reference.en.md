## Methods

- `Slider::new(value)` — a slider from 0 to 100 in steps of 1 showing `value`.
- `.range(min, max)` — the smallest and largest value; a reversed range is put in order.
- `.step(step)` — the distance of one step; values snap to steps from the minimum. A step that is not positive becomes 1.
- `.suffix(text)` — text written right after the value, such as `"%"`.
- `.format(|value| text)` — writes the value yourself instead of with the step's decimals; the suffix still follows.
- `.disabled(bool)` — greys the slider out; it cannot be focused or moved.
- `.on_change(|value| msg)` — message with the new value whenever the knob reaches another step. Without it the slider is shown but cannot be focused.

## Behaviour

- Takes all the width it is given and one row. The value is right-aligned in room for the widest of the minimum, the maximum and the current value, followed by two cells and the rail.
- Narrower than the value plus four cells, only the value is drawn.
- ← → one step, Page Up / Page Down a tenth of the range (at least one step), Home / End the ends.
- Pressing the rail jumps to the value under the pointer and captures the pointer until release; dragging follows. Pressing the value only focuses.
- The wheel over the slider moves one step (up increases, down decreases), stops at the ends, and is kept by the slider so a surrounding scroll view does not scroll; it does not take focus. A disabled slider ignores it.
- Keyboard changes step the knob one cell every `motion.step`, at most eight steps; pointer and wheel changes and reduced motion move it at once.

## Theme keys

- `slider` — `fill` (the done part), `fill-cell` (the done part in ASCII mode), `track`, `knob`; states `hover`, `focus`, `disabled`.
- `slider-value` — `fg`, `bold`; the same states.
- `[motion]` — `step`, and `pulse-period` for the breathing knob.
- `[icons]` — `slider-rail`, `slider-knob`; a blank glyph draws the rail as cell colours.
