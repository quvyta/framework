## Methods

- `Select::new(options)` — options as text.
- `.selected(Option<usize>)` — the chosen option.
- `.on_select(|index| msg)` — sent when a different option is chosen.
- `.placeholder(text)` — shown while nothing is chosen.
- `.max_visible(rows)` — rows before the list scrolls. Default: 8.
- `.disabled(bool)` — not focusable, cannot open. Default: `false`.

## Keys

- Closed: `enter`, `space` or `down` opens.
- Open: `up` `down` move, `home` `end` jump, `pgup` `pgdn` page, a letter jumps, `enter` or `space` chooses, `esc` closes, `tab` closes and moves focus.

## Mouse

- Click the field to open or close; click an option to choose; the wheel scrolls the list and the scrollbar can be pressed and dragged.
- Moving the pointer over the options moves the one highlight.
- A click outside closes and still reaches what it landed on.

## Theme keys

- `select` with `hover`, `focus`, `active` (open), `disabled`.
- `select-placeholder`, `select-chevron`, `select-menu` (`bg`).
- `select-option` with `hover` and `checked`; `select-check`.

## Icons

- `chevron-down` on the field, `check` next to the chosen option.
