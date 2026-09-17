## When to use

Use a list for a column of items the user moves through and opens: containers, files, menu entries, search results. It stays fast at any length because it only draws the rows it shows.

## Step by step

1. Build the rows: `ListItem::new(name)`, with `.icon("dot", Some("success"))` for a status mark and `.detail(status)` for a quiet right column.
2. Separate sections with `ListItem::header(title)` and `ListItem::gap()`; they are never selected.
3. Show the selection you keep in your state: `.selected(self.selected)`.
4. Receive movement with `.on_select(Msg::Select)` and opening with `.on_activate(Msg::Open)`.
5. For multiple selection add `.checked(vec_of_bools)` and `.on_toggle(Msg::Toggle)`.
6. Say what an empty list means: `.empty_text(t!("containers.none"))`.

## How it works

- **Touched rows rise.** Hover raises a row's surface with a soft pillar; the selected row rises further. The pillar breathes only while the list has focus, so the eye always knows where the keyboard is.
- **Only the icon and the label slide.** A hovered or selected row moves its icon and label one cell right. The pillar, the check mark of a multiple selection, the detail column and the scrollbar stay exactly where they were, so a mark is always where you click it. The label always keeps one spare cell, so it is cut with `…` at the same place whether its row rests or slides.
- **The mouse does what the keys do.** A click opens a row; in a multiple selection a click on the mark (or the cell after it) checks the row without opening it.
- **The selection stays in view.** Moving with the keyboard scrolls the list; the wheel scrolls without changing the selection.
- **Scrollbar only when needed.** A scrollbar column appears on the right when rows overflow, in the style the theme picks; drag it or click it.
- **Faint rows are still rows.** `.faint(true)` draws items that exist but are not ready, like planned components in this menu.

## Styling with a theme

```toml
[style."list-item:hover"]
bg = "$raised"
pillar = "mix($accent, $surface, 45%)"

[style."list-item:selected:focus"]
pillar = "pulse($accent, $accent-2)"
```

Turn the slide off for a theme with `slide = false` under `[motion]`.

## Common mistakes

- **Storing the scroll position.** The runtime keeps it; your state holds the selection only.
- **Rebuilding ids.** Give the list a stable `.id`, especially when it can appear and disappear.
- **Colour-only status.** Pair the status dot with a word in the detail column.
