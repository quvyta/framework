## CardGrid

- `CardGrid::new(count)` — a grid of `count` cards, 24 to 32 cells wide, three rows of content, two cells and one row apart.
- `.card(|ui, index| ..)` — builds what card `index` shows; called for the cards on screen while the grid paints. The closure is `'static`.
- `.card_width(min, max)` — the least and the most a card gets; the column count follows from `min`.
- `.card_height(rows)` — rows of content in every card, without its padding.
- `.gap(columns, rows)` — cells between cards of a row, rows between rows of cards.
- `.selected(Option<usize>)` — the selected card.
- `.on_select(|index| msg)` — the selection moved.
- `.on_activate(|index| msg)` — a card was opened with Enter or a click.
- `.checked(Vec<bool>)` and `.on_toggle(|index| msg)` — checks, off by default.
- `.empty(EmptyState)` — what an empty grid shows.
- `.disabled(bool)` — no hover, focus or press; cards fade, the selection still shows.
- `.scrollbar(ScrollbarStyle)` — pins a scrollbar style instead of the theme's.

## Keys

- Arrows move between cards and stop at the edges; `home` `end` go to the first and last card; `pgup` `pgdn` move the rows that fit; `enter` activates; `space` checks while checks are on, activates otherwise.

## Mouse

- A click selects and activates a card; a click on the mark in a card's top right corner (or a cell beside it) only checks it; the wheel scrolls a row of cards; drag the scrollbar.

## Layout

- Columns: `(width + gap) / (min + gap)`, at least one. Every card is `(width - gaps) / columns` wide, at most `max`.
- An area narrower than `min` shows one column as wide as the area.
- The scrollbar takes the last column only when rows overflow; the wheel and the scrollbar count rows of cards.

## Theme keys

- `card` with `hover`, `selected`, `focus`, `pressed` — `bg`, `padding`, `pillar`.
- `card-mark` — the mark of a checked card; `card-mark.off` — the faint mark a lit card offers while checks are on.
- `scrollbar` (`style`, `track`, `thumb`) and `scrollbar.<style>`.
- `[icons]` — `check`: the mark (`✓`, ASCII `v`); `pillar`.
