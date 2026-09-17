## Methods

- `KeyHints::new()` — an empty bar.
- `.hint(key, label)` — a hint on the left with the key written as given.
- `.action(scope, name)` — a keymap action on the left; keys from the keymap, label from `quvyta.keys.<name>` or `keys.<name>`.
- `.action_right(scope, name)` — a keymap action on the right.

## Behaviour

- Measures the full width it gets and one row.
- Drops left hints from the end until they fit; right hints stay.
- On the left, every `.hint` comes before every `.action`, whatever the call order, so actions are dropped first.
- An action shows its first key only; actions without a bound key are left out.

## Theme keys

- `key-hints` — `bg`, `padding`.
- `key-hint-key` — `bg`, `fg`, `bold`.
- `key-hint-label` — `fg`.
