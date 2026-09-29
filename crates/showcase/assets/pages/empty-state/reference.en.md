## Methods

- `EmptyState::new(title)` — an empty state reading `title`.
- `.icon(key)` — an icon from the icon set, drawn above the title when no status tone is set.
- `.message(text)` — the explanation under the title; wraps to at most 52 cells.
- `.action(button)` — adds a Button under the explanation. Call it more than once for several equal choices; the first one is primary and keeps the reading and focus order.
- `.tone(ToastKind)` — gives the icon and title the `success`, `warning`, `danger` or `info` theme colour and uses the matching status sign. Without a tone the existing icon and title colours stay as they are.

## Behaviour

- Measures the full width it gets and the rows its parts need; paints centred in the area.
- Several action buttons share a centred row with the same two-cell gap as dialog actions. When they do not fit, they stand one under another, each centred, in the order they were added.
- When the area is short it drops, in order: the icon with its gap, the gap above the actions, explanation lines, and finally the actions. The title and actions stay as long as there is room.
- Lines that do not fit the width are cut with `…`.
- Every action is focusable. Tab and the arrow keys visit the buttons in reading order, and each button keeps its own variant, shortcut and message.
- A tone uses the same status colour and sign as `Toast`; the sign remains a visible cell in sixteen colours and ASCII icon mode.

## Theme keys

- `empty-state-icon` — `fg`.
- `empty-state-title` — `fg`, `bold`.
- `empty-state-message` — `fg`.
- `success`, `warning`, `danger` and `info` — the status colours used by `.tone(..)`.
- `inbox` — a suitable default icon for empty collections when no tone supplies the sign.
