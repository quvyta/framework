## List

- `List::new(items)` — a list of `ListItem`s.
- `List::shared(items)` — items kept in your state as an `Arc<[ListItem]>`; only the `Arc` is cloned each frame, so a long list is not rebuilt.
- `.selected(Option<usize>)` — the selected row.
- `.on_select(|index| msg)` — the selection moved.
- `.on_activate(|index| msg)` — a row was opened with Enter or a click.
- `.checked(Vec<bool>)` and `.on_toggle(|index| msg)` — multiple selection.
- `.empty_text(text)` — shown when there are no items.
- `.scrollbar(ScrollbarStyle)` — pins a scrollbar style instead of the theme's.

## ListItem

- `ListItem::new(label)`, `ListItem::header(title)`, `ListItem::gap()`.
- `.icon(key, Some(token))` — icon before the label, optionally coloured.
- `.detail(text)` — right-aligned faint text.
- `.faint(bool)` — faint but selectable.

## Keys

- `up` `down` or `k` `j` move; `home` `end` jump; `pgup` `pgdn` page; `enter` activates; `space` toggles in multiple selection, activates otherwise.

## Mouse

- Click selects and activates; a click on the check mark or the cell after it only toggles; the wheel scrolls; drag the scrollbar.

## Row anatomy

- Pillar, then fixed marks (the check mark), then the sliding part (icon and label), then the fixed detail at the right.
- With `motion.slide` on, a hovered or selected row moves only the sliding part one cell right. The label keeps one spare cell, so it is cut at the same place resting and sliding.

## Theme keys

- `list-item` with `hover`, `selected`, `focus`, `pressed` — `bg`, `fg`, `bold`, `pillar`.
- `list-item.faint`, `list-header`, `list-detail`, `scrollbar` (`style`, `track`, `thumb`) and `scrollbar.<style>`.
- `[icons]` — `select-on`, `select-off`: the multi-select marks (a checked and an empty box).
