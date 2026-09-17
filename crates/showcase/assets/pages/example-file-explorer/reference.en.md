## Methods

- `Tree::new(nodes)` with `.selected`, `.on_select`, `.on_expand`; nodes use `.expandable(true)`, `.expanded(bool)`, `.loading(bool)` and `.children(..)`; an unreadable folder gets a `.faint(true)` child with `.icon("error", Some("danger"))`.
- `Table::new(columns, rows)` with `.selected`, `.sort`, `.on_select`, `.on_activate`, `.on_sort`, `.empty_text`; cells use `TableCell::new(..).icon("folder", Some("accent"))`.
- `read_folder(path)` inside `Command::perform`, giving a `Listing` of `FileEntry` values.
- `CodeView::new(text, Language::Rust | Toml | Plain)` and `Markdown::new(text)` inside a `ScrollView`.
- `Text::rich(spans)` with `Span::role("faint" | "secondary" | "body")` and `.bold()` for the path and the status.

## Behaviour

- Selecting a folder in the tree makes it current; opening a node reads it the first time.
- Selecting a table row previews a file; opening a folder row makes it current and opens its ancestors in the tree.
- A folder or file still being read does not clear what is shown: the table and the preview change in one frame when the answer arrives. The tree row of a folder read for longer than 300 ms spins for at least 500 ms.
- Sorting keeps folders on top and orders the rest by name or size.
- The preview reads at most 64 KiB and 400 lines; files containing zero bytes are reported as binary.
- Hidden entries (names starting with a dot) are left out.

## Theme keys

- The example adds no styles: rows use `list-item`, the header `table-header`, chevrons `tree-chevron`, the preview `code` and `markdown-*`, the path `faint` and `secondary` typography.
