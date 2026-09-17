## Methods

- `RailTab::new(name)` — a tab; `.icon(key)`, `.badge(text)`, `.status(token)` add an icon, faint text at the right and a status dot.
- `TabRail::new(tabs)` — the rail with the first tab open.
- `.active(index)` — the open tab.
- `.on_select(|index| msg)` — sent when another tab is opened.
- `.collapsed(bool)` — a thin strip: the pillar, a one-cell icon (or the name's first letter; a status colours it) that never slides, a cell of air and a scrollbar column; names on hover. Default: `false`.
- `.collapsed_marker(marker)` — what the collapsed strip shows for tabs without their own marker: `CollapsedMarker::Icon` (the icon, or the initial without one), `Initial` or `Number` (the position from 1; `…` past 9). Default: `Icon`.
- `RailTab::marker(marker)` — the same choice for one tab; it wins over the rail's.
- `.on_add(|| msg)` — ends the rail with an add (`+`) row that sends `msg` when clicked.
- `.closable(|index| msg)`, `.pinned(indices)`, `.reorderable(|from, to| msg)` — the same options as `Tabs`; apply the messages with `TabEdit::apply`.
- `.on_drag_scroll(|first| msg)` — sent for each row a dragged tab scrolls the rail; `first` is the first row now in view.
- `.row_height(lines)` — every row is a block this many lines tall (at least one, no upper limit). Default: `1`.
- `.gap(lines)` — any number of empty lines between rows. Default: one line when `row_height` is above 1, none otherwise; `gap(0)` stacks the blocks.
- `.context_menu(|index| items)` — a `Vec<ContextItem>` for tab `index`, opened by a right click on the tab or by the menu key; the chosen entry's message is sent. Same as on `Tabs`.

## Behaviour

- One row per tab, or one block per tab with `row_height` above one. Trailing marks are anchored to the right edge: status dot, badge, close mark. `row_height(1)` with no gap draws exactly the plain rail.
- With a `row_height` above 1 tabs are separated by one empty row unless `gap` says otherwise; `gap(0)` stacks the blocks.
- A block taller than a line is raised at rest (`rail-tab.tall`), its content is on the middle line (the upper middle line of an even block), its close mark flush right on its first line (in a two-line block that is the middle line) and its pillar covers every line. Presses, hover, drags and right clicks count anywhere on the block; lines of a gap belong to no tab.
- Scrolling, the scrollbar and following the open tab count whole blocks. A rail shorter than one block cuts every block to its own height.
- The add row, the collapsed strip and the collapsed name card have the height of a block; the strip's icon and the card's text and close mark sit on its middle line.
- In a collapsed rail the icon stays in the column right after the pillar whether the tab rests, is hovered or is open; only the pillar appears. The expanded rail keeps the one-cell slide of its icon and name.
- In a rail too narrow for its trailing marks, a mark that would reach the pillar is left out, and the close mark needs a cell beside the pillar.
- A right click on a tab or its name card opens that tab's menu at the pointer and opens no tab; while the menu is open it hides the name card, a right click on another tab moves the menu there, a left click beside it closes it and goes on to what it landed on, and Esc closes it. The menu key and shift+F10 open the open tab's menu below its row with the first entry highlighted. If the menu's tab goes away, the menu closes. Without `context_menu` a right click and the menu key do nothing.
- A collapsed rail measures 4 columns and always keeps its last column for the scrollbar, so it never changes width when tabs overflow; its name card is drawn beside the hovered row (or the open row while the rail has keyboard focus), stays while the pointer is on the card, opens the tab on a click and ends in the close mark when the tab can close.
- The add row is not a tab: it is never selected, closed or moved.
- When the tabs do not fit, the rail follows the open tab when it changes, scrolls with the wheel and draws a scrollbar in its last column.
- A drag starts after the pointer moves one line; the target is the resting block under the pointer.
- When rows overflow, a dragged tab held on the first or last visible block (with the lines after the last one) or past the rail's top or bottom scrolls the rail by one block: the first step after 400 ms, then one every 150 ms, 30 ms sooner for every line past the edge down to 60 ms. The scrollbar is lit while it can scroll. Leaving the end stops at once and the next arrival waits the full 400 ms again; the rail stops at its ends. A rail showing a single block scrolls only past its edges.

## Keys

- `up` `down` (or `k` `j`) open the neighbour, `home` `end` the first and last, `ctrl w` closes, `ctrl shift up` / `down` move the open tab, `menu` or `shift f10` open the context menu of the open tab.

## Mouse

- Click a tab to open it, click `×` or middle click to close, drag to reorder (hold on an end row to scroll), right click for its menu, wheel or click and drag the scrollbar to scroll. In a collapsed rail the name card takes the same clicks.

## Theme keys

- `rail-tab` — `bg`, `fg`, `bold`, `pillar`; states `hover`, `selected`, `focus`; variant `ghost` for the dragged tab; variant `tall` for blocks taller than a line (with `hover` and `selected`).
- `rail-badge` — `fg`; `selected`.
- `rail-hint` — `bg`, `fg` of the name card beside a collapsed rail; `hover` under the pointer.
- `rail-add` — `bg`, `fg`, `pillar` of the add row; `hover`; variant `tall`.
- `close-mark`, `tab-ghost`, `tab-drop` — shared with `Tabs`; `scrollbar`.
- `quvyta.tab-rail.add` — the label of the add row and of its name card.
- `context-menu`, `context-item` (`hover`, `disabled`, `danger`), `context-item-shortcut`, `context-item-chevron` — the context menu.

## Icons

- `pillar`, `close`, `dot` for the status, `add` for the add row, plus the icons you name on the tabs.
