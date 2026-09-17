## Methods

- `Popover::new(open)` — shows the layer while `open` is true.
- `.anchor(|ui| …)` — the widgets the layer belongs to; they are laid out where the popover is added.
- `.content(|ui| …)` — the widgets inside the layer.
- `.on_dismiss(msg)` — sent on Esc and on a press outside the anchor and the layer.
- `.placement(Placement)` — `Below` (default), `Above`, `Right` or `Left`; flips when there is no room.
- `.focus_inside(bool)` — focus moves into the layer when it opens and back when it closes. Default: `false`.
- `.show(ui)` — adds it and returns the node, so `.id(…)` and sizes apply to the anchor.

## Placement

- `Placement::ALL`, `.name()` — every side and a short name for settings screens.

## Keys

- `esc` — sends the dismiss message.
- With focus inside, `tab` moves through the layer's widgets and on to the rest of the screen.

## Mouse

- A press outside sends the dismiss message and still reaches what it landed on. A press on a widget outside the anchor that opened the popover only dismisses it; a press on the anchor reaches the anchor, which usually toggles.
- Presses inside the layer and on the anchor work as usual.

## Theme keys

- `popover` — `bg` (default `$overlay`), `padding` (default `[1, 2]`).
- `[motion] enter` — how long the layer takes to unfold.
