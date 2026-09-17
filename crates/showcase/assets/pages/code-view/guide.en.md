## When to use

Show code people read, copy or compare: snippets in help, configuration previews, diffs of settings. The Code section of every page in this showcase is a code view of the page's own source.

## Step by step

1. Create it with the code and its language: `CodeView::new(source, Language::Rust)`.
2. Hide line numbers for short snippets: `.line_numbers(false)`.
3. React to copying: `.on_copy(Msg::Copied)`, for example to show a toast.

## How it works

- **Highlighting from the theme.** Keywords, types, functions, macros, strings, numbers, comments, attributes and lifetimes for Rust; tables, keys, strings, numbers and booleans for TOML. Every kind is a `code-token` variant, so a theme restyles code like any widget.
- **Long lines wrap** with a two-cell indent instead of being cut; the line number appears only on the first row.
- **Copy with one key.** Focus the block with Tab and press `c`: the code goes to the clipboard, also over SSH, and the block flashes.
- **The surface is its frame.** A code block is a slightly raised surface with padding; no borders.
- **Selectable by itself.** A drag inside the code selects text and stays in the block, never its padding. Copy (`ctrl c` or the right click menu) leaves the line numbers out; Raw copy keeps them. Turn selection off with `.selectable(false)` on the node.

## Styling with a theme

```toml
[style."code-token.keyword"]
fg = "$accent"

[style."code-token.comment"]
fg = "$muted"
italic = true
```

## Common mistakes

- **Plain text as code.** Use `Language::Plain` for logs and output; highlighting would mislead.
- **Huge files.** A code view draws all its rows; wrap it in a scroll view and show the part that matters.
