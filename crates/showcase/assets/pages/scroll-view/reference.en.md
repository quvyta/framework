## Methods

- `ScrollView::new()` — add content with `ui.add_with`.
- Size it with `.fill()`, `.height(Length::Cells(n))` or any length.
- `.scrollbar(ScrollbarStyle)` — pins a scrollbar style instead of the theme's.

## Keys

- While focused: `up` `down` one row, `pgup` `pgdn` one page, `home` `end` to the edges.

## Mouse

- Wheel scrolls three rows; click or drag the scrollbar.

## Behaviour

- Scrolls to reveal a newly focused widget inside it.
- Keeps its position per id; inside `ui.page` it survives while the page is hidden.

## Theme keys

- `scrollbar` with `hover` — `style` (`block`, the default, `half`, `thin`, `dots`), `track`, `thumb`; `scrollbar.<style>` tunes one style.

## Related

- `ScrollMetrics` — `total`, `visible`, `offset`, `.overflows()`, `.thumb(track)`, `.offset_at(row, track)`, `.max_offset()` for custom scrolling widgets.
