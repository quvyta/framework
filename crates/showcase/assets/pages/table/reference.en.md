## Methods

- `Table::new(columns, rows)` — `rows` is a `Vec<TableRow>` or an `Arc<[TableRow]>`.
- `.selected(Option<usize>)`, `.on_select(|index| msg)`, `.on_activate(|index| msg)`.
- `.checked(Vec<bool>)` and `.on_toggle(|index| msg)` — multiple selection.
- `.sort(column, SortDirection)` — shows the sort arrow; the rows must already be in that order.
- `.on_sort(|column, direction| msg)` — turns on sorting by title clicks and keys.
- `.empty_text(text)` — shown under the header when there are no rows.
- `.context_menu(|index| Vec<ContextItem<Msg>>)` — gives every row a menu of its own, built for the row it opens on.
- `.menu_on_activate(bool)` — Enter and a click open the row's menu instead of sending `on_activate`; an empty menu opens nothing. Off by default.
- `.activate_on(Click::Single | Click::Double)` — `Single` by default: a click selects and activates. With `Double` a click only selects and a second press on the same row within `Click::INTERVAL` (400 ms) activates it; Enter activates either way.
- `.multi_select(&selected, |rows| msg)` — several rows are selected at once: Ctrl+click adds or removes a row, Shift+click selects a range, `shift+up` `shift+down` `shift+pgup` `shift+pgdn` `shift+home` `shift+end` stretch it, `ctrl+a` selects every row, Space toggles the cursor's row and `esc` reduces several selected rows to the cursor's; `rows` is always the whole new selection and `.selected(..)` is the cursor. Selected rows share the selection tone; only the cursor's row has the pillar.
- `.box_select(bool)` — with `multi_select`, a drag from the free space below the rows draws a box in the `text-selection` tone and the rows it covers become the selection, or join it with Ctrl held at the press; a click there clears the selection.
- `.droppable(|drop| msg, |index| accepts)` — the pressed row, or the selection it is part of, is dragged onto a row `accepts` says yes to, which takes the `tree-drop` tone; `drop` is a `RowDrop { rows, into }`. A release anywhere else does nothing. With `Click::Single` a row then activates on release, so a press that becomes a drag activates nothing.
- `.on_copy_drop(|drop| msg)` — a drop released with Ctrl held asks for a copy with this instead of the move.
- `Column::new(title)`, `.width(ColumnWidth::Fixed(n) | Fit | Fill(weight))`, `.min(cells)`, `.align(Align)`, `.sortable(bool)`.
- `TableRow::new(cells)`, `.faint(bool)`; `TableCell::new(text)`, `.icon(glyph, color)`, `.color(token)`; strings convert into cells. `glyph` is an icon key (`"dot"`, `Glyph::key(..)`) or `Glyph::literal(..)`, drawn as the glyph, a space and the text; with `Some(token)` it is drawn in that colour, with `None` it is `muted` and takes the row's text colour when the row is selected. Truncation cuts the text only.
- `SortDirection::Ascending | Descending`, `.reversed()`.

## Behaviour

- Keys while focused: `up` `down` or `k` `j`, `pgup` `pgdn`, `home` `end` move; `enter` activates; `space` toggles in multiple selection, activates otherwise; `left` `right` scroll overflowing columns; `s` sorts by the next sortable column, `shift s` reverses.
- With a row menu: a right click on a row opens that row's menu at the pointer and makes the row the selection unless it is checked; the menu key or `shift f10` opens the menu of the selected row below it, scrolling it into view first. The row the menu belongs to stays raised while it is open. A press beside the menu closes it and still reaches what it landed on.
- Mouse: a click selects and activates a row; a click on the check mark or the cell after it only toggles; a click on a sortable title sorts, again reverses; a click on a header arrow scrolls the columns one step; the wheel and the scrollbar scroll.
- Fixed and fitting widths come first, filling columns share what is left by weight. When the minimums do not fit, columns keep their minimums and scroll sideways.
- The first visible cell keeps one spare cell for the selection slide and cuts long text with `…`. The check mark never slides.

## Theme keys

- `list-item` with `hover`, `selected`, `focus`, `pressed`, and `list-item.faint` — rows, shared with List.
- `table-header` — `bg`, `fg`; `hover` over a sortable title, `selected` on the sorted title.
- `table-sort` — `fg` of the sort arrow.
- `table-scroll` — `fg`, `bg` of the header arrows; `hover`.
- `list-header` — the empty text; `scrollbar` — `track`, `thumb`.
- `[icons]` — `select-on`, `select-off`: the multi-select marks (a checked and an empty box).
