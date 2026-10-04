## When to use

Use big text for the one figure a screen is about: a clock on a status board, the uptime of a service, a countdown, the title of a splash screen. It draws attention strongly, so a screen should have one, rarely two.

## Step by step

1. Add it: `ui.add(BigText::new("14:32"))`.
2. Put a small faint caption above it that says what the figure is.
3. Give the most important figure the accent: `.variant("accent")`.
4. Update the text from your state; it redraws like any text.
5. For a logo or a splash title, let the letters blend into a second theme colour: `.gradient("info", Gradient::Columns)`, or `Gradient::Rows` to blend downwards.
6. Leave room: the text is three rows tall (five in ASCII mode) and about four cells per character.
7. When the text comes from outside — a song title, a file name — ask `BigText::fits(text)` whether it will be drawn big; a character this font has no form for makes the whole text plain bold text.

## How it works

- **A tiny pixel font.** Every glyph is a bitmap five pixels tall and a few columns wide (one for `:`, `.` and `'`, two for a space and `,`, four for `N` and `Ñ`, five for `M`, `W`, `Æ` and `Œ`). It draws the digits, `:`, `.`, `%`, `-`, the punctuation `'`, `!`, `?`, `&`, `(`, `)`, `,`, `/` and every Latin letter: A to Z with their Turkish (`Ç Ğ İ Ö Ş Ü`) and Western European (`À Á Â Ä Å È É Ê Ë Ì Í Î Ï Ñ Ò Ó Ô Ö Ø Ù Ú Û Ü Ý Ÿ ß Æ Œ`) forms.
- **A lowercase letter is drawn uppercase.** It is drawn with the uppercase form of its character, following the Turkish rule: `ı` is always `I`, `i` is `İ` while the active language is Turkish or Azerbaijani and `I` in every other, so "içinde" reads as İÇİNDE on a Turkish screen and IÇINDE on an English one.
- **Five rows leave no room above a capital.** An accent is a mark of its own in the top row and the letter is drawn under it; a cedilla takes the bottom row instead. `Ü` and `Ÿ` use their top row for the two dots and stand a row lower. `Ä` and `Ö` are a pixel wider than the plain letters they come from, because squeezed into three columns they would read as `H` and `V`.
- **Half blocks.** With Unicode and Nerd Font glyphs, two pixels share one cell through `▀`, `▄` and `█`, so five pixels fit in three rows. ASCII mode has no half blocks and paints one pixel per cell in the background colour.
- **A blend, not a glow.** A gradient names the theme colour the letters end in, never a colour of its own, so a logo follows the theme. It is painted once and never moves; for letters that shimmer, use `ShimmerText`. The blend steps per cell: across the columns it has as many steps as the text is wide, down the rows three with Unicode and Nerd Font glyphs and five in ASCII mode.
- **When the blend falls back.** Where the terminal has only the sixteen standard colours, or where the theme does not know the colour asked for, the letters keep their flat colour. Rounding every cell to that palette on its own would speckle them; the flat colour reads in every terminal. The 256-colour palette keeps the blend and rounds each step to its nearest entry.
- **Graceful fallback.** A text with a character this font has no form for, Cyrillic or Japanese or an emoji, is drawn whole at normal size in bold rather than with a hole where that character is; so is a text that does not fit the area. It measures as the one row it paints.

## Common mistakes

- **Sentences.** Big text is for a figure or a word; long text becomes hard to read and wide.
- **Several big figures.** When everything is big, nothing is. Pick the one that matters.
- **No caption.** "99.98%" means nothing without "Uptime" above it.
- **Another script.** Cyrillic, CJK or an emoji has no big form; the text is read as plain bold text, which keeps every character but loses the size.
