## Methods

- `CommandPalette::new(commands, on_close)` — the palette; Esc, the close mark, a click outside and running a command send `on_close`.
- `.dismissable(bool)` — whether Esc, the close mark and a click outside close it, all together. Default: `true`. `false` hides the mark; running a command still closes.
- `.keymap(bool)` — also list keymap actions (not focus moves or the palette itself). Their ids are `action:<name>`. Default: `false`.
- `.recent(ids)` — ids listed first while the filter is empty, most recent first.
- `.on_run(|id| msg)` — sent after every command that runs.
- `.placeholder(text)` — the faint text of the empty filter. Default: `quvyta.palette.placeholder`.
- `.width(cells)` — width with padding. Default: 72.
- `PaletteCommand::new(id, label, msg)` — a command.
- `.chord(label)` — the key shown on the right, e.g. `"ctrl r"`.

## Keys

- Typing, `backspace`, `ctrl w`, `ctrl u`, `left` `right` `home` `end` edit the filter; pasting works too.
- `up` `down` or `ctrl p` `ctrl n` move the highlight; `pgup` `pgdn` move by a page.
- `enter` runs the highlighted command; `esc` closes while dismissable.

## Mouse

- Moving the pointer over a row moves the one highlight; a pointer resting where the palette opened waits until it moves.
- Click a row to run it; the wheel scrolls and the scrollbar can be pressed and dragged.
- The close mark `×` in the top right corner of the surface, on the row above the filter, closes: three cells that light up under the pointer. A click on the dimmed screen closes too. Both only while dismissable.

## Behaviour

- With a query, entries are sorted by match quality (ties keep their order) without headers.
- Running sends `on_close`, then the command's message or its keymap action, then `on_run`.
- Up to 10 rows before scrolling; the filter, highlight and scroll reset when it opens again.
- The hint line shows `esc close` (while dismissable), `↑↓ move`, `⏎ run` and how many commands match.
- A pillar runs down the surface's left edge; the highlighted row's own pillar brightens it on that row.
- A modal layer placed in the upper part of the screen; focus returns on close.

## Theme keys

- `modal` (`bg`, `padding`, `pillar`), `layer-backdrop` — the surface; `close-mark` — the close mark.
- `layer-filter`, `layer-filter-mark`, `layer-filter-placeholder`, `layer-filter-cursor`, `layer-match`.
- `palette-item` with `hover` (`bg`, `fg`, `pillar`), `palette-chord` with `hover`, `palette-header`.
- `layer-hint-key`, `layer-hint-label`, `scrollbar`; `[motion]` — `enter`, `slide`.
- Language — `quvyta.palette.placeholder`, `.recent`, `.all`, `.empty`; `quvyta.layer.close`, `.move`, `.run`.
