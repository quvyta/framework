## Methods

- `TextInput::new(value)` — a field showing `value`.
- `.on_change(|text| msg)` — the new value after every edit.
- `.on_submit(|text| msg)` — the value when Enter is pressed.
- `.on_cancel(msg)` — sent when Esc is pressed in the field, instead of letting the key go on; an open list of suggestions takes the first Esc.
- `.placeholder(text)` — faint text while empty.
- `.password(bool)` — masks characters with the `mask` icon. Default: `false`.
- `.max_length(n)` — at most `n` characters.
- `.invalid(bool)` — invalid look. Default: `false`.
- `.disabled(bool)` — read-only, not focusable. Default: `false`.
- `.select_on_focus(range)` — selects the characters in `range` each time the field gains focus, cursor at the range's end; cut to the text. Not set by default.
- `.select_all_on_focus()` — selects the whole text each time the field gains focus.
- `.suggestions(rows)` — the `Suggestion`s the field offers under itself, in the order they should be read. The list appears only together with `.on_suggestion`; a field with neither behaves as before. Not set by default.
- `.max_suggestions(n)` — how many of them the list holds at once, `8` by default; the first `n` are shown.
- `.on_suggestion(|index| msg)` — the place of the chosen row, counted from the first row passed to `.suggestions`.

## Suggestions

- `Suggestion::new(label)` — a row labelled `label`, the text the row shows.
- `.detail(text)` — a faint note on the right of the row, e.g. where a path points.
- `.icon(key)` — an icon key drawn before the label; labels line up after the widest icon in the list.
- `.label()` — the text of the row.

## Keys

- `left` `right`, with `ctrl` by word, with `shift` selecting; `home` `end`.
- `backspace` `delete`; `ctrl backspace` and `ctrl w` delete a word; `ctrl u` deletes to the start.
- `ctrl a` select all; `ctrl z` undo; `ctrl y` or `ctrl shift z` redo.
- `ctrl c` copy, `ctrl x` cut the selection (never in password fields); `ctrl v` and the terminal's paste insert.
- With a selection, `left` and `right` clear it and move one step past its left or right end; with `ctrl` a word from that end.
- `shift f10` or `menu` opens the edit menu under the field.
- `enter` submits; `tab` moves focus.
- With the suggestion list open, `up` `down` choose a row and `enter` takes it; `up` from the first row and `down` from the last leave the typed text standing, and `enter` then submits. Typing chooses nothing, and `esc` closes the list and keeps the text.

## Mouse

- Click places the cursor; drag selects. A click that gives focus to a field with `select_on_focus` places the cursor and selects nothing.
- Right click opens the edit menu at the pointer: Cut, Copy, Paste, Select all. Inside the selection it keeps it; elsewhere it places the cursor first. Cut and Copy need a selection, Paste text on the system clipboard, the terminal's or from a copy inside the application.
- A click on a row of the suggestion list chooses it. A press anywhere else closes the list and still reaches what it landed on.

## Theme keys

- `text-input` with `hover`, `focus`, `invalid`, `disabled` — `bg`, `fg`, `padding`.
- `text-input-prompt`, `text-input-placeholder`, `text-input-selection`, `text-input-cursor`.
- The suggestion list is drawn as a menu: `context-menu` for its surface, `context-item` with `hover` for a row and the `▌` pillar of the chosen one, `context-item-detail` for a note.

## Icons

- `prompt` before the text; `mask` for passwords; whatever a suggestion's `.icon(key)` names.

## Language keys

- `quvyta.edit.cut`, `quvyta.edit.copy`, `quvyta.edit.paste`, `quvyta.edit.select-all`.
