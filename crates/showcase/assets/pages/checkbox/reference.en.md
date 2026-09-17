## Methods

- `Checkbox::new(checked)` — a box showing `checked`.
- `.label(text)` — text two cells after the box; clicking it toggles.
- `.partial(bool)` — shows the box partly checked; toggling asks for `true`.
- `.style(CheckboxStyle)` — `Box` (default): two cells of colour. `Check`: three cells with a check or a dash.
- `.disabled(bool)` — cannot be focused or toggled.
- `.on_toggle(|on| msg)` — message with the new state.

## Behaviour

- Measures two cells (`Box`) or three (`Check`), plus two and the label when labelled; one row. A narrow space cuts the label with `…`.
- `Box` blends each cell from the empty colour (the states without `checked`) to the filled colour (with `checked`) over `motion.step` × 3; reduced motion switches at once. Partly checked fills the left cell.
- Clicking anywhere on the box, the gap or the label toggles.
- Enter, Space or a click released over the widget toggles it.
- Without `on_toggle` it is shown but cannot be focused.

## Theme keys

- `checkbox` — `bg`, `fg`, `bold`; states `hover`, `focus`, `checked`, `disabled`; variant `partial` (check style; the box style fills the left cell instead).
- `checkbox-label` — `fg`, `bold`; the same states.
- `[motion]` — `step` (the box blend).
- `[icons]` — `check`, `check-partial` (check style only).
