# Contributing

Thank you for looking. Bug reports, questions, fixes and new components are all welcome. For
anything larger than a small fix, please open an issue first so we can agree on the shape before
you spend time on it.

## Setup

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.95.0, edition 2024); `rustup` picks it up
by itself. Once per clone, enable the checks that run before each commit:

```sh
git config core.hooksPath .githooks
```

The pre-commit hook runs, in the C locale:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

A commit goes through only when all four pass. Please do not skip the hook with `--no-verify`;
pull requests are checked with the same commands before they are merged.

## Tests and the showcase

```sh
cargo test --workspace                     # every test, including the showcase's
cargo run -p quvyta-framework-showcase     # the showcase, or ./showcase.sh
```

Widget tests use `Harness`: it drives an application with keys and clicks and reads the screen as
text and colours, without a terminal. A bug fix comes with a test that fails before the fix.

For a visual review, write every showcase page in every built-in theme to one HTML file:

```sh
QUVYTA_REVIEW=1 cargo test -p quvyta-framework-showcase visual_review
# then open target/showcase-review.html in a browser
```

Please look at your change there in all themes, and try it by hand in the showcase with the
keyboard and the mouse, before opening a pull request.

## Proposing a new component

1. Check that it is really new. A widget whose layout is not fundamentally different from an
   existing one is usually an option on that widget instead.
2. Add an entry to `crates/showcase/CATALOG.toml` with `id`, `name`, `group`, `kind`,
   `priority`, `status = "planned"` and short design notes in `notes` (English) and `notes-tr`
   (Turkish). A planned item shows its notes, faint, in the showcase menu.
3. Build the widget in `crates/quvyta-framework/src/widgets/`, with its tests.
4. Add its showcase page in `crates/showcase/src/pages/`: a live demo, with `// region: <name>`
   markers around the code the Code section shows, and a guide and a reference in English and
   Turkish under `crates/showcase/assets/pages/<page>/`.
5. In the catalog entry, set `status = "done"` and fill in `page` and `types` (every public type
   and function the item adds). The catalog tests fail if a public widget type is not listed in
   any item, or if a `done` item is missing its page, code regions, guide or reference.

If you cannot write the Turkish guide and reference, say so in the pull request; we will add them.

## Rules the code follows

- **Aesthetics.** Shape comes from colour, not characters: no brackets around controls, no frames,
  no `|` separators, in any glyph mode. One accent colour; status colours only with a marker. Only
  list-like widgets slide. Every state is designed: empty, loading, error, narrow, disabled. The
  full list, with the reason for each rule, is in
  [`docs/design-principles.md`](docs/design-principles.md).
- **Simple default.** A widget with no options is its plainest form; each extra ability is an
  independent option.
- **No `unsafe`.** The workspace forbids it (`unsafe_code = "forbid"`). If a system call needs it,
  use a crate that wraps it safely, and say in `Cargo.toml` why the dependency is there.
- **Documentation.** Every public item has English rustdoc (`missing_docs` is denied). Comments
  explain why the code is the way it is, not what it does line by line.
- **Text in language files.** Text a user reads lives in the language files under
  `crates/quvyta-framework/assets/locales/`, not in the source.
- **Loaders never panic.** A broken theme, icon, language or settings file turns into a diagnostic
  with a file, line and column; the built-in defaults keep working.
- **No placeholders.** No `todo!()`, stubs or pretend return values. If something cannot be done
  yet, leave it out and say why.
- **One job per file.** A file that starts doing two things is split.

## Commit messages

Plain English sentences in lower case that describe the effect, for example
`stack a dialog's buttons one under another when they do not fit side by side`.

## Licence

By contributing you agree that your contribution is released under the MIT licence of this
repository.
