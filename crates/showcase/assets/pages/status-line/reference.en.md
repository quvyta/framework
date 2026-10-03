## Methods

- `StatusLine::new(text)` — a status line reading `text` in the neutral `Info` tone.
- `.tone(ToastKind)` — the status the sentence reports: `Success`, `Warning`, `Danger` or `Info`. It gives the line its sign and its colour.
- `.action(button)` — a Button at the end of the line; it keeps its own variant, shortcut and message, and sends that message when it is pressed.
- `ToastKind::ALL` and `ToastKind::name()` — every kind, and the short name of one, as a settings screen or a locale key writes it.

## Behaviour

- One row while the sentence and the button fit: sign, a space, the sentence, then the button after a gap of two cells.
- In a narrower area the sentence wraps under itself and keeps its column, and the button takes a row of its own. Nothing is ever cut.
- Measures the width of one row while it fits, and the width it is given otherwise, with a row for the sentence and another for the button below it.
- The sign and the sentence are text; only the button takes focus and input.
- Wraps where it is placed: `.width(Length::Cells(n))` for a narrow line, `.fill_width()` for the whole row.

## Theme keys

- `status-line` — `fg`, `bold`, with the variants `success`, `warning`, `danger` and `info`. The sentence takes the kind's colour when a theme names no variant; the sign always does.
- `success`, `warning`, `danger`, `info` — the colours both the sign and the sentence use.
- `[icons]` — `success`, `warning`, `error`, `info` for the signs, one cell each in every glyph mode.
