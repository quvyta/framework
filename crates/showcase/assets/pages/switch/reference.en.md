## Methods

- `Switch::new(on)` — a capsule switch showing `on`.
- `.label(text)` — text two cells after the switch; clicking it toggles.
- `.style(SwitchStyle)` — `Capsule` (default), `Rail` or `Labeled`.
- `.disabled(bool)` — cannot be focused or toggled.
- `.on_toggle(|on| msg)` — message with the new state.

## Behaviour

- Capsule and rail are five cells wide; `Labeled` is the longer word plus six.
- The knob travels three cells, one per `motion.step`; colours blend with each cell.
- Enter, Space or a click released over the widget toggles it.

## Theme keys

- `switch` — `track`, `track-on`, `knob`, `knob-on`; states `hover`, `focus`, `checked`, `disabled`.
- `switch-labeled` — `bg`, `fg`, `bold`, `dot`; the same states.
- `switch-label` — `fg`; the same states.
- `[motion]` — `step`.
- `[icons]` — `switch-rail`, `switch-knob`, `cap-left`, `cap-right`, `dot`.
- Language — `quvyta.switch.on`, `quvyta.switch.off`.
