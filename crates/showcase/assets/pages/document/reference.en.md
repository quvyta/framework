## Shape

- `Shape::new()` — a table that declares nothing yet; `Default` gives the same.
- `.required(key, kind)` — a key the document must hold. A missing one is an error at the table's start, a value of another type an error where it stands.
- `.optional(key, kind)` — a key it may hold. A missing one is no problem, a value of another type a warning.
- `.table(key, shape)` — a table inside this one. The table itself is optional; its required keys are required once it is there. Reads `[key]` and the dotted `key.name` alike.
- `.entries(key, shape)` — an array of tables, `[[key]]` repeated, each entry shaped on its own. A document that lists none simply has none.
- Keys are single names, not dotted paths. Declaring a name again replaces whatever it declared before, whichever of the four builders declared it.

## ValueKind

- `ValueKind::text()` — any string.
- `ValueKind::choice(["podman", "docker"])` — one string of a fixed list; anything else is reported as `must be one of podman, docker`.
- `ValueKind::integer()` — a whole number, in any base TOML writes (`0x1f` reads as 31).
- `ValueKind::flag()` — `true` or `false`.
- There is no float, no array of scalars and no validator, because no data file the ecosystem writes holds one yet. Check what only your application can judge after reading it, and report it at `Table::value_location(key)`, where the framework reports its own findings about a value.

## Document

- `Document::parse(file, text, &shape)` — reads TOML text; `file` is the name diagnostics carry.
- `Document::open(path, &shape) -> io::Result<Document>` — reads the file at `path` and names diagnostics after it. A missing file is an `io::Error`, not an empty document.
- `.root() -> &Table` — the document's root table.
- `.diagnostics() -> &[Diagnostic]` — every problem, in the order they were found: syntax errors first, then the shape check.
- `.is_clean() -> bool` — whether nothing at all was wrong.
- Reading never writes, never repairs, never leaves a backup and never panics.

## Table

- `.text(key)`, `.integer(key)`, `.flag(key)` — the value when it was there and of the declared type, `None` otherwise. `choice` reads back with `text`.
- `.table(key) -> Option<&Table>` — the nested table, when the document holds it.
- `.entries(key) -> &[Table]` — the entries in file order; empty when there are none.
- `.location(key) -> Option<&Location>` — where the key was written.
- `.value_location(key) -> Option<&Location>` — where the value was written: in `mode = "halb"` the column of `"halb"`, the place a wrong type or choice is reported at, for a problem only the application can see. `None` for a key that was missing or unreadable.

## Diagnostics

- `` `colour` is not part of the document; it is ignored `` — a warning where the key stands.
- `` `name` must be a string, found 7; it is ignored `` — an error for a required key, a warning for an optional one, where the value stands.
- `` `id` is required and missing `` — an error at the start of the table it is missing from.
- `` `profile[1].name` is required and missing `` — the same inside an entry of an array, at the line that opens the entry.
- A syntax error keeps the parser's own words, with its line and column.

## Keys

- The playground switches documents with a segmented control; nothing on this page has keys of its own.
