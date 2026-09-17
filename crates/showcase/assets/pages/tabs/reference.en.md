## Methods

- `Tabs::new(labels)` — one tab per label.
- `.active(index)` — the open tab. Default: 0.
- `.numbered(bool)` — numbers before labels and number keys. Default: `false`.
- `.on_select(|index| msg)` — sent when another tab is opened.

## Keys

- While focused: `left` `right` or `h` `l` open the neighbour; `1`–`9` open a numbered tab; `ctrl pgup` / `ctrl pgdn` scroll an overflowing strip by one tab.

## Mouse

- Click a tab to open it.
- Click an arrow, or turn the wheel over an overflowing strip, to scroll one tab.

## Theme keys

- `tab` with `hover`, `selected`, `focus` — `bg`, `fg`, `bold`, `pillar`.
- `tab-index` with the same states.
- `tab-arrow` — `bg`, `fg`, `pillar`; `hover`, `pressed`, `disabled` for an arrow with nothing more to show.

## Icons

- `chevron-left` and `chevron-right` for the arrows; `pillar` for the hovered and open tabs and the hovered arrow.
