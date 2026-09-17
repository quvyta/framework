## When to use

Every scrolling widget draws the same scrollbar, and the theme decides how it looks. Choose a style when you write a theme: `block` is the default and reads well everywhere, `half` is lighter, `thin` suits dense tools, `dots` is the quietest. Pin a style on one widget only when that place needs a different look from the rest of the application.

## Step by step

1. In a theme file, pick the style: `[style.scrollbar]` then `style = "thin"`.
2. Tune its colours if needed: `track` and `thumb`, and `[style."scrollbar:hover"]` for the brighter thumb while the bar is hovered or dragged.
3. To tune one style without touching the others, write a variant rule named after it: `[style."scrollbar.dots"]`.
4. To pin a style on one widget, call `.scrollbar(ScrollbarStyle::Thin)` on a `List` or a `ScrollView`.

## How it works

- **One column, only on overflow.** When everything fits, no style draws anything and the content gets the column back.
- **No frames.** The track is a tone or a faint glyph, never a line around the content.
- **`block`** uses no glyphs at all: track and thumb are background colours. It looks the same in every glyph mode, and since there is no character in the column, a text selection never picks the scrollbar up. **`half`** draws a thin track `▕` with a half-block thumb `▐`. **`thin`** draws no track, only a thin thumb. **`dots`** draws a dotted track `·` with a filled thumb `•`.
- **ASCII terminals** get coloured cells in place of block glyphs, so every style still works.
- **Unknown words are reported.** A theme with `style = "wavy"` gets a diagnostic with the file, line and column, and the widget keeps the default.

## Common mistakes

- **Pinning a style everywhere.** Then the theme can no longer change it; pin only where a place is truly different.
- **A thumb too close to the track.** Check `thumb` against `track` in every theme, especially for `block`, where colour is all there is. On a raised surface (a text area) the track can match the surface; the thumb still shows the position.
