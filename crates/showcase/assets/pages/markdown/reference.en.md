## Methods

- `Markdown::new(source)` — parses the document.
- The whole document is a text selection region; heading and quote pillars (with their gap) and code block padding and gutters are decoration for clean copies.

## Blocks

- `#` and `##` headings with the pillar; `###` and deeper as quiet headings.
- Paragraphs, soft and hard line breaks.
- `-` and `1.` lists, nested.
- `>` quotes.
- Fenced code with `rust`, `toml` or no language.
- `---` becomes space.

## Inline

- `**strong**`, `*emphasis*`, `` `code` ``, `[links](url)` (text only).

## Theme keys

- `markdown-heading` with variants `h1`, `h2`, `h3` — `fg`, `bold`, `pillar`.
- `markdown-text`, `markdown-strong`, `markdown-emphasis`, `markdown-code`, `markdown-link`, `markdown-bullet`, `markdown-quote` (`fg`, `italic`, `pillar`).
- Code blocks use `code`, `code-line-number` and `code-token.<kind>`.
