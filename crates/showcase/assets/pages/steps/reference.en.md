## Methods

- `Steps::new(labels)` — a row of steps, the first one current, not choosable.
- `.current(index)` — the current step; earlier steps are finished, a value past the end finishes all.
- `.vertical(bool)` — one step per line.
- `.running(bool)` — the current marker breathes.
- `.failed(bool)` — the current marker becomes the error icon in the danger colour.
- `.on_select(|index| msg)` — finished steps can be chosen; the message carries the step.

## Behaviour

- A step is its marker, two cells and its label; steps in a row are three cells apart.
- When the row does not fit, markers stay one cell apart and the current label follows them.
- Choosable steps get one cell of padding on each side so the hover surface has room; under the pointer or the keyboard the first of those cells shows the pillar.
- Keys while focused: ←/→ in a row, ↑/↓ in a column, Home and End move between finished steps; Enter or Space chooses. A click on a finished step chooses it; other steps ignore clicks.
- The steps take focus only while `on_select` is set and at least one step is finished.

## Theme keys

- `step` — `bg`, `pillar`; states `hover`, `focus` (choosable finished steps).
- `step-marker` — `fg`; states `checked` (finished), `active` (current); variants `running`, `failed`.
- `step-label` — `fg`, `bold`; the same states and variants, plus `hover` and `focus`.
- `[icons]` — `check`, `dot`, `dot-outline`, `error`.
