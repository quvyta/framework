## When to use

Use a radio group when exactly one of a few options must be chosen and people should see all of them at once: a container engine, a density. For more than about five options, or when space is tight, use a select.

## Step by step

1. Keep the choice in your application: `engine: Option<usize>`.
2. Draw the group: `RadioGroup::new(["Podman", "Docker"]).selected(state.engine)`.
3. Handle choices: `.on_select(|index| Msg::Engine(index))`.
4. Lay short options in a row with `.horizontal(true)`.
5. Start with `None` when there is no sensible default; people then have to choose.
6. The square is the default. For a square that grows into a box add `.style(RadioStyle::Mark)`, for the checkbox's box `.style(RadioStyle::Box)`, for dots `.style(RadioStyle::Dot)`.

## How it works

- **One control, one focus stop.** Tab reaches the group once; arrow keys (Up and Down, or Left and Right in a row) choose the neighbour right away, Home and End the ends.
- **Four styles, no brackets.** `Square` (default): every option shows a small square centred in two cells, quiet for the options not chosen and the chosen colour for the choice. `Mark`: the same, but the chosen square grows into a full two-cell box of the accent. `Box`: every option is the checkbox's two-cell box, empty or filled. `Dot`: a filled dot for the choice and faint rings for the rest.
- **Soft colour changes.** With `Square`, choosing blends the new square's colour from the quiet tone to the chosen tone over two `motion.step`s while the old square blends back; nothing changes shape. With `Mark` the same blend runs and the square swaps for the full box halfway, with no size in between, so nothing wobbles. With reduced motion the change is immediate. The mark is always two cells wide, so labels never move. A hovered square lifts only to a tone below the chosen one, so it never looks chosen.
- **The small square is two sextants.** `🬇🬃` comes from Symbols for Legacy Computing. kitty, WezTerm, Ghostty and foot draw these characters themselves, so they always fit the cell. Other terminals take them from the font; if yours has none, install a font that covers the block or replace the icons.
- **The square lives in the icon set.** It is the icon `radio-mark-small`, which a theme's `[icons]` table can replace. A blank glyph, as in ASCII, draws a box instead whose tone blends from the box style's empty tone to the chosen tone.
- **The box looks like a checkbox, on purpose.** What differs is the meaning: a radio group keeps one option chosen, a checkbox is on or off by itself. Give the group a heading so people know only one can be picked.
- **Hover is per option.** The option under the pointer lifts its square (or its empty box) a step; clicking anywhere on an option, its mark or its label, chooses it. No pillar.
- **Focus breathes on the choice.** Keyboard focus breathes on the chosen box. With nothing chosen, focus warms the first option's square, and Space or Enter chooses it.

## Common mistakes

- **Using radios for on/off.** Two options "On" and "Off" are a switch.
- **Long option lists.** Seven radios push the rest of the form away; use a select.
- **Drawing the square in code.** Replace the `radio-mark-*` icons in a theme instead, so ASCII and every terminal keep working.
