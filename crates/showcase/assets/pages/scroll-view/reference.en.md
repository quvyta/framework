## Methods

- `ScrollView::new()` — add content with `ui.add_with`.
- Size it with `.fill()`, `.height(Length::Cells(n))` or any length.
- `.scrollbar(ScrollbarStyle)` — pins a scrollbar style instead of the theme's.
- `.follow_end(bool)` — keeps the end of growing content in view while the person is at the end; off by default.

## Keys

- While focused: `up` `down` one row, `pgup` `pgdn` one page, `home` `end` to the edges. With `follow_end`, `end` also follows the end again.

## Mouse

- Wheel scrolls three rows; click or drag the scrollbar.
- With `follow_end`, a click on the lines-below note follows the end again.

## Behaviour

- Scrolls to reveal a newly focused widget inside it.
- Keeps its position per id; inside `ui.page` it survives while the page is hidden.
- With `follow_end`: opens at the end; growth glides to the new end over the theme's `page` duration, or jumps with reduced motion. Scrolling up (wheel, keys, scrollbar) stops following and shows the lines-below note; reaching the bottom again follows. A move to show a focused widget or a revealed area stops following when it leaves the end and keeps it when it ends there. Content that fits always counts as at the end.

## Theme keys

- `scrollbar` with `hover` — `style` (`block`, the default, `half`, `thin`, `dots`), `track`, `thumb`; `scrollbar.<style>` tunes one style.
- `log-more` — the lines-below note, shared with `LogView`. Its text is the framework string `quvyta.log.below`.

## Related

- `ScrollMetrics` — `total`, `visible`, `offset`, `.overflows()`, `.thumb(track)`, `.offset_at(row, track)`, `.max_offset()` for custom scrolling widgets.
