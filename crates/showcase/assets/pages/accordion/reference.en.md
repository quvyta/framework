## Methods

- `Accordion::new(sections)` — titles as `&str`, `String` or `Section`; add one body child per section with `ui.add_with`.
- `.open(iter of bool)` — which sections are open; missing entries are closed.
- `.on_toggle(|index, open| msg)` — a section was opened or closed.
- `.single(bool)` — at most one open section.
- `Section::new(title)`, `.icon(key)`, `.detail(text)` — the title row.

## Keys

- While focused: `up` `down` move between titles, `home` `end` jump, `enter` or `space` open and close.

## Mouse

- A click released on a title toggles it.

## Behaviour

- An open body takes its natural height plus `section-body` padding.
- Opening reveals the body row by row over twice `motion.enter`; closing is immediate.
- The detail is dropped when the row is too narrow; titles truncate with `…`.
- With `motion.slide` on, a hovered or focused title slides only its icon and title one cell right; the chevron and the detail never move.

## Theme keys

- `section` — `gap` rows between sections.
- `section-title` — `bg`, `fg`, `bold`, `pillar`; states `hover`, `focus`, `checked` (open).
- `section-chevron`, `section-detail` — `fg`; the same states.
- `section-body` — `bg`, `padding`.
- Icons — `section-open`, `section-closed`.
