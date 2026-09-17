## Methods

- `NumberInput::new(value)` — a field showing `value`, without limits, stepping by 1.
- `.range(min, max)` — the smallest and largest allowed value; a reversed range is put in order.
- `.step(step)` — how far ↑ and ↓ move; its decimals decide the writing and whether `.` can be typed. A step that is not positive becomes 1.
- `.steppers(bool)` — clickable minus and plus segments on the right.
- `.placeholder(text)` — faint text while the field is empty.
- `.invalid(bool)` — marks the value as failing a rule of your own.
- `.disabled(bool)` — read-only and unfocusable.
- `.on_change(|value| msg)` — message with every new valid number inside the range.

## Behaviour

- Measures the prompt, padding, the widest of the minimum, maximum and value (eight cells without a range) and, with steppers, six cells.
- Typing accepts digits, `-` when the minimum is below zero and `.` when the step has decimals; everything else edits like `TextInput`.
- Unparsable or out-of-range text is shown with the `invalid` state and not sent; an empty field is neither marked nor sent.
- ↑ ↓ one step, Page Up / Page Down ten steps, from the typed number when it is one, clamped to the range.
- A click on a stepper segment steps once and flashes the segment; at a limit the segment shows its `disabled` state.
- When the application's value changes, the text is rewritten from it with the step's decimals.
- The wheel over the field moves one step per notch, clamped to the range, and is not passed on to a scroll view around it.
- A right click, Shift+F10 or the menu key opens the edit menu (Cut, Copy, Paste, Select all); while it is open ↑ ↓ move in the menu, not the value.

## Theme keys

- `text-input`, `text-input-prompt`, `text-input-placeholder`, `text-input-selection`, `text-input-cursor` — the field, as for `TextInput`.
- `number-input-stepper` — `bg`, `fg`; states `hover`, `focus` (the field is focused), `pressed`, `disabled`.
- `[icons]` — `prompt`, `stepper-minus`, `stepper-plus`.
