## When to use

Use a duration input for a length of time: a focus session, a break, a daily target, a timeout, how long until a reminder. It is built like the time input — hours and minutes, optionally seconds, each part typed or nudged on its own — but the value is a `std::time::Duration`, the unit words follow the language, and a length never wraps around.

## Step by step

1. Keep the length in your application: `session: Duration`.
2. Draw it: `DurationInput::new(state.session)`. The plain field shows hours and minutes: `0 h 50 min`, `0 sa 50 dk` in Turkish.
3. Handle changes: `.on_change(|length| Msg::Session(length))` and store the length in `update`.
4. Add `.seconds(true)` when seconds matter, such as a timeout.
5. Check rules that involve other fields in your application and show them with `.invalid(true)` and a message, such as a break longer than the session.
6. To say why a pasted text was not read, add `.on_reject(|error| Msg::Rejected(error))` and show `error.message(ui.env().i18n())` under the field.

## How it works

- **Segments with unit words.** The numbers sit on one surface; the unit words after them are faint and come from the language files, so the field reads `1 h 30 min` in English and `1 sa 30 dk` in Turkish.
- **Keyboard:** ← and → move between segments, ↑ and ↓ change the active one by one of its unit. Typing fills the active segment two digits at a time: `0130` gives 1 h 30 min. `:` or Space moves on after a single digit, Backspace sets the segment to zero.
- **A length carries instead of wrapping.** 0 h 59 min up is 1 h 00 min; down stops at zero and up at 99 h 59 min 59 s. Minutes typed past 59 carry too: `90` in the minutes reads 1 h 30 min.
- **The wheel changes the segment under the pointer**, one unit per notch, without a click and without moving focus. Over a unit word it changes the segment the pointer was last over; if it came straight onto the word, the active segment. This is exactly how the time input behaves.
- **Copying and pasting.** Ctrl+A selects the whole length; Ctrl+C copies it as written in the active language (`1 h 30 min`), Ctrl+X copies it and sets zero. A paste reads `90 min`, `90 dk`, `1 h 30 min`, `1 saat 30 dakika`, `1h30m`, `1,5 sa` or the clock face `2:15` — in any language the application knows, whatever language is active. A bare number is minutes. A right click opens the same actions as a menu.
- **Empty and narrow.** A zero length is the empty state: its digits rest as faint as the unit words until the pointer or focus arrives. Where the words do not fit, the field shows the clock form `1 : 30` with the same segments.

## Common mistakes

- **A time input for a length.** A time of day wraps at midnight and has no units; a length of 90 minutes is not 01:30 in the morning.
- **Writing unit words in code.** The words come from `quvyta.duration.*`; an application that adds a language adds its words there and the field reads and writes them.
- **Reading `2:15` as minutes and seconds.** The clock face is always hours and minutes; for minutes and seconds write `2 min 15 s`.
