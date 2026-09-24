## Methods

- `ContextMenu::new(items)` — wraps the widgets added with `ui.add_with(…)`. Its messages must be `Clone`; the menus of `Tabs` and `TabRail` do not need that.
- `ContextItem::new(label, msg)` — an action that sends `msg` when chosen.
- `ContextItem::submenu(label, items)` — a row that opens `items` beside the menu.
- `ContextItem::gap()` — an empty row between groups.
- `.icon(key)` — icon before the label.
- `.shortcut(label)` — faint key label at the right edge (display only).
- `.detail(text)` — faint note on the right in the muted tone, before the shortcut or the submenu arrow, drawn on disabled rows too; e.g. why the row cannot be used. The menu widens to fit it; in a narrow space it is cut before the label, and left out below four cells.
- `.disabled(bool)` — greyed out, skipped by the keyboard. Default: `false`.
- `.danger(bool)` — drawn in the danger colour. Default: `false`.

## Keys

- Closed: `shift f10` or `menu` (the menu key, reported by terminals with the kitty keyboard protocol) opens below the focused widget in the area. A menu with no items does not open.
- Open: `up` `down` move, `home` `end` jump, a letter jumps, `right`, `enter` or `space` opens a submenu, `left` closes it, `enter` or `space` chooses, `esc` closes one level, `tab` closes and moves focus.

## Mouse

- Right click opens at the pointer. Hover highlights rows and opens submenus; a click chooses, a click on a gap or a disabled row does nothing. A press outside closes the menu and still reaches its target; a right click inside the area opens it again there.

## Theme keys

- `context-menu` — `bg` (default `$overlay`).
- `context-item` with `hover` (`bg`, `fg`, `pillar`) and `disabled`; `context-item.danger` with `hover`.
- `context-item-shortcut`, `context-item-chevron` and `context-item-detail` with the row's `hover` and `disabled`; the built-in theme draws all three in `$muted` and brightens only the chevron on `hover`.

## Icons

- `chevron-right` marks submenus; `pillar` marks the highlighted row.
