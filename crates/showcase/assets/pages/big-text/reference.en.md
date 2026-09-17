## Methods

- `BigText::new(text)` — big `text`.
- `.variant(name)` — theme variant, such as `"accent"` or `"dim"`.

## Behaviour

- Measures the width of its glyphs plus one column between them, and three rows (five in ASCII mode).
- Supports `0`–`9`, `:`, `.`, `%`, `-`, `A`–`Z` (lowercase drawn as uppercase); other characters are drawn as a two-column space.
- When the area is smaller than the big form, draws the text at normal size in bold, cut with `…` if needed.
- Not focusable; sends no messages.

## Theme keys

- `big-text`, `big-text.<variant>` — `fg`.
