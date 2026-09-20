## When to use

Show code people read, copy or compare: snippets in help, configuration previews, diffs of settings, a package recipe to review before it runs. The Code section of every page in this showcase is a code view of the page's own source.

## Step by step

1. Create it with the code and its language: `CodeView::new(source, Language::Rust)`.
2. Hide line numbers for short snippets: `.line_numbers(false)`.
3. React to copying: `.on_copy(Msg::Copied)`, for example to show a toast.
4. Show a diff: `.line_marks(marks)` with one `LineMark::Added`, `Removed` or `Unchanged` per line.
5. Point at lines: `.highlight_lines(12..=14, LineTone::Warning)` for findings, `LineTone::Accent` for the line someone went to.
6. Go to a line: put the code view in a `ScrollView` and add `.reveal(line)`; the scroll view glides just far enough to show it. In a diff, go by the file's own number instead: `.reveal_number(22)`.
7. A diff that starts part way into a file: `.line_numbers_from(numbers)`, one number per line and `None` where a line has none, such as a hunk header.

## How it works

- **Highlighting from the theme.** Keywords, types, functions, macros, strings, numbers, comments, attributes and lifetimes for Rust; tables, keys, strings, numbers and booleans for TOML; comments, strings, variables and expansions, keywords, function definitions and here-documents for shell scripts. Every kind is a `code-token` variant, so a theme restyles code like any widget.
- **The language from a file name.** `Language::from_file_name("PKGBUILD")` is shell, like `.sh`, `.bash`, `.zsh` and `.install`; `.rs` is Rust, `.toml` TOML, anything else plain.
- **Diffs and findings are tints with a sign.** Marked or highlighted lines get a tint across the whole row, every wrapped row included, and a sign in a one-cell column at the left edge: `+` on success for added lines, `−` on danger for removed ones, the warning icon for a warning, the pillar for the accent. A highlight wins over a diff mark on the same line. The column appears only once something is marked, and copies leave the signs out.
- **Going to a line glides.** `.reveal(line)` scrolls the enclosing scroll view the least that shows the line with two rows of context, over the length of a page change; with reduced motion it jumps. It happens when the line changes, so the user can scroll away afterwards.
- **Long lines wrap** with a two-cell indent instead of being cut; the line number appears only on the first row.
- **Copy with one key.** Focus the block with Tab and press `c`: the code goes to the clipboard, also over SSH, and the block flashes.
- **The surface is its frame.** A code block is a slightly raised surface with padding; no borders.
- **Selectable by itself.** A drag inside the code selects text and stays in the block, never its padding. Copy (`ctrl c` or the right click menu) leaves the line numbers out; Raw copy keeps them. Turn selection off with `.selectable(false)` on the node.

- **A diff's numbers belong to the files, not to the text.** A diff stands the lines of two versions one after another, so counting from the top numbers neither of them: a finding that says `PKGBUILD:22` would point at the wrong line. With `.line_marks(...)` the numbers follow the files instead — a removed line carries the old file's number, an added line the new file's, and a line in both carries the new file's. `.reveal_number(22)` then goes to the line that number means; where a removed line and an added line share a number, the line the new file numbers that way is the one reached, because that is the file a finding is about.

## Styling with a theme

```toml
[style."code-token.keyword"]
fg = "$accent"

[style."code-token.comment"]
fg = "$muted"
italic = true

[style."code-line.added"]
bg = "mix($success, $surface, 10%)"
fg = "$success"
```

## Common mistakes

- **Plain text as code.** Use `Language::Plain` for logs and output; highlighting would mislead.
- **Huge files.** A code view draws all its rows; wrap it in a scroll view and show the part that matters.
- **Revealing every frame.** `.reveal(line)` acts when the line changes; keep the line in your state and change it when the user asks to go somewhere, not on every view.
- **A status colour without meaning.** Use `LineTone::Accent` to point at a line; `Warning` says something is worth a careful look.
