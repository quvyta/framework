## Methods

- `EmptyState::new(title)` — an empty state reading `title`.
- `.icon(key)` — an icon from the icon set, drawn muted above the title.
- `.message(text)` — the explanation under the title; wraps to at most 52 cells.
- `.action(button)` — a Button under the explanation; it keeps its own variant, shortcut and message.

## Behaviour

- Measures the full width it gets and the rows its parts need; paints centred in the area.
- When the area is short it drops, in order: the icon with its gap, the gap above the action, explanation lines, and finally the action.
- Lines that do not fit the width are cut with `…`.
- Only the action is focusable; the rest is text.

## Theme keys

- `empty-state-icon` — `fg`.
- `empty-state-title` — `fg`, `bold`.
- `empty-state-message` — `fg`.
- Icon `inbox` — a suitable default icon for empty collections.
