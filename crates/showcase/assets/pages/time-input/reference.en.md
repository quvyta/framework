## Methods

- `TimeOfDay::new(hour, minute, second)` — a time on a 24-hour clock; each part is capped at its largest value. The fields `hour`, `minute`, `second` are public, and times compare in clock order.
- `TimeInput::new(time)` — a field showing hours and minutes.
- `.seconds(bool)` — shows and edits seconds too.
- `.invalid(bool)` — marks the time as failing a rule of your own.
- `.disabled(bool)` — greys the field out; it cannot be focused or changed.
- `.on_change(|time| msg)` — message with the new time after every change. Without it the field is shown but cannot be focused.

## Behaviour

- Measures four cells per segment and one per colon: 9 cells, 14 with seconds.
- ← → change the active segment and stop at the ends; ↑ ↓ change it by one and wrap.
- Digits fill the active segment; a segment is complete after two digits or after a digit that cannot start a value in range, then the next segment becomes active. A second digit that would leave the range starts the segment again.
- `:` moves to the next segment, Backspace sets the active segment to zero.
- A click activates the segment under the pointer. When the field loses focus the first segment becomes active again.
- Written in 24-hour form in every language.
- The wheel changes the segment under the pointer by one per notch and wraps, without moving focus or the active segment. Over a colon it changes the segment the pointer was last over since it entered the field, or the active segment when there was none; leaving the field forgets it. A disabled field or one without `on_change` lets the wheel pass on, so the page scrolls.
- Ctrl+A selects the whole time (every segment takes `text-input-selection`); any other key or a click clears it. Ctrl+C copies `hh:mm` or `hh:mm:ss`; Ctrl+X copies and sets `00:00`.
- A pasted `h:mm` or `h:mm:ss` with every part in range sets the time; anything else is ignored and passed on.
- A right click, Shift+F10 or the menu key opens the edit menu; Cut and Copy need the whole time selected. Language keys `quvyta.edit.*`.

## Theme keys

- `time-input` — `bg`, `fg`; states `hover`, `focus`, `invalid`, `disabled`.
- `time-segment` — `bg`, `fg`, `bold`; `selected` for the active segment, `active` while a first digit waits for its second.
- `time-separator` — `fg`.
