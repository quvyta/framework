## Methods

- `HoldToConfirm::new(label)` — a focusable chip with the label and three bars.
- `.on_confirm(msg)` — sent once when the hold completes. Without it the control is inactive.
- `.duration(duration)` — how long to hold. Default: 1.2 s.
- `.key(chord)` — holding the chord anywhere confirms too, while the control is in the view.
- `.floating(bool)` — takes no room; shows a card in the top left corner only while held. Default: `false`.
- `.disabled(bool)` — greyed out, not focusable, holding does nothing.
- `.color(expression)` — the colour a full bar has, written like a theme colour: `"$danger"`, `"$success"`, `"mix($accent, $danger, 50%)"` or `"#RRGGBB"`. Default: the theme's `hold.to` (warning). Theme tokens are the goal; a fixed hex works but ignores the theme. An expression that is not one colour of the current theme falls back to `hold.to`.
- `Theme::solid(expression)` — resolves such an expression against a theme, or returns why it cannot.

## Keys

- `enter` / `space` held while focused.
- The `key` chord held anywhere, after focused widgets had their chance and before the keymap; only inside the top dialog while one is open.

## Mouse

- Hold the left button on the control. Moving off it or releasing cancels.

## Behaviour

- 9 cells in 3 bars of 3 with one cell between; the label takes the rest and is cut with `…` when narrow.
- Bar `n` (0, 1, 2) blends as a whole from `track` to `to` while the progress goes from `n/3` to `(n+1)/3` of the duration; the message is sent when the progress reaches 1. At 1/6 the first bar is halfway, at 1/2 the first is full and the second halfway, at 5/6 the third is halfway.
- Letting go empties the bars from where they were over `motion.enter`; a floating card stays until they are empty.
- With `.color`, the bars and the floating card's pillar blend towards that colour instead of `to`; the blend and the timing are the same. The expression is resolved every frame, so a theme change applies at once.
- Hover and focus draw the pillar in the first cell.
- A key counts as released on a release event (kitty keyboard protocol) or when no repeat arrives within 650 ms of the press or 350 ms of the last repeat.
- Enter and Space repeats, which the runtime keeps from pressing buttons again, still reach the control.
- A held mouse button is checked every 40 ms through the runtime's pointer repeat.
- Reduced motion switches each bar at the end of its third and empties at once; the duration stays the same.

## Theme keys

- `hold` — `bg`, `fg`, `bold`, `padding`, `track` (empty bar), `to` (full bar; the warning tone in the built-in themes), `pillar`; states `hover`, `focus`, `active` (held), `disabled`.
- `hold-card` — `bg`, `padding`, `pillar` (by default blending from `muted` to `to` as the hold goes on).
- `[motion]` — `enter` (emptying).
- `[icons]` — `pillar`.
