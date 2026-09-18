## Methods

- `BigText::new(text)` — big `text`.
- `.variant(name)` — theme variant, such as `"accent"` or `"dim"`.
- `.gradient(to, direction)` — blends the letters into theme colour `to`, running `Gradient::Columns` (left to right) or `Gradient::Rows` (top to bottom).

## Behaviour

- Measures the width of its glyphs plus one column between them, and three rows (five in ASCII mode).
- Supports `0`–`9`, `:`, `.`, `%`, `-`, `A`–`Z` (lowercase drawn as uppercase); other characters are drawn as a two-column space.
- When the area is smaller than the big form, draws the text at normal size in bold, cut with `…` if needed; the fallback keeps the flat colour.
- Without a gradient the letters take one colour, the style's `fg`. A gradient blends from that colour into the theme colour named, one step per cell of the direction it runs in.
- The gradient falls back to the flat colour at `ColorDepth::Ansi16` and when the theme does not know the colour named; `Ansi256` keeps it.
- Static: the blend never animates. Not focusable; sends no messages.

## Theme keys

- `big-text`, `big-text.<variant>` — `fg`.
