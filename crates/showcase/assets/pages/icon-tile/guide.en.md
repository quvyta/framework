## When to use

Use an icon tile when a person recognises things by their picture and not by their name: the icons
on a desktop, the entries of a file manager's grid. A tile is a glyph over a centred name in a cell
of ten columns by three rows, so a folder of them lines up however many there are and however long
their names are. For a list with a size, a date and permissions, use a table; for one item that
takes a click, use a button.

## Step by step

1. Say what the tile shows: `IconTile::new("folder", "Photos")`. The first argument is a key of the
   icon set, so the glyph follows the glyph mode; a character of your own is a `Glyph::literal`.
2. A tile measures itself as `IconTile::WIDTH` columns by `IconTile::HEIGHT` rows, or less in a
   narrower place, and draws what fits there.
3. Say which tiles are chosen with `.selected(bool)` and which one the keyboard is on with
   `.cursor(bool)`.
4. Over a picture, say so with `.backed(bool)`: the tile then stands on a tile of the theme's
   `surface` tone, so its name reads whatever the picture is.
5. For a kind told by its colour, say so with `.color("series-3")`. The name keeps its own tone, so
   a grid of coloured icons still reads as one.
6. For a cut entry or one on its way somewhere, say so with `.faint(bool)`.
7. A tile draws and nothing more: the surface that lays tiles out takes the presses. Put tiles in a
   [`CardGrid`](card-grid) with `.bare_cards(true)` and wire the grid's `.on_select` and
   `.on_activate`, or draw them yourself over your own floor as a desktop does.

## How it works

- **The pillar's column is always there.** A chosen or a cursor tile carries the accent pillar `▌`
  down its first column, whether it is chosen or not, so the glyph and the name stand in the same
  cells in every state and nothing shifts.
- **A chosen tile takes the selected ground.** Its name takes the text colour; an unchosen tile's
  name keeps the `dim` tone. A cursor tile takes the pillar alone, so a surface that keeps the two
  apart says which is which.
- **The pointer lifts the ground a step**, as a pressable panel does, and the name brightens with
  it. A tile over a picture lifts only its own tile of the surface tone, which is not the picture's
  to tint.
- **A name is cut with an ellipsis.** `IconTile::shown_name` is the name as a tile shows it; put the
  whole one in a `Tooltip` beside the tile.
- **The keyboard makes the pillar breathe** while the grid has focus reached with the keyboard, and
  reduced motion holds it still.
- **ASCII keeps up.** The icon set's ASCII column is used in ASCII mode, and a name cut where the
  terminal cannot show an ellipsis ends in `~` instead.

## Styling with a theme

```toml
[style.icon-tile]
fg     = "$text"
name   = "$dim"

[style."icon-tile:hover"]
name = "$text"

[style."icon-tile:selected"]
bg     = "$active"
name   = "$text"
pillar = "$accent"

[style."icon-tile:focus"]
pillar = "pulse($accent, $accent-2)"

[style."icon-tile.faint"]
fg   = "$muted"
name = "$muted"
```

## Common mistakes

- **A tile that answers a press.** It draws and nothing more; a surface that lays tiles out owns the
  pointer, and a tile that also answered would take it away from the ground around it.
- **One column fewer on a chosen tile.** Keep `IconTile::PILLAR` in every tile so a selection does
  not shift the icons next to it.
- **A grid of tiles with the cards' own surfaces.** Give the grid `.bare_cards(true)`: a bare card
  stands on the ground the grid is given and takes the whole of its cell, so only the chosen tile
  has a surface of its own.
