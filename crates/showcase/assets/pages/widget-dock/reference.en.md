## Methods

- `WidgetDock::new(sections)` — titles in display order; add one body child per widget with `ui.add_with`.
- `.open(iter of bool)` — open state by display position.
- `.on_toggle(|position, open| msg)` — a widget was opened or closed.
- `.on_move(|from, to| msg)` — turns on reordering; remove at `from`, insert at `to`.
- `.empty_text(text)` — shown when there are no widgets.
- `Section::new(title)`, `.icon(key)`, `.detail(text)` — shared with `Accordion`.

## Keys

- While focused: `up` `down` `home` `end` move between titles, `enter` or `space` toggle, `ctrl+shift+up` `ctrl+shift+down` move the widget (with `on_move`).

## Mouse

- Click a title to toggle; drag a title to move the widget (with `on_move`).

## Behaviour

- Fills its area: titles and gaps first, then open bodies share the remaining rows; small bodies keep their natural height, the rest split evenly.
- Opening and closing animate within the area; reduced motion jumps.

## Theme keys

- Everything of `Accordion`: `section`, `section-title`, `section-chevron`, `section-detail`, `section-body`.
- `section-title.ghost` — the dragged title; `section-drop` — `bg` of the landing row; `section-empty` — `fg`.
