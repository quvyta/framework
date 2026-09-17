## When to use

Use a time input for a time of day: when a maintenance window starts, when a nightly backup runs, a timeout written as hours, minutes and seconds. Every part can be typed or nudged on its own, and the value is always a valid time.

## Step by step

1. Keep the time in your application: `starts: TimeOfDay`.
2. Draw it: `TimeInput::new(state.starts)`. The plain field shows hours and minutes.
3. Handle changes: `.on_change(|time| Msg::Starts(time))` and store the time in `update`.
4. Add `.seconds(true)` when seconds matter.
5. Check rules that involve other fields in your application and show them with `.invalid(true)` and a message, such as an end before its start.

## How it works

- **Segments on one surface.** Hours, minutes and seconds are two-digit segments; the colons between them are faint. While the field has focus one segment is active and lifted with a touch of the accent.
- **Always 24 hours.** The field writes `14:30` in every language. It does not follow a locale's 12-hour clock; if your users expect AM and PM, say so next to the field.
- **Keyboard:** ← and → move between segments, ↑ and ↓ change the active one and wrap around (23 becomes 00, 00 becomes 59). Typing fills the active segment: `0930` gives 09:30. A digit that cannot start a two-digit value, such as 7 in the hours, completes the segment at once. `:` moves on, Backspace sets the segment to zero.
- **Mouse:** a click activates the segment under the pointer.
- **The value is always valid.** There is no half-typed state to validate; every change sends a complete `TimeOfDay`.
- **The wheel changes the segment under the pointer**, one per notch, wrapping like ↑ ↓. There is no need to click it first, and the active segment and focus stay where they are. Over a colon the wheel changes the segment the pointer was last over; if it came straight onto the colon, the active segment.
- **Copying a time.** Ctrl+A selects the whole time; Ctrl+C copies it as `09:30` (`09:30:15` with seconds) and Ctrl+X copies it and sets it to zero. Pasting `9:30` or `09:30:15` sets the time; other text is ignored. A right click opens the same actions as a menu.

## Common mistakes

- **Time inputs for durations longer than a day.** Hours stop at 23; use a number input with a unit.
- **Validating inside the field.** A time is always valid on its own; relations between times belong to your application.
- **Twelve-hour labels next to the field.** The field is always 24-hour; write your hints the same way.
