## Methods

- `TextInput::new(value)` — a field showing `value`.
- `.on_change(|text| msg)` — the new value after every edit.
- `.on_submit(|text| msg)` — the value when Enter is pressed.
- `.placeholder(text)` — faint text while empty.
- `.password(bool)` — masks characters with the `mask` icon. Default: `false`.
- `.max_length(n)` — at most `n` characters.
- `.invalid(bool)` — invalid look. Default: `false`.
- `.disabled(bool)` — read-only, not focusable. Default: `false`.

## Keys

- `left` `right`, with `ctrl` by word, with `shift` selecting; `home` `end`.
- `backspace` `delete`; `ctrl backspace` and `ctrl w` delete a word; `ctrl u` deletes to the start.
- `ctrl a` select all; `ctrl z` undo; `ctrl y` or `ctrl shift z` redo.
- `ctrl c` copy, `ctrl x` cut the selection (never in password fields); `ctrl v` and the terminal's paste insert.
- With a selection, `left` and `right` clear it and move one step past its left or right end; with `ctrl` a word from that end.
- `shift f10` or `menu` opens the edit menu under the field.
- `enter` submits; `tab` moves focus.

## Mouse

- Click places the cursor; drag selects.
- Right click opens the edit menu at the pointer: Cut, Copy, Paste, Select all. Inside the selection it keeps it; elsewhere it places the cursor first. Cut and Copy need a selection, Paste text on the system clipboard, the terminal's or from a copy inside the application.

## Theme keys

- `text-input` with `hover`, `focus`, `invalid`, `disabled` — `bg`, `fg`, `padding`.
- `text-input-prompt`, `text-input-placeholder`, `text-input-selection`, `text-input-cursor`.

## Icons

- `prompt` before the text; `mask` for passwords.

## Language keys

- `quvyta.edit.cut`, `quvyta.edit.copy`, `quvyta.edit.paste`, `quvyta.edit.select-all`.
