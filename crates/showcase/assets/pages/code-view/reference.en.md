## Methods

- `CodeView::new(code, language)` — `Language::Rust`, `Language::Toml` or `Language::Plain`.
- `.line_numbers(bool)` — Default: `true`.
- `.on_copy(msg)` — sent after copying with `c`.

## Keys

- While focused: `c` copies the whole code and flashes.
- Mouse: a drag inside the code selects it and never reaches the padding; the line number gutter is decoration, left out of clean copies.

## Language

- `Language::from_tag("rust" | "rs" | "toml" | other)` — for fenced code tags.

## Theme keys

- `code` with `focus` and `pressed` — `bg`, `padding`.
- `code-line-number` — `fg`.
- `code-token.<kind>` — `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `plain`.
