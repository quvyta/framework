## IconTile

- `IconTile::new(glyph, name)` — a tile drawn with `glyph` and called `name`. `glyph` is a key of
  the icon set, such as `"folder"`, or a `Glyph::literal` for a character of the application's own.
- `.selected(bool)` — the tile is one of the chosen ones: it takes the `active` ground and the
  accent pillar down its whole height, and its name takes the text colour.
- `.cursor(bool)` — the keyboard is on the tile: the accent pillar alone, so a surface that keeps the
  selection and the cursor apart can say which is which.
- `.backed(bool)` — the tile stands on a tile of the theme's `surface` tone under its glyph and its
  name, for a floor with a picture under it.
- `.color(token)` — the glyph is drawn in theme colour `token`, such as `"accent"` or `"series-3"`.
  The name keeps its own tone.
- `.faint(bool)` — the tile is drawn in the `faint` role's tone: a cut entry, or one on its way
  somewhere. A chosen faint tile keeps the selected row's colours.
- `IconTile::shown_name(name) -> Cow<str>` — the name as a tile shows it: whole when it fits, else cut
  with an ellipsis. Put the whole one in a `Tooltip` beside the tile.
- `IconTile::WIDTH` (10), `IconTile::HEIGHT` (3), `IconTile::SIZE` and `IconTile::PILLAR` (1) — the
  size one tile takes and the column kept for the pillar.

## Input

A tile draws and answers nothing: no key, no press, no focus. The surface that lays tiles out owns
the pointer, so a grid of them is a `CardGrid` with `.bare_cards(true)` and its own `.on_select` and
`.on_activate`, and a desktop's own floor reads the mouse against its grid. Such a grid lends a tile
the pointer and the focus of the cell it stands in, so it lights and breathes as it would alone.

## Layout

- A tile measures itself as `SIZE` in the room it is given, and draws what fits: the glyph centred
  in the columns beside the pillar, the name under it, cut again to what is really there.
- Three rows: the glyph, the name, and a free row under both so two tiles never touch.
- A row of tiles lays out like any other row; a `CardGrid` with `.card_width(IconTile::WIDTH,
  IconTile::WIDTH)`, `.card_height(IconTile::HEIGHT)` and `.gap(1, 1)` gives one cell of a gap
  between them.

## Theme keys

- `icon-tile` — `fg` (the glyph's colour), `name` (the name's), `bg` (the ground a chosen tile
  takes), `pillar` (the accent of its first column).
- `icon-tile` with states `hover`, `selected` and `focus`; a `focus` rule written after `selected`
  wins where a chosen tile has both, so the chosen tile's pillar breathes too.
- `icon-tile.faint` — the variant of a `[variant]`-style entry the application has marked.
- `pulse()` on the pillar is what makes it breathe; `mix()` on the ground is what lifts it under the
  pointer, and a `card` grid's own `bg` and `pillar` are not drawn for a bare card.
