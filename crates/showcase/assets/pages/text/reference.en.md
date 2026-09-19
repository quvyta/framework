## Text

- `Text::new(text)` — body text.
- `Text::rich([spans])` — text made of spans.
- `.role(name)` — typography role: `title`, `body` (default), `secondary`, `faint`.
- `.color(token)` — colour token for every span without its own colour.
- `.bold()` — every span bold.
- `.no_wrap()` — one line per source line, cut with `…`. Default: wraps.
- `.align(Align)` — `Start` (default), `Center`, `End`.
- Not selectable with the mouse; `NodeMut::selectable(true)` on its node makes it a selection region.

## Span

- `Span::new(text)`, `.role(name)`, `.color(token)`, `.on(token)` for background, `.bold()`.

## Measuring

- Width is the widest line; height is the number of wrapped lines. With `Length::Fill` or no width the text wraps at the width the layout gives.
- Wrapping breaks at whitespace; a run without spaces (a word with its punctuation, a version like `2026.9.1`) moves to the next line whole, or is split between graphemes when it is wider than the line.
- Chinese and Japanese break between ideographs and kana; closing marks (`。` `、` `，` `！` `）` `」` `ー`, small kana and their fullwidth forms) never start a line and opening brackets (`「` `（` `【`) never end one. Korean keeps its words whole.
- No-break spaces (U+00A0, U+202F, U+2007) never break a line.
- `qframe::text::{width, grapheme_width, truncate, truncate_middle, wrap, wrap_ranges}` measure and cut text for custom widgets.
- `truncate_middle(text, max) -> Cow<str>` — at most `max` cells, the middle replaced by `…`; unchanged when it fits, the end gets the odd cell, `…` alone at 1 and nothing at 0.

## Theme keys

- `[typography]` roles with `fg`, `bold`, `italic`, `underline`, `dim`.
