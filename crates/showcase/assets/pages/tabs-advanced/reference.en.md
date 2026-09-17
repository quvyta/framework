## Methods

- `.closable(|index| msg)` — close marks, middle click and ctrl+w; the message asks to close tab `index`.
- `.pinned(indices)` — tabs that cannot be closed and show no mark.
- `.tab_width(TabWidth)` — `Fit` (default), `Fixed(cells)` or `Fill`.
- `.overflow(Overflow)` — `Arrows` (default) or `Menu`.
- `.reorderable(|from, to| msg)` — drag and keyboard reordering; `to` is the tab's index after the move.
- `.on_drag_scroll(|first| msg)` — sent for each tab a dragged tab scrolls the strip; `first` is the position of the first tab now in view.
- `.context_menu(|index| items)` — the `Vec<ContextItem>` of tab `index`; a right click on the tab or the menu key opens it and the chosen entry's message is sent.
- `TabEdit::Close(index)`, `TabEdit::Move { from, to }` — `.apply(&mut tabs, &mut active)` edits your list and keeps the open tab.
- Plain methods stay: `Tabs::new`, `.active`, `.numbered`, `.on_select`.

## Behaviour

- Closing the open tab with `apply` opens the tab that takes its place, or the new last tab.
- `Fixed(n)` never goes below one cell of label, where a long label shows only `…`; `Fill` stops shrinking at 12 cells or the label's own width.
- With `Arrows` the strip follows the open tab only when it changes, so the arrows can look around freely. An arrow with nothing more to show sinks to the surface and ignores presses. After tabs close, the strip scrolls back as soon as the remaining tabs fit.
- A strip too narrow for its controls cuts the open tab short; a menu strip with no room for any tab lists every tab and checks the open one.
- With `Menu` the menu lists hidden tabs only; ↑ ↓ Home End move, typing a letter jumps, Enter chooses, Esc or a click elsewhere closes.
- A drag starts after the pointer moves two cells. The drop target is the resting tab under the pointer, so the preview never jitters.
- With `Arrows` and hidden tabs, a dragged tab held on an arrow, or past either end of the strip, scrolls it: the first step after 400 ms, then one every 150 ms, 30 ms sooner for every cell past the strip's edge down to 60 ms. The arrow shows its hover tone and flashes on each step; leaving it stops at once, and the next arrival waits the full 400 ms again. At the end the arrow sinks and stepping stops. Dropped on an arrow the tab lands on the nearest tab in view. A `Menu` strip does not scroll this way: open the hidden tab first.
- With `context_menu`, a right click on a tab (its close mark included) opens that tab's menu at the pointer and opens no tab; a right click on another tab moves it, a left click beside it closes it and goes on to what it landed on, Esc closes it. The menu key and shift+F10 open the open tab's menu below it with the first entry highlighted. Opening one of the two menus closes the other. If the menu's tab goes away, the menu closes. Without the option a right click, the menu key and shift+F10 do nothing.

## Keys

- `ctrl w` closes, `ctrl shift left` / `right` move the open tab, `ctrl pgup` / `pgdn` scroll while tabs are hidden, `down` opens the menu of hidden tabs, `menu` or `shift f10` the context menu of the open tab.

## Mouse

- Click `×` or middle click a tab to close; drag a tab to move it, holding it on an arrow to reach hidden tabs; right click a tab for its menu; click the arrows or turn the wheel to scroll; click the count to list hidden tabs. With the menu open a click on a tab closes the menu and opens the tab, a click on the count only closes the menu.

## Theme keys

- `close-mark` — the three cells of the close mark: `fg`, `bg`, `bold`; `active` on a hovered or open tab, `hover` under the pointer (all three cells light up).
- `close-mark` with `active` and `hover` together — the lit mark on a raised tab, one step above the open tab's tone.
- `tab-arrow` — `bg`, `fg`, `pillar`; `hover`, `pressed`, `disabled`.
- `tab-menu` — `bg`, `fg`, `pillar`; `hover`, `active` while open.
- `tab-ghost` — `bg`, `fg`, `bold` of the dragged tab; `tab-drop` — `bg` of the slot it lands in.
- `popup-menu`, `popup-item` (`hover`, `checked`), `popup-check` — the menu of hidden tabs.
- `context-menu`, `context-item` (`hover`, `disabled`, `danger`), `context-item-shortcut`, `context-item-chevron` — the context menu.

## Icons

- `close` for the mark, `chevron-left` / `chevron-right` for arrows, `chevron-down` for the menu control, `check` for the open tab when the menu lists every tab.
