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
- `.activate_on(Click::Single | Click::Double)` — `Single` by default: a click selects and opens. With `Double` a click only selects and a second press on the same card within `Click::INTERVAL` (400 ms) opens it; Enter opens either way.
- `.multi_select(&selected, |cards| msg)` — several cards are selected at once: Ctrl+click adds or removes a card, Shift+click selects a range in reading order, Shift with the arrows, `pgup` `pgdn`, `home` or `end` stretches it, `ctrl+a` selects every card, Space toggles the card the keys are on and `esc` reduces several selected cards to that one; `cards` is always the whole new selection. Selected cards take the selected surface.
- `.box_select(bool)` — with `multi_select`, a drag from the free space between and after the cards draws a box in the `text-selection` tone and every card it touches becomes the selection, or joins it with Ctrl held at the press; a click there clears the selection.
- `.droppable(|drop| msg, |index| accepts)` — the pressed card, or the selection it is part of, is dragged onto a card `accepts` says yes to, which takes the `tree-drop` tone; `drop` is a `RowDrop { rows, into }`. A release anywhere else does nothing.
- `.on_copy_drop(|drop| msg)` — a drop released with Ctrl held asks for a copy with this instead of the move.
- `.empty(EmptyState)` — what an empty grid shows.
- `.context_menu(|index| Vec<ContextItem<Msg>>)` — gives every card a menu of its own, built for the card it opens on.
- `.disabled(bool)` — no hover, focus or press; cards fade, the selection still shows.
- `.scrollbar(ScrollbarStyle)` — pins a scrollbar style instead of the theme's.

## Keys

- Arrows move between cards and stop at the edges; `home` `end` go to the first and last card; `pgup` `pgdn` move the rows that fit; `enter` activates; `space` checks while checks are on, activates otherwise.

## Mouse

- With a card menu: a right click on a card opens that card's menu at the pointer and makes the card the selection unless it is checked; the menu key or `shift f10` opens the menu of the card the keys are on, scrolling it into view first. That card stays raised while the menu is open.
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
