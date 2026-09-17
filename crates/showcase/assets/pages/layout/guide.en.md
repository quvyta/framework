## Rows, columns and stacks

Every screen is a tree of containers. `ui.column` stacks children top to bottom, `ui.row` places them left to right, and `ui.stack` draws them on top of each other. Containers nest freely.

## Sizes

Each child has a width and a height, each one of three kinds:

- `Length::Auto` — as much as the widget measures. Text measures its words, a button its label and padding.
- `Length::Cells(n)` — exactly `n` columns or rows.
- `Length::Fill(weight)` — a share of what is left after `Auto` and `Cells` siblings took theirs. `Fill(2)` gets twice what `Fill(1)` gets.

`.fill()`, `.fill_width()` and `.fill_height()` are shorthands for `Fill(1)`.

## Spacing and alignment

- `.gap(n)` leaves `n` cells between children of a row or column.
- `.padding(Padding::symmetric(v, h))` keeps space free inside any node.
- `.justify(Align::Center)` places children along the main axis when there is room left; `.align(..)` places them across it.

Spacing comes from empty cells and surface colour, never from lines. Use a consistent rhythm: one row between groups, two or three columns between siblings.

## Surfaces instead of frames

Group related content in a `Panel`: a surface one step above its background. An inset panel inside a panel is one step higher again. The eye reads the grouping from tone alone.

## Pages and the router

`Router<P>` keeps the pages a user opened as a stack. `push` opens a page, `back` returns, `replace` swaps the current page without history. Draw the current page inside `ui.page(key, ...)`: every page keeps its focus and scroll position while it is hidden, so going back lands where the user left.

## The application shell

`AppShell` builds the usual layout: header, sidebar, body and footer, separated by surface colour. Below `collapse_below` columns the sidebar hides; with `sidebar_open(true)` it appears as a layer over the body. This showcase is built with it.

## Common mistakes

- **Fill inside an Auto parent.** A parent that sizes itself by its content has no room to share; give the parent a size.
- **Fixed widths everywhere.** Prefer `Fill` so screens adapt to the terminal.
- **Drawing separators.** Use a gap or a surface change instead of a line of characters.
