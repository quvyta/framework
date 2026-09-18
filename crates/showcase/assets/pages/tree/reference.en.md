## Methods

- `Tree::new(roots)` — top-level `TreeNode`s.
- `.selected(Option<&str>)` — the key of the selected node.
- `.on_select(|key| msg)`, `.on_activate(|key| msg)`, `.on_expand(|key, open| msg)`.
- `.empty_text(text)` — shown when there are no nodes.
- `.reorderable(|step| msg)` — nodes can be moved among their siblings; `step` is a `TreeMove` with `key`, `parent` (`None` at the top level), `from` and `to`. `step.apply(&mut siblings)` moves the item in your own list.
- `.context_menu(|key| items)` — the `ContextItem`s of the node with that key.
- `TreeNode::new(key, label)`, `.children(nodes)`, `.expanded(bool)`, `.expandable(bool)`, `.loading(bool)`.
- `.icon(key, Some(token))`, `.detail(text)`, `.faint(bool)`.

## Behaviour

- Keys while focused: `up` `down` or `k` `j`, `pgup` `pgdn`, `home` `end` move; `right` or `l` opens or steps into the first child; `left` or `h` closes or steps to the parent; `enter` opens or closes a folder and activates a leaf; `space` activates.
- Mouse: a click on the chevron opens or closes; a click on a row selects it and then behaves like `enter`; the wheel and the scrollbar scroll.
- Reordering: a press on a row (not on its chevron) selects it; moving the pointer a row makes it a drag. The landing place is the resting sibling under the pointer, or the nearest one on screen; the siblings are shown in the order the drop would give, the landing place is tinted and a ghost row follows the pointer. The dragged node's children fold away during the drag. Released, the tree sends one `TreeMove`; released where it started, nothing. Without a drag the release opens, closes or activates the row like `enter`. `ctrl+shift+up` and `ctrl+shift+down` move the selected node one place and stop at the ends. A node never changes parent.
- Held on the top or bottom row or past them, a drag scrolls one row after 400 ms, then every 150 ms, sooner the further past the edge.
- Context menu: a right click on a row opens its node's menu at the pointer and keeps the row raised; `shift+f10` or the menu key opens the menu of the selected node below its row, scrolling it into view. Choosing an entry sends its message. A right click below the rows does nothing.
- Each level is indented by two cells. Leaves keep the chevron's column empty so names of one level line up.
- Only the icon and label slide one cell right on a hovered or selected row; the indentation, the chevron (or the loading spinner) and the detail stay. The label keeps one spare cell, so it is cut at the same place resting and sliding.

## Theme keys

- `list-item` with `hover`, `selected`, `focus`, `pressed`; `list-item.faint`; `list-detail`; `list-header` for the empty text.
- `tree-chevron` — `fg`, with `hover` and `selected`.
- `spinner` — the loading chevron; `scrollbar` — `track`, `thumb`.
- `tab-drop` — the landing slot of a dragged node; `tab-ghost` — the row following the pointer. The menu uses the keys of `ContextItem`.
- Icons: `tree-collapsed`, `tree-expanded`, `spinner`.
