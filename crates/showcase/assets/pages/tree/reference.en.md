## Methods

- `Tree::new(roots)` — top-level `TreeNode`s.
- `.selected(Option<&str>)` — the key of the selected node.
- `.on_select(|key| msg)`, `.on_activate(|key| msg)`, `.on_expand(|key, open| msg)`.
- `.empty_text(text)` — shown when there are no nodes.
- `.reorderable(|step| msg)` — nodes can be moved among their siblings; `step` is a `TreeMove` with `key`, `parent` (`None` at the top level), `from` and `to`. `step.apply(&mut siblings)` moves the item in your own list.
- `.context_menu(|key| items)` — the `ContextItem`s of the node with that key.
- `.multi_select(&selected, |keys| msg)` — several nodes can be selected; `selected` are their keys and `keys` is always the whole new selection. `.selected(..)` is the cursor.
- `.droppable(|drop| msg, |key| accepts)` — dragged nodes drop into the nodes `accepts` says yes to; `drop` is a `TreeDrop` with `keys` (in tree order, without nodes inside another moving node) and `into` (`None` for the top level).
- `.on_copy_drop(|drop| msg)` — a drop released with Ctrl held asks for a copy with this instead of the move; a terminal that does not report Ctrl with the pointer always moves.
- `.activate_on(Click::Single | Click::Double)` — `Single` by default: a click selects and does what Enter does. With `Double` a click only selects and a second press on the same row within `Click::INTERVAL` (400 ms) opens, closes or activates it; the chevron, ← and → still open and close with one click.
- `.box_select(bool)` — with `multi_select`, a drag from the free space below the rows draws a box in the `text-selection` tone and the rows it covers become the selection, or join it with Ctrl held at the press; a click there clears the selection.
- `TreeNode::new(key, label)`, `.children(nodes)`, `.expanded(bool)`, `.expandable(bool)`, `.loading(bool)`.
- `.icon(key, Some(token))`, `.detail(text)`, `.faint(bool)`.

## Behaviour

- Keys while focused: `up` `down` or `k` `j`, `pgup` `pgdn`, `home` `end` move; `right` or `l` opens or steps into the first child; `left` or `h` closes or steps to the parent; `enter` opens or closes a folder and activates a leaf; `space` activates.
- Layout: leaves keep the chevron column empty so labels of one level line up; a tree where no row opens has no chevron column at all, so its labels start where a `Menu` beside it starts.
- Mouse: a click on the chevron opens or closes; a click on a row selects it and then behaves like `enter`; the wheel and the scrollbar scroll.
- Reordering: a press on a row (not on its chevron) selects it; moving the pointer a row makes it a drag. The landing place is the resting sibling under the pointer, or the nearest one on screen; the siblings are shown in the order the drop would give, the landing place is tinted and a ghost row follows the pointer. The dragged node's children fold away during the drag. Released, the tree sends one `TreeMove`; released where it started, nothing. Without a drag the release opens, closes or activates the row like `enter`. `ctrl+shift+up` and `ctrl+shift+down` move the selected node one place and stop at the ends. A reorder never changes a node's parent.
- Held on the top or bottom row or past them, a drag scrolls one row after 400 ms, then every 150 ms, sooner the further past the edge.
- Multi-select: a plain click or arrow makes a row the only selected one. `ctrl` + click adds or removes a row and moves the cursor there; `shift` + click selects the rows from the last plain or `ctrl` click to this one. `shift+up`, `shift+down`, `shift+pgup`, `shift+pgdn`, `shift+home` and `shift+end` stretch that range; `ctrl+a` selects every row shown; `space` adds or removes the cursor's row instead of activating it; `esc` reduces several selected nodes to the cursor's and otherwise goes on to the parents. Every selected row takes the selection tone; only the cursor's row keeps the pillar and slides.
- Dropping: a press on a selected row keeps the selection and a drag carries all of it; a press elsewhere selects that row alone. The row under the pointer takes `tree-drop` when `accepts` says yes to it and it is not one of the dragged nodes, inside one of them or the node they all are in already; such a refused row is painted `list-item.faint`. A closed node the drag rests on for 400 ms opens. The free rows below the last row stand for the top level. Released on an accepted row, the tree sends one `TreeDrop`; anywhere else, nothing. A click without a drag reduces the selection to that row on release.
- With `reorderable` and `droppable` both on, a drag of one node over a row `accepts` says yes to drops it in, and over any other row lands among its siblings as above; a drag of several nodes only drops.
- Context menu: a right click on a row opens its node's menu at the pointer and keeps the row raised; `shift+f10` or the menu key opens the menu of the selected node below its row, scrolling it into view. Choosing an entry sends its message. A right click below the rows does nothing. With multi-select, a right click on a selected row keeps the selection; on another row it first selects that row alone.
- Each level is indented by two cells. Leaves keep the chevron's column empty so names of one level line up.
- Only the icon and label slide one cell right on a hovered or selected row; the indentation, the chevron (or the loading spinner) and the detail stay. The label keeps one spare cell, so it is cut at the same place resting and sliding.

## Theme keys

- `list-item` with `hover`, `selected`, `focus`, `pressed`; `list-item.faint`; `list-detail`; `list-header` for the empty text.
- `tree-chevron` — `fg`, with `hover` and `selected`.
- `spinner` — the loading chevron; `scrollbar` — `track`, `thumb`.
- `tab-drop` — the landing slot of a dragged node; `tab-ghost` — the row following the pointer. The menu uses the keys of `ContextItem`.
- `tree-drop` — `bg`, `fg`, `bold` of the node a drop goes into, and `bg` of the free rows when it goes to the top level.
- Icons: `tree-collapsed`, `tree-expanded`, `spinner`.
