## Methods

- `Panel::new()` — an untitled panel; add content with `ui.add_with`.
- `.title(text)` — heading in the first row, followed by one empty row.
- `.variant(name)` — theme variant, e.g. `inset`.
- `.gap(rows)` — rows between children. Default: 1.
- `.selected(bool)` — selected surface and pillar. Default: `false`.
- `.on_press(msg)` — makes the panel pressable: hover, focus and pressed looks, focus order, keys and clicks.
- `.disabled(bool)` — a pressable panel that does not react: no hover, focus or press. Still shows `selected`. Default: `false`.

## Behaviour

- Hover: the surface lifts 8% towards `$text` (at least 1.2:1 against rest in the built-in themes), the title turns `$dim`, a soft pillar runs down the panel's whole left edge.
- Pressed (a flash of `motion.flash`): 16% towards `$text`, title `$text`, pillar `$accent`.
- Keyboard focus: like hover with a breathing pillar. Focus from a click is not shown.
- Selected: `$active` with a full-height breathing pillar; hovering a selected panel lifts it 8% too.
- A panel without `on_press`, or disabled, never changes under the pointer.

## Messages

- The `on_press` message on Enter, Space or a left click released over the panel; releasing elsewhere cancels.

## Theme keys

- `panel` with `hover`, `focus`, `pressed`, `selected` — `bg`, `padding`, `pillar`.
- `panel.<variant>` — e.g. `panel.inset`.
- `panel-title` with `hover`, `focus`, `pressed` — `fg`, `bold`.
