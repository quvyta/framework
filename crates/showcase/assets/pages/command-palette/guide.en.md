## When to use

Add a command palette when an application has more commands than keys people can remember: navigating to pages, acting on many items, switching themes. It makes everything reachable from the keyboard with a few letters, and it teaches keys by showing each command's chord.

## Step by step

1. Keep whether it is open: `palette_open: bool`, and open it from the global `palette` action (`ctrl p`) in `App::action`.
2. Build the commands in `view`: `PaletteCommand::new("restart-web", t!("restart", name = "web"), Msg::Restart(id))`.
3. Show the key that runs a command elsewhere: `.chord("ctrl r")`.
4. Add it while open: `ui.add(CommandPalette::new(commands, Msg::ClosePalette))`.
5. List the keymap's actions too with `.keymap(true)`; running one is the same as pressing its key.
6. To offer recent commands, keep ids in your state from `.on_run(|id| Msg::Ran(id.to_owned()))` and pass them back with `.recent(ids)`.

## How it works

- **Fuzzy by words.** Letters match in order anywhere in the label; runs of letters and word starts rank first, so "lg red" finds "Show logs of redis". Matched characters take the accent.
- **Keyboard first, the mouse just as well.** The filter always has the keys: ↑/↓ or Ctrl+P/Ctrl+N move, PgUp/PgDn page, Enter runs, Esc closes. With the mouse, pointing at a row moves the same highlight, a click runs it, the wheel and the scrollbar scroll, and the close mark `×` or a click outside closes.
- **One highlight.** The keyboard and the pointer move the same highlight, so two rows are never lit. A pointer that happens to rest where the palette opens changes nothing until it moves.
- **Dismissable means Esc, × and the click outside together.** `.dismissable(false)` turns all three off and hides the mark; running a command still closes the palette.
- **Running closes first.** The close message arrives before the command's message, so the command can open another layer.
- **Virtualised.** Only visible rows are drawn and the highlight scrolls into view; thousands of commands stay quick.
- **Rows behave like list rows.** The highlighted row raises, brightens the pillar on the surface's left edge and slides its label one cell, while the chord stays put on the right.
- **Recent first.** With no query, recent commands appear under "Recent", then "All commands".
- **It sits high.** The palette stays anchored near the top, so its list grows and shrinks below the filter without jumping.

## Common mistakes

- **Labels without verbs.** "Logs" is ambiguous; "Show logs of redis" reads as an action.
- **Unstable ids.** Recent commands match by id; derive ids from what the command does, not from list positions.
- **Doing work while building commands.** `view` runs every frame; commands only carry messages.
