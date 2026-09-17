## When to use

Use Markdown for text someone writes rather than code: guides, help screens, release notes, descriptions loaded from files. Every guide and reference in this showcase is Markdown.

## Step by step

1. Load or embed the document: `include_str!("help.md")` or text from a file.
2. Add it: `ui.add(Markdown::new(text)).fill_width()`.
3. Put it in a `ScrollView` when it can be taller than the screen.

## What is supported

- Headings: level one and two carry the accent pillar, level three is quieter.
- Paragraphs that wrap at words, with **strong**, *emphasis*, `inline code` and links. Punctuation right after a span stays with it: a `.` or `,` after inline code never starts a line alone, even when a code span wider than the line has to break.
- Bulleted and numbered lists, nested lists.
- Quotes, drawn with a faint pillar.
- Fenced code blocks, highlighted for `rust` and `toml`, with line numbers.

A horizontal rule becomes empty space. Markdown never draws lines of characters.

## How it works

The document is parsed once when the widget is created. Painting lays blocks out for the current width, skips blocks outside the visible area, and styles every piece from the theme, so a document follows theme changes like any other widget.

- **Selectable by itself.** A drag selects inside the document. Copy leaves out the pillars of headings and quotes with the cell after them, and the padding and line numbers of code blocks; Raw copy keeps everything. `.selectable(false)` on the node turns it off.

## Common mistakes

- **Wide tables.** Tables are not rendered; use lists of `name — description`.
- **Very long code lines.** They wrap with an indent; keep samples narrow.
