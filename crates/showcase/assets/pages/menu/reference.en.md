## Methods

- `MenuItem::new(key, label)` — a destination; `.icon(key, Option<colour>)`, `.badge(text)`.
- `MenuGroup::new(key, items)` — a group; `.title(text)` adds the faint heading.
- `Menu::new(groups)` — the menu.
- `.selected(Option<&str>)` — the key of the current item.
- `.on_select(|key| msg)` — sent when an item other than the current one is opened.
- `.collapsible(|group, open| msg)` — titled groups fold; `.collapsed(keys)` lists the closed ones.

## Behaviour

- Groups are separated by an empty row. A folded group keeps its heading.
- The keyboard cursor starts on the current item and is drawn like hover while the menu has focus. There is one such highlight: moving the pointer onto a row moves the cursor there.
- A foldable heading rises with the pillar and slides its title like an item; its chevron stays at the right.
- Type-ahead only looks at visible items and wraps around.
- Opening the current item flashes it and sends nothing.

## Keys

- `up` `down` move, `home` `end` jump, a letter jumps to the next item starting with it, `enter` or `space` opens.
- With `collapsible`: `enter` on a heading toggles, `right` opens, `left` closes, `left` on an item goes to its heading.

## Mouse

- Click an item to open it, click a heading to fold it, turn the wheel or drag the scrollbar to scroll.

## Theme keys

- `menu-item` — `bg`, `fg`, `bold`, `pillar`; states `hover`, `selected`, `focus`, `pressed`.
- `menu-badge` — `fg`; `selected`.
- `menu-heading` — `fg`; `hover` with `bg` and `pillar` in collapsible menus.
- `scrollbar`.

## Icons

- `pillar`, `chevron-down` and `chevron-right` on folding headings, plus the icons you name.
