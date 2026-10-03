## Methods

- `TextArea::new(text)` — an area showing `text`.
- `.placeholder(text)` — faint text while the area is empty.
- `.max_length(n)` — at most `n` characters; a line break counts as one.
- `.counter(bool)` — the character count on a row below the text, as `count / limit` with a limit.
- `.line_numbers(bool)` — a faint number before the first row of every line.
- `.language(language)` — `Language::Rust`, `Language::Toml`, `Language::Shell` or the default `Language::Plain`. Each part of the text takes the `code-token.<kind>` colour of its token: `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `variable`, `plain`. `Language::from_file_name(name)` picks the language of the file being edited.
- `.variant("plain")` — paper: no field surface, the tone of whatever the area sits on in every state; hover and focus show as the pillar, invalid text as a danger pillar at rest. Cursor and selection are unchanged.
- `.invalid(bool)` — marks the text as failing validation.
- `.disabled(bool)` — read-only and unfocusable.
- `.on_change(|text| msg)` — message with the new text after every edit.
- `.on_submit(|text| msg)` — message with the text on Ctrl+Enter.

## Behaviour

- Takes all the width it is given; measures its wrapped rows clamped to 3..8, plus the counter row and padding. A node height overrides it.
- Rows wrap at words at the text width minus one cell, which the cursor uses after a full row; spaces at a wrap hang at the row's end.
- When the rows do not fit, the right column shows the scrollbar and the text wraps one cell narrower.
- Enter inserts a line break; Ctrl+Enter submits when `on_submit` is set, otherwise it is not used.
- ↑ ↓ and Page Up / Page Down keep the column in a goal that resets on any other key; past the first or last row they go to the start or end of the text.
- Home / End: row start and end (End stops before a wrapped row's trailing space); with Ctrl the text's.
- Editing keys as in `TextInput`; Ctrl+U deletes to the start of the line. Pasted `\r\n` becomes `\n`, a tab four spaces.
- Every key and paste scrolls the cursor into view; the wheel scrolls three rows; pressing or dragging the scrollbar scrolls; pressing text places the cursor and dragging selects.
- Without Shift, ← → with a selection clear it and move from its left or right end; ↑ ↓ and Page Up / Page Down move from its upper or lower end.
- A right click, Shift+F10 or the menu key opens the edit menu of `TextInput` (Cut, Copy, Paste, Select all); a right click inside the selection keeps it, elsewhere it places the cursor first. Language keys `quvyta.edit.*`.
- A language changes the colour and nothing else: a selection keeps its own colours, the cursor block looks as before, the placeholder is uncoloured, and a disabled area keeps its muted text. The tokens are coloured once when the text or the language changes, not on every frame.

## Theme keys

- `text-area` — `bg`, `fg`, `padding`, `pillar`, and the `see-through` flag, which leaves the ground unpainted; states `hover`, `focus`, `invalid`, `disabled`. The `plain` variant sets `see-through` and a `$danger` pillar for `invalid`.
- `text-area-line-number` — `fg`; `selected` for the cursor's line.
- `text-area-counter` — `fg`.
- `code-token.<kind>` — the colours of the text with a language; the same keys the code view uses.
- `text-input-placeholder`, `text-input-selection`, `text-input-cursor` — shared with `TextInput`.
- `scrollbar` — `track`, `thumb`; `[icons]` `scroll-thumb`, `scroll-track`.
- `[motion]` — `cursor-blink`.
