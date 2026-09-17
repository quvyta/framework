## Methods

- `Tree::new(roots)` — top-level `TreeNode`s.
- `.selected(Option<&str>)` — the key of the selected node.
- `.on_select(|key| msg)`, `.on_activate(|key| msg)`, `.on_expand(|key, open| msg)`.
- `.empty_text(text)` — shown when there are no nodes.
- `TreeNode::new(key, label)`, `.children(nodes)`, `.expanded(bool)`, `.expandable(bool)`, `.loading(bool)`.
- `.icon(key, Some(token))`, `.detail(text)`, `.faint(bool)`.

## Behaviour

- Keys while focused: `up` `down` or `k` `j`, `pgup` `pgdn`, `home` `end` move; `right` or `l` opens or steps into the first child; `left` or `h` closes or steps to the parent; `enter` opens or closes a folder and activates a leaf; `space` activates.
- Mouse: a click on the chevron opens or closes; a click on a row selects it and then behaves like `enter`; the wheel and the scrollbar scroll.
- Each level is indented by two cells. Leaves keep the chevron's column empty so names of one level line up.
- Only the icon and label slide one cell right on a hovered or selected row; the indentation, the chevron (or the loading spinner) and the detail stay. The label keeps one spare cell, so it is cut at the same place resting and sliding.

## Theme keys

- `list-item` with `hover`, `selected`, `focus`, `pressed`; `list-item.faint`; `list-detail`; `list-header` for the empty text.
- `tree-chevron` — `fg`, with `hover` and `selected`.
- `spinner` — the loading chevron; `scrollbar` — `track`, `thumb`.
- Icons: `tree-collapsed`, `tree-expanded`, `spinner`.
