## Methods

- `CodeView::new(code, language)` — `Language::Rust`, `Language::Toml`, `Language::Shell` or `Language::Plain`.
- `.line_numbers(bool)` — Default: `true`.
- `.on_copy(msg)` — sent after copying with `c`.
- `.line_marks(marks)` — one `LineMark` per line from line 1: `Added` (success tint, `+`), `Removed` (danger tint, `−`), `Unchanged`. Default: none. The line numbers then follow the files rather than the text: a removed line carries the old file's number, an added line the new file's, a line in both the new file's.
- `.highlight_lines(range, tone)` — lines counted from 1, any range such as `4..=6` or `9..`; `LineTone::Accent` (pillar) or `LineTone::Warning` (warning icon). Wins over a mark; the later call wins where ranges meet.
- `.reveal(line)` — the enclosing `ScrollView` shows the line with two rows of context when the line changes; glides, or jumps with reduced motion. Past the end: the last line.
- `.reveal_number(number)` — the same, by the number a line is drawn with rather than its place in the text. Where a removed and an added line share a number, the line the new file numbers that way is reached; a number no line carries scrolls nowhere. Given as well as `.reveal(...)`, this wins.
- `.line_numbers_from(numbers)` — one `Option<usize>` per line from line 1; `None` leaves that line's column blank, and lines past the last number are blank. Wins over the numbers `.line_marks(...)` implies. Default: none.

## Keys

- While focused: `c` copies the whole code and flashes.
- Mouse: a drag inside the code selects it and never reaches the padding; the line number gutter is decoration, left out of clean copies.

## Language

- `Language::from_tag("rust" | "rs" | "toml" | "sh" | "bash" | "shell" | "zsh" | "pkgbuild" | other)` — for fenced code tags.
- `Language::from_file_name(name)` — `PKGBUILD`, `.sh`, `.bash`, `.zsh`, `.install` are shell; `.rs` Rust; `.toml` TOML; others plain.
- `TextArea::language(language)` — the same languages colour a text area while it is edited, in the same `code-token` styles; the code view stays the viewer for whole files.

## Highlighting your own text

- `qframe::text::highlight(code, language)` — the byte ranges of the code's tokens, in order, covering the whole text: the first range starts at zero, each one starts where the last ended, the last ends at the text's length. `Language::Plain` gives the whole text as one `Token::Plain`.
- `Token::style_variant()` — the variant of the `code-token` style that paints a token: `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `variable`, `plain`. An application that draws code in a widget of its own asks the theme for `code-token.<variant>` and paints each range with it, which is exactly what this widget does.
- `Token` is `#[non_exhaustive]`: a new kind of token may be added, so match with a wildcard rather than listing every kind.
- `Language` is the same type as `qframe::widgets::Language`, re-exported at `qframe::text::Language`.

## Theme keys

- `code` with `focus` and `pressed` — `bg`, `padding`.
- `code-line-number` — `fg`.
- `code-token.<kind>` — `keyword`, `type`, `function`, `macro`, `string`, `number`, `comment`, `attribute`, `lifetime`, `punctuation`, `table`, `key`, `variable`, `plain`.
- `code-line.<look>` — `bg` for the row, `fg` for its sign; look is `added`, `removed`, `accent` or `warning`.

## Icons

- `line-added`, `line-removed` — diff signs; `warning` and the pillar for highlights.
