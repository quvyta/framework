## When to use

`Text` is every word on screen that is not part of a control: headings, paragraphs, labels, status lines, hints.

## Step by step

1. `Text::new(t!("key"))` draws body text.
2. Pick a role for hierarchy: `.role("title")`, `"secondary"` or `"faint"`. Roles are defined by the theme's `[typography]`, so hierarchy changes with the theme.
3. Mix styles in one line with `Text::rich([...])` and spans: `Span::new(..).color("success").bold()`, `.on("raised")` for a marked background, `.role("faint")`.
4. Long text wraps to the width it is given. Leave the width to the layout (`.fill_width()` or nothing) and a paragraph uses the whole space and wraps again when the terminal is resized; give it `.width(Length::Cells(60))` only for a fixed measure. Call `.no_wrap()` for a single line that is cut with `…` instead.
5. A path or any text whose start and end both matter is cut in the middle: `Text::new(qframe::text::truncate_middle(path, max))` turns `~/.config/quvyta/launcher.conf` into `~/.config/q…auncher.conf` at 24 cells, keeping the folder tree and the file name.
6. `.align(Align::Center)` or `Align::End` aligns each line.
7. Text is not selectable with the mouse. Where it is content worth copying, such as a message or an address, add `.selectable(true)` to its node.

## How it works

- **Widths are real cell widths.** Turkish letters, combining accents, wide East Asian characters and emoji are measured by their display width, so columns line up.
- **Wrapping keeps words whole.** A line breaks between words, and punctuation stays with its word, so a comma never starts a line; only a word wider than the whole line is split between characters. Spaces at a break are dropped.
- **Every script by its own rules.** Chinese and Japanese are written without spaces, so a line may break between any two characters, but never before closing marks such as `。` `、` `」` or the long vowel `ー`, and never after an opening `「` or `（`. A no-break space (U+00A0, U+202F, U+2007) is part of the word: French puts one before `?` and `:`, and they never start a line. The OTHER SCRIPTS panel shows all three at the width you pick; there is nothing to set.
- **Titles and labels are not content.** Nothing is selectable unless asked, so dragging over headings and hints selects nothing; opt in per node where copying makes sense.
- **Cuts are honest.** When text must not wrap, the last visible cell is an ellipsis, so the reader knows something is hidden.
- **The cut mark follows the glyph mode.** An ASCII terminal cannot show `…`, so in ASCII mode every cut ends in `~` instead, the mark shortened file names have long used. It takes the same one cell, so nothing moves when the mode changes.
- **A middle cut keeps both ends.** `truncate_middle` shares the cells left after `…` between the start and the end, giving the end the odd one, and never splits a wide character or a letter from its accent; a cell one side cannot use goes to the other.
- **Hierarchy is colour and weight.** No underlines of dashes, no capitals shouting. Titles are bold text colour, secondary text is dimmer, faint text is dimmer still.

## Styling with a theme

```toml
[typography]
title     = { fg = "$text", bold = true }
secondary = { fg = "$dim" }
faint     = { fg = "$muted" }
```

## Common mistakes

- **Colour to show hierarchy.** Use roles; keep colour for meaning.
- **Status by colour only.** Write the word too: “passed”, not a green dot alone.
- **Ordinary spaces before French punctuation.** Write `dossier\u{202F}?`, not `dossier ?`, or the mark can end up alone at the start of a line.
- **Cutting important text.** Let explanations wrap; cut only lines where the start says enough, like titles. A path loses its file name when cut at the end; cut it in the middle.
