# Design principles

These are the rules every qframe widget follows. They are not settings: a widget that breaks one
of them is a bug. Each rule comes with the reason it exists. The full design document,
[`design.md`](design.md), is in Turkish; this page is its short English summary.

## Why no boxes?

Most terminal interfaces draw their structure with characters: `┌──┐` around panels, `[ OK ]`
around buttons, `|` between columns. qframe draws none of them. Buttons, tabs, selected rows and
panels are shaped by a background tone, padding and one accent bar (`▌`).

There are three reasons:

- **Room.** A box costs two columns and two rows. In an 80×24 terminal, a few nested panels
  spend a large share of the screen on their frames.
- **Copying.** Text selected in a terminal takes the frame characters with it. Without them, what
  you copy is what you read.
- **Look.** Modern terminals show 24-bit colour. With tones instead of lines, a terminal
  application can look like a designed desktop application while keeping the speed and plainness
  of a terminal.

The price is a dependency on colour. The built-in themes are tuned so that neighbouring tones stay
apart, and meaning is never carried by colour alone (see rule 5).

## The rules

1. **Shape comes from colour, not characters.** A button, tab, chip, selected row or checkbox is a
   surface tone, padding and elevation. *Why:* characters take room and end up in copied text;
   tone does neither.

2. **No decoration characters, in any glyph mode, ASCII included.** No wrapping in `[ ]`, `( )`,
   `< >` or `{ }`; no `[x]` or `(o)` patterns; no full frames (`┌─┐`, `+--+`); no `|` separators;
   no `===`, `-->` or `>>` ornaments; no lines across the screen. Your own content is exempt. The
   only exceptions are `▌` (the accent bar) and `━` (the track of a slider or rail switch). *Why:*
   one exception invites the next; a hard rule keeps every widget in the same visual language.

3. **Separation by space and tone.** Surfaces stack as canvas, surface, raised, active and overlay.
   A divider or panel edge is a tone step: it brightens on hover and takes the accent while you
   drag it. *Why:* a line would be a box by another name, and a tone step still shows where you
   can grab.

4. **Hierarchy by colour and weight.** Headings and secondary text differ in tone and weight, set
   in the theme's `typography`. *Why:* a terminal has one font size, so tone and weight are the
   only levers left.

5. **One accent colour.** Status colours (success, warning, danger, info) are used only for
   meaning, and always together with a marker: a dot, an icon or a word. *Why:* a second accent
   competes with the first, and colour alone fails colour-blind users and monochrome terminals.

6. **The accent bar is `▌`.** It sits at the far left of whatever is raised, hovered or selected.
   A passing state (hover, focus) marks one cell; a whole surface (a dialog, a panel) carries the
   bar down its full left edge. *Why:* one mark used everywhere is learned once.

7. **Tones only climb.** Rest, hover, focus and pressed get steadily stronger, never the reverse.
   Keyboard focus is shown only when focus arrived by keyboard; a clicked control stays calm under
   the pointer. *Why:* a flash that looks like shrinking reads as a glitch, and a focus ring after a
   click tells the user nothing new.

8. **Only lists slide.** In lists, menus, trees, tables, tabs and dropdown options, the leading icon
   and text of the selected row move one cell to the right. The bar, checkboxes, expand arrows and
   controls on the right stay put. Buttons and fields never slide, and turning sliding on or off
   changes no width. *Why:* a moving row shows where the selection went; a moving button would
   only look unstable.

9. **Motion whispers.** Durations come from the theme's `[motion]` table. When a movement steps
   cell by cell, its colour blends in proportion to the step, so there are no cartoon-like
   in-between frames. Reduced motion (a setting, `Command::set_reduced_motion` or
   `QUVYTA_REDUCED_MOTION`) makes everything instant. *Why:* motion should explain a change, not
   draw attention to itself.

10. **No fixed colours.** Every colour comes from the theme: tokens like `$accent`, and functions
    like `mix($a, $b, 40%)`. *Why:* a hard-coded colour breaks the moment someone switches theme.

11. **Three glyph modes.** Every icon is designed for Nerd Font, Unicode and ASCII, and every glyph
    is exactly one cell. ASCII mode uses real ASCII only. *Why:* not every terminal has a patched
    font, and a glyph of the wrong width shifts the whole line.

12. **Every state is designed.** Empty, loading, error, narrow, disabled, hovered, focused and
    pressed. *Why:* the undesigned state is the one users meet first, on a small window or a slow
    disk.

13. **Simple default, layered options.** A widget with no options is its plainest form. Each extra
    ability is an independent option. A new widget is added only when its layout is fundamentally
    different (horizontal `Tabs` and vertical `TabRail`); shared behaviour lives in one internal
    model. *Why:* options that combine freely stay understandable, and one model means one place to
    fix a bug.

## How the rules are kept

Part of this is enforced by tests. The showcase tests fail if any page, in any glyph mode, draws a
box-drawing character, or brackets on a demo surface. Widget tests check screen text,
colours, keyboard and mouse behaviour, narrow widths, reduced motion and ASCII mode through the
`Harness`. The rest is review: every page is looked at in each built-in theme before it is called
done (see [CONTRIBUTING.md](../CONTRIBUTING.md)).
