## When to use

Reach for a document when the file belongs to your application rather than to the user: a project file, a profile definition, a record of a session, a list of sources. It is a TOML file with a shape you declare, it can hold an array of tables (`[[profile]]`), and it is never repaired behind your back.

`Settings` is the other half and not this one. Settings are preferences: every key has a default, self-healing may rewrite the file, and a value it cannot store is dropped. A document has no defaults, holds tables and arrays of tables, and reading it writes nothing at all — not the file, not a backup beside it.

Four differences, and each of them decides which one you want:

- **Whose file it is.** `Settings` holds the user's preferences; a document holds your application's data.
- **Defaults.** Every settings key has one, so a missing key is not a problem. A document key is required or optional, and nothing is ever filled in for it.
- **What it can hold.** `SettingValue` has no table, so `[[profile]]` entries are skipped. A document has `Shape::table` and `Shape::entries`.
- **What a broken file costs.** Self-healing settings may be rewritten, keeping the old file as `settings.toml.bak`. A broken document is only reported, and saving is a step you take yourself with `storage::atomic_write`.

## Step by step

1. Describe the file once: `Shape::new().required("id", ValueKind::text())`, `.optional(…)`, `.table("engine", …)` for a table inside it and `.entries("profile", …)` for one entry per `[[profile]]`.
2. Read it: `Document::open(path, &shape)?`, or `Document::parse(name, text, &shape)` when you already hold the text.
3. Pull what you need out of `document.root()`: `text`, `integer`, `flag`, `table` and `entries`. Each answers `None` — or no entries — for anything that was missing or unreadable.
4. Show `document.diagnostics()` to the user. Each one carries the file, line and column where the problem is.
5. Decide what a document without its required keys means to you. `Document` reports; only you know whether the rest is still worth opening.
6. When you save, build the text yourself and write it with `storage::atomic_write`. Reading and writing are two steps on purpose.

## How it works

- **The shape is the whole check.** A declared key of the declared type is kept. A key nobody declared is a warning and is dropped. A value of another type is dropped too: an error when the key is required, a warning when it is optional. A required key that is not in the file at all is an error, reported where its table starts — for an entry of an array, at the `[[profile]]` line that opens it.
- **A broken document still gives its readable part.** A file a user edited by hand is usually wrong in one place. An application that can still name the project, with one unreadable profile missing, is more use than one that opens nothing; and every other loader in this framework reads that way. The diagnostics say exactly what was lost.
- **Nothing is ever written while reading.** There is no repair, no default written into the file and no `.bak` file. This is not a setting you can turn off: the type has no way to write.
- **A syntax error does not end the read.** The parser recovers, the error is reported with its line and column, and the keys after it are still read.
- **Keys are names, not paths.** Nesting is declared with `Shape::table`, which reads `[engine]` with `kind` under it and the same key written as `engine.kind` — the same file either way.
- **Writing stays yours.** Your structure knows the order, the grouping and the comments your file should have; a generic writer would not. `storage::atomic_write` is the safe write itself, and the demo shows the file the application would hand it.

## Common mistakes

- **Using `Settings` for a data file.** It has no table value, so `[[profile]]` entries are skipped without a word, and with self-healing on the file is rewritten without them.
- **Treating a diagnostic as a failure.** Most of them mean "this one entry is gone", not "this file is unusable". Read `root()` first, then decide.
- **Expecting an empty document from a missing file.** `Document::open` returns the I/O error, because "not written yet" and "written empty" are different things and only you know which you expect.
- **Declaring defaults in the shape.** There are none. Keep your fallbacks where the value is used, so the file stays the user's words and not ours.
- **Reporting your own problems without a place.** When a value reads but means nothing to you — a name no file system accepts, a day the calendar does not have — `Table::value_location(key)` gives you the line and column of the value itself, the same place the document's own type errors point at; `Table::location(key)` gives the key's.
