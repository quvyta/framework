## View

- `ui.add(widget)` — adds a widget and returns its node.
- `ui.add_with(container, |ui| ...)` — adds a widget that holds children, such as `Panel` or `ScrollView`.
- `ui.column(|ui| ...)`, `ui.row(|ui| ...)`, `ui.stack(|ui| ...)` — containers.
- `ui.page(key, |ui| ...)` — a column that remembers focus and scroll while hidden.
- `ui.spacer()` — empty space that fills.
- `ui.env()` — theme, icons, language and keymap.
- `ui.size()` — the terminal's `Size` (`width`, `height` in cells), the same in every nested builder; for choosing the application's arrangement.

## Node

- `.id(name)` — a stable name; state and focus follow it.
- `.width(Length)`, `.height(Length)`, `.fill()`, `.fill_width()`, `.fill_height()`.
- `.padding(Padding)`, `.gap(cells)`, `.justify(Align)`, `.align(Align)`.
- `.wrap(bool)` — a row moves the children that do not fit to the next line; `gap`, `justify` and spacers work on each line.
- `.line_gap(rows)` — empty rows between the lines of a wrapping row (default 0).
- `.selectable(bool)` — `true` makes the node a region where a mouse drag selects text; `false` keeps selection out of it and everything inside.

## Length and Align

- `Length::Auto`, `Length::Cells(n)`, `Length::Fill(weight)`.
- `Align::Start`, `Align::Center`, `Align::End`.

## Router

- `Router::new(root)`, `.current()`, `.push(page)`, `.back() -> bool`, `.replace(page)`, `.can_go_back()`, `.history()`, `.direction()` (`Navigation::Forward` or `Navigation::Back`, for page transitions).

## AppShell

- `AppShell::new()` with `.header`, `.sidebar`, `.body`, `.footer` closures and `.show(ui)`.
- `.sidebar_width(columns)` (default 28), `.collapse_below(columns)` (default 90), `.sidebar_open(bool)`.
- Theme keys: `shell-header`, `shell-sidebar`, `shell-body`, `shell-footer` (`bg`).
