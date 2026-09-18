## Methods

- `DurationInput::new(length)` — a field showing a `Duration` as hours and minutes. Parts of a second are not shown and are dropped by the first change.
- `.seconds(bool)` — shows and edits seconds too.
- `.invalid(bool)` — marks the length as failing a rule of your own.
- `.disabled(bool)` — greys the field out; it cannot be focused or changed.
- `.on_change(|length| msg)` — message with the new length after every change. Without it the field is shown but cannot be focused.
- `.on_reject(|error| msg)` — message with the `DurationError` of a paste that could not be read. Without it such a paste is ignored and passed on, as in the time input.
- `parse_duration(text, i18n)` — reads a written length the way a paste does: `Ok(Duration)` or the `DurationError` of the first part that could not be read.
- `DurationError::message(i18n)` — the reason in the active language. The variants are `Empty`, `Character`, `UnknownUnit`, `BadNumber`, `MissingNumber`, `MissingUnit`, `RepeatedUnit(DurationUnit)`, `BadClock` and `TooLarge`.
- `DurationUnit` — `Hours`, `Minutes`, `Seconds`.

## Reading

- Numbers with units, in any order, each unit once: `1 h 30 min`, `1sa30dk`, `2 hours`. Case does not matter; `İ`, `I` and `ı` read alike.
- The unit words are `quvyta.duration.hour-words`, `minute-words` and `second-words` of every language, the active one first.
- A number after a unit with no unit of its own takes the next smaller one: `1 h 30` is 1 h 30 min. A number alone is minutes.
- A decimal point or comma splits a unit, rounded to the second: `1.5 h`, `1,5 sa`.
- A clock face `h:mm` or `h:mm:ss` is hours and minutes; minutes and seconds are one or two digits under 60.
- Minutes and seconds past 59 carry over; the longest length is 99 h 59 min 59 s.

## Behaviour

- Measures a cell of room, two digits and a cell of room per segment, plus the unit words: 12 cells in English and Turkish, 17 with seconds. Hours handed in beyond 99 widen their segment rather than show a wrong number.
- Narrower than that, the field shows the clock form `hh : mm` with the same segments; narrower still, it is cut at the edge.
- ← → change the active segment and stop at the ends; ↑ ↓ change the length by one unit of the active segment, carrying between units, stopping at zero and at the longest length.
- Digits fill the active segment two at a time, then the next segment becomes active. `:` and Space move on, Backspace sets the active segment to zero.
- A click activates the segment under the pointer; a unit word belongs to the segment before it. When the field loses focus the first segment becomes active again.
- The wheel changes the segment under the pointer by one unit per notch, without moving focus or the active segment. Over a unit word or colon it changes the segment the pointer was last over since it entered the field, or the active segment when there was none. A disabled field or one without `on_change` lets the wheel pass on, so the page scrolls.
- Ctrl+A selects the whole length; Ctrl+C copies it as written in the active language, leaving out parts that are zero (`1 h 30 min`, `45 min`, `0 min`); Ctrl+X copies and sets zero.
- A paste that reads sets the length; without seconds on screen the pasted seconds are dropped and the length keeps its own.
- A right click, Shift+F10 or the menu key opens the edit menu; Cut and Copy need the whole length selected.
- A zero length rests with its digits in `time-separator`, the empty state, until the field is hovered or focused.

## Theme keys

Shared with the time input:

- `time-input` — `bg`, `fg`; states `hover`, `focus`, `invalid`, `disabled`.
- `time-segment` — `bg`, `fg`, `bold`; `selected` for the active segment, `active` while a first digit waits for its second.
- `time-separator` — `fg` of the unit words, the colons and a zero length at rest.

## Language keys

- `quvyta.duration.hours`, `minutes`, `seconds` — the words shown after the numbers.
- `quvyta.duration.hour-words`, `minute-words`, `second-words` — every word read as a unit, comma separated. They must not clash across languages.
- `quvyta.duration.hour-name`, `minute-name`, `second-name` and `empty`, `character`, `unit`, `number`, `no-number`, `no-unit`, `repeated`, `clock`, `too-large` — the reasons.
