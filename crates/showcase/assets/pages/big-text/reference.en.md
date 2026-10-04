## Methods

- `BigText::new(text)` — big `text`.
- `BigText::fits(text)` — whether every character of `text` has a big form, so it is drawn as glyphs and not as plain bold text; a space counts, it draws as a blank between two words.
- `.variant(name)` — theme variant, such as `"accent"` or `"dim"`.
- `.gradient(to, direction)` — blends the letters into theme colour `to`, running `Gradient::Columns` (left to right) or `Gradient::Rows` (top to bottom).

## Behaviour

- Measures the width of its glyphs plus one column between them, and three rows (five in ASCII mode).
- Draws `0`–`9`, `:`, `.`, `%`, `-`, `'`, `!`, `?`, `&`, `(`, `)`, `,`, `/` and every Latin letter: A–Z with their Turkish `Ç Ğ İ Ö Ş Ü` and Western European `À Á Â Ä Å È É Ê Ë Ì Í Î Ï Ñ Ò Ó Ô Ö Ø Ù Ú Û Ü Ý Ÿ ß Æ Œ` forms.
- A lowercase letter is drawn with the uppercase form of its character: `ı` is always `I`, `i` is `İ` while the active language is Turkish or Azerbaijani and `I` in every other. The language in force while painting decides, as `Env::i18n` gives it.
- A text with a character this font has no form for, Cyrillic or Japanese or an emoji, is drawn whole at normal size in bold instead of with a hole where the character is, and measures as the one row it paints; so is a text whose glyphs do not fit the area, cut with `…` if needed. The fallback keeps the flat colour.
- Without a gradient the letters take one colour, the style's `fg`. A gradient blends from that colour into the theme colour named, one step per cell of the direction it runs in.
- The gradient falls back to the flat colour at `ColorDepth::Ansi16` and when the theme does not know the colour named; `Ansi256` keeps it.
- Static: the blend never animates. Not focusable; sends no messages.

## Theme keys

- `big-text`, `big-text.<variant>` — `fg`.
