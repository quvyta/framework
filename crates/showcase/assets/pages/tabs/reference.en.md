## Methods

- `Tabs::new(labels)` — one tab per label.
- `.active(index)` — the open tab. Default: 0.
- `.numbered(bool)` — numbers before labels and number keys. Default: `false`.
- `.on_select(|index| msg)` — sent when another tab is opened.
- `.badge(index, count)` — a count one space after the name of tab `index`; above 99 it reads `99+`, and zero shows nothing and takes no room. An index past the last tab is ignored. Default: no counts.
- `.busy(index, bool)` — a thin spinner before the name of tab `index`, turning on the framework's clock; one dot standing still with reduced motion. It takes no room. An index past the last tab is ignored. Default: not busy.
- `.status(index, token)` — a dot before the name of tab `index` in the theme colour `token`, such as `success`, `warning` or `danger`; a busy tab turns in it instead of the accent. Default: no status.

## Keys

- While focused: `left` `right` or `h` `l` open the neighbour; `1`–`9` open a numbered tab; `ctrl pgup` / `ctrl pgdn` scroll an overflowing strip by one tab.

## Mouse

- Click a tab to open it.
- Click an arrow, or turn the wheel over an overflowing strip, to scroll one tab.

## Theme keys

- `tab` with `hover`, `selected`, `focus` — `bg`, `fg`, `bold`, `pillar`.
- `tab-index` with the same states.
- `tab-badge` with the same states — `fg`, `bold` of a count.
- The `accent` colour, or the colour a status names, for the busy mark and the status dot.
- `tab-arrow` — `bg`, `fg`, `pillar`; `hover`, `pressed`, `disabled` for an arrow with nothing more to show.

## Layout

- A tab: two cells of padding (the second holds the busy or status mark, and the name then moves one cell on into the spare cell), the number and a space when numbered, the name, a space and the count when it has one, the close mark when closable, a spare cell for the slide and two cells of padding.
- A tab too narrow for everything cuts its name with `…` first; the count and the close mark keep their cells. The count does not slide with the name.
- With reduced motion a busy tab's mark is the `dot` icon.
- In the menu of hidden tabs a count follows its name, two spaces after it.

## Icons

- `chevron-left` and `chevron-right` for the arrows; `pillar` for the hovered and open tabs and the hovered arrow.
