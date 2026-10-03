## Methods

- `KeyHints::new()` — an empty bar.
- `.hint(key, label)` — a hint on the left with the key written as given.
- `.action(scope, name)` — a keymap action on the left; keys from the keymap, label from `quvyta.keys.<name>` or `keys.<name>`.
- `.action_right(scope, name)` — a keymap action on the right.
- `.action_first(scope, name)` — a keymap action before every `.hint`; the last of the left group to drop.
- `.action_labelled(scope, name, label)` — a keymap action on the left with the application's own label; the key follows the keymap, and an unbound action draws nothing.
- `.action_labelled_first(scope, name, label)` — `.action_labelled` in `.action_first`'s place: read first and the last to drop, for the key that says what the screen's main action does now.
- `.faint(bool)` — the whole bar a step quieter, for a screen that has gone still.
- `Keymap::label_for(scope, name)` — the first chord of an action written for a sentence, or `None` when it has none.

## Behaviour

- Measures the full width it gets and one row.
- Drops left hints from the end until they fit; right hints stay.
- On the left, `.action_first` actions come first, then every `.hint`, then every `.action`, whatever the call order: plain actions drop first and first actions last.
- An action shows its first key only; actions without a bound key are left out.

## Theme keys

- `key-hints` — `bg`, `padding`.
- `key-hint-key` — `bg`, `fg`, `bold`.
- `key-hint-label` — `fg`.
- `key-hint-key.faint`, `key-hint-label.faint` — the same for a faint bar.
