## When to use

Use a table when every row has the same few facts that people compare: containers with their status, CPU and memory, builds with their duration, files with their size. For a single column of names use a list; for nested things use a tree.

## Step by step

1. Describe the columns: `Column::new(t!("name"))` fills the free room; `.width(ColumnWidth::Fit)` sizes a column to its widest cell; `.width(ColumnWidth::Fixed(9))` fixes it; `.min(12)` keeps a filling column readable.
2. Right-align numbers with `.align(Align::End)` so their digits line up.
3. Build the rows: `TableRow::new([name, status, cpu])`; a cell can carry a status icon, `TableCell::new(word).icon("dot", Some("success"))`, or a quiet glyph before a name, `TableCell::new(name).icon(Glyph::literal('\u{e76e}'), None)` for a glyph you looked up yourself or `Glyph::key("project")` for an icon of the set.
4. Keep the rows in your state as an `Arc<[TableRow]>` when there are many, and pass a clone: the table then never copies them.
5. Show and receive the selection with `.selected(..)`, `.on_select(..)` and `.on_activate(..)`.
6. For sorting, mark columns `.sortable(true)`, handle `.on_sort(|column, direction| ..)` by reordering your data, and show the result with `.sort(column, direction)`.
7. For multiple selection add `.checked(bools)` and `.on_toggle(..)`; say what an empty table means with `.empty_text(..)`.

## How it works

- **The header is a surface, not a line.** Titles sit faint on the raised tone; the sorted title brightens and carries an accent arrow.
- **Columns are separated by air.** Two empty cells between columns; nothing is drawn between them.
- **Only the first cell slides.** A hovered or selected row rises, shows the pillar and moves its first visible cell one cell right. The check mark of a multiple selection and the numbers and statuses to the right stay still, so the eye can compare them while moving and the mark is always where you click it.
- **Too many columns scroll sideways.** When the minimum widths do not fit, ← and → move whole columns. Arrows at the ends of the header say more columns are hidden on that side; they are buttons too: a click scrolls one column, and they light up under the pointer.
- **Any number of rows.** Only the rows on screen are painted; `Fit` widths are measured once per set of rows.
- **A glyph before a name is quiet.** Without a colour it is `muted`, a step below the name, and takes the name's colour on the selected row, so the eye reads the name first and meaning never rides on the glyph's colour. It is drawn as the glyph, one space, then the text, in every glyph mode. A narrow column cuts only the text with `…`; the glyph and its space always stay. The demo's names carry their program's Nerd Font glyph in Nerd mode and the set's project icon otherwise; the playground turns them off.
- **Your data, your order.** The table never reorders rows. It asks for a sort and shows the arrow you give it back.

## Common mistakes

- **Sorting inside `view`.** Sort in `update` when the sort message arrives; `view` runs every frame.
- **Rebuilding 100 000 rows every frame.** Build them when the data changes and keep the `Arc`.
- **Colour-only status.** Pair the coloured dot with a word, as the demo does.
- **Separating columns with a drawn bar.** Spacing and alignment already separate them.
