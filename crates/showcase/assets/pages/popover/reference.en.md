## Methods

- `Popover::new(open)` — shows the layer while `open` is true.
- `.anchor(|ui| …)` — the widgets the layer belongs to; they are laid out where the popover is added.
- `.content(|ui| …)` — the widgets inside the layer.
- `.on_dismiss(msg)` — sent on Esc and on a press outside the anchor and the layer.
- `.placement(Placement)` — `Below` (default), `Above`, `Right` or `Left`; flips when there is no room.
- `.focus_inside(bool)` — focus moves into the layer when it opens and back when it closes. Default: `false`.
- `.match_anchor_width(bool)` — the layer opens exactly as wide as the anchor, cut to the screen, instead of as wide as its content; the content is measured at that width. Default: `false`.
- `.show(ui)` — adds it and returns the node, so `.id(…)` and sizes apply to the anchor.

## Width

- The layer is as wide as its content by default. With `.match_anchor_width(true)` its width is the anchor's, cut to the screen, and its content is measured at that width: a line longer than the anchor is cut there rather than widening the layer. The anchor's area is the node `.show(ui)` returns, so `.width(Length::Cells(n))` on it sets how wide the anchor, and with the option the layer, is.

## Placement

- `Placement::ALL`, `.name()` — every side and a short name for settings screens.

## Standing apart

- `PaintCx::floating(rect, paint)` — for a floating surface of your own: paint it with this from `paint_overlay`. It reads the ring of cells just outside `rect`, runs `paint`, and when the surface's tone sits too close to a ground covering at least a quarter of the ring, moves every background inside `rect` together. Text colours stay.

## Keys

- `esc` — sends the dismiss message.
- With focus inside, `tab` moves through the layer's widgets and on to the rest of the screen.

## Mouse

- A press outside sends the dismiss message and still reaches what it landed on. A press on a widget outside the anchor that opened the popover only dismisses it; a press on the anchor reaches the anchor, which usually toggles.
- Presses inside the layer and on the anchor work as usual.

## Theme keys

- `popover` — `bg` (default `$overlay`), `padding` (default `[1, 2]`). Over a ground closer than 0.05 in OKLab the background steps towards `$text` or `$canvas` by the least that clears it, at most 30%.
- `[motion] enter` — how long the layer takes to unfold.
