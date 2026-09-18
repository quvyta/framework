## When to use

Use a card grid when each item deserves a small surface of its own and the items read side by side: the apps of a store, the programs of a launcher, profiles to choose from. For a column of rows with details, use a list or a table.

## Step by step

1. Say how many cards there are: `CardGrid::new(apps.len())`.
2. Give the card sizes: `.card_width(24, 32)` (the least and the most a card gets), `.card_height(3)` rows of content, `.gap(2, 1)` between columns and rows. These are also the defaults.
3. Show the selection you keep in your state with `.selected(self.selected)`, and receive moves with `.on_select(Msg::Select)` and opening with `.on_activate(Msg::Open)`.
4. Build what a card shows with `.card(|ui, index| { .. })`. The closure owns what it reads: clone an `Rc<[App]>` from your state into it. Build text with `Text::no_wrap()`, so a narrow card cuts it with `…`.
5. For several choices at once add `.checked(vec_of_bools)` and `.on_toggle(Msg::Toggle)`.
6. Say what an empty grid means with `.empty(EmptyState::new(..).action(..))`.

## How it works

- **Columns follow the width.** As many cards as fit at the least width share the room, each at most the widest; the rest stays free on the right. With 24 to 32 cells and a gap of 2, a 96-cell area shows three columns and a 140-cell area five. An area narrower than one card shows one column as wide as the area.
- **Only the cards on screen are built.** The closure runs while the grid paints, for the visible cards only, so ten thousand cards cost what one screen of them costs.
- **The arrows move like a grid.** Left and right move along a row and stop at its ends; up and down keep the column, and down onto a shorter last row gives its last card. Home and End jump to the first and last card, PgUp and PgDn a screen of rows. The selected card stays in view.
- **One card is lit at a time.** Under the pointer a card rises one tone with a soft pillar down its left edge. While the pointer moves over the grid it carries the highlight and the selected card rests; the next key takes it back and goes on from the card the pointer was on.
- **A card is a surface, not a row.** The selected card takes the selected surface; its pillar breathes while the grid has focus reached with the keyboard. Nothing slides.
- **Checks sit in the corner.** A checked card carries the accent mark in its top right corner. The lit card offers a faint mark there: click it to check without opening. Space checks the selected card.
- **The mouse does what the keys do.** A click selects and opens a card, the wheel scrolls a row of cards, and the scrollbar can be pressed and dragged.

## Styling with a theme

```toml
[style.card]
bg      = "$raised"
padding = [0, 2]

[style."card:hover"]
bg     = "mix($text, $raised, 8%)"
pillar = "mix($accent, $active, 45%)"

[style."card:selected:focus"]
pillar = "pulse($accent, $accent-2)"
```

## Common mistakes

- **Borrowing the state in the closure.** The card closure lives as long as the widget; move an `Rc` or `Arc` clone into it instead of a reference.
- **Buttons inside a card.** Widgets in a card are drawn but take no input: the whole card is the pressable surface. Put actions on the page the card opens.
- **Wrapping text in a card.** A card has a fixed height; wrapped text is cut at the bottom. Use `no_wrap` so it ends in `…`.
