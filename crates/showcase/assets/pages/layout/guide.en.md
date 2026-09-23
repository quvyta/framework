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

## Rows that wrap

A row of buttons that fits in English can be too wide in German or Turkish, or on a narrow terminal. `.wrap(true)` on a row moves the children that do not fit to the next line instead of letting them run past the edge:

```rust
ui.row(|ui| {
    ui.add(Button::new(t!("install")).on_press(Msg::Install));
    ui.add(Button::new(t!("show-command")).on_press(Msg::ShowCommand));
    ui.add(Button::new(t!("cancel")).on_press(Msg::Cancel));
})
.gap(1)
.wrap(true);
```

- Lines are filled in order. Each child takes the width it measures; one that does not fit after the others starts the next line. A child wider than the whole row gets a line of its own and is narrowed to the row's width.
- A row whose children fit looks exactly as it does without `wrap`.
- Every line is a row of its own: `gap` falls between the children of a line, never before the first; `justify` places each line in the room it leaves; a `spacer` takes what is left on its own line.
- A spacer stays on the line of the child before it. When that line has no room even for the gap before it, the spacer is left out rather than pushing the next line away from the edge.
- The row is as tall as its lines, so what comes after it moves down. Lines touch; `.line_gap(1)` leaves an empty row between them.
- Tab and the mouse follow the buttons where they are drawn.

## Choosing the layout from the space

`ui.size()` is the room the application is drawing into: the whole terminal, in columns and rows. An application reads it in `view` and picks its arrangement, so "below 48 columns, fold the three columns" is one `if`:

- Wide: the columns side by side in a `row`.
- Narrow: the same content in a `column`, with what does not fit moved to a page of its own.

The number is the same in every nested builder — inside a `column`, a `Panel`, a `SidePanel` part or a `Modal` it is still the terminal — because the view is built before layout divides the screen. That keeps the decision simple: it is about the terminal the user has. Reading it is not I/O; after a resize the next frame sees the new size, and `Harness::resize` does the same in tests, so the narrow layout is tested like any other screen.

This is for the application's own arrangement. A widget that adapts inside its own rectangle, such as a list column shortening its labels, does that by itself.

## Surfaces instead of frames

Group related content in a `Panel`: a surface one step above its background. An inset panel inside a panel is one step higher again. The eye reads the grouping from tone alone.

## Pages and the router

`Router<P>` keeps the pages a user opened as a stack. `push` opens a page, `back` returns, `replace` swaps the current page without history. Draw the current page inside `ui.page(key, ...)`: every page keeps its focus and scroll position while it is hidden, so going back lands where the user left.

## The application shell

`AppShell` builds the usual layout: header, sidebar, body and footer, separated by surface colour. Below `collapse_below` columns the sidebar hides; with `sidebar_open(true)` it appears as a layer over the body. This showcase is built with it.

## Common mistakes

- **Fill inside an Auto parent.** A parent that sizes itself by its content has no room to share; give the parent a size.
- **Fixed widths everywhere.** Prefer `Fill` so screens adapt to the terminal.
- **Keeping the size in state.** `ui.size()` is always current in `view`; a copy kept in the application's state goes stale on the next resize.
- **Drawing separators.** Use a gap or a surface change instead of a line of characters.
