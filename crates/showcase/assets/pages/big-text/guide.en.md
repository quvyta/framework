## When to use

Use big text for the one figure a screen is about: a clock on a status board, the uptime of a service, a countdown, the title of a splash screen. It draws attention strongly, so a screen should have one, rarely two.

## Step by step

1. Add it: `ui.add(BigText::new("14:32"))`.
2. Put a small faint caption above it that says what the figure is.
3. Give the most important figure the accent: `.variant("accent")`.
4. Update the text from your state; it redraws like any text.
5. Leave room: the text is three rows tall (five in ASCII mode) and about four cells per character.

## How it works

- **A tiny pixel font.** Every glyph is a bitmap five pixels tall, three wide (one for `:` and `.`, four for `N`, five for `M` and `W`). Digits, `:`, `.`, `%`, `-` and the letters A to Z are included; lowercase uses the uppercase forms; other characters become a space.
- **Half blocks.** With Unicode and Nerd Font glyphs, two pixels share one cell through `▀`, `▄` and `█`, so five pixels fit in three rows. ASCII mode has no half blocks and paints one pixel per cell in the background colour.
- **Graceful fallback.** When the area is too narrow or too short, the text is drawn at normal size in bold instead of being cut.

## Common mistakes

- **Sentences.** Big text is for a figure or a word; long text becomes hard to read and wide.
- **Several big figures.** When everything is big, nothing is. Pick the one that matters.
- **No caption.** "99.98%" means nothing without "Uptime" above it.
