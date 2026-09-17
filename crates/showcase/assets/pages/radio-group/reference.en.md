## Methods

- `RadioGroup::new(options)` — a vertical group with nothing chosen.
- `.selected(Option<usize>)` — the chosen option.
- `.horizontal(bool)` — one row with four cells between options.
- `.style(RadioStyle)` — `Square` (default): a small centred square for every option; the chosen one takes the chosen colour. `Mark`: the same square, grown into a full two-cell box when chosen. `Box`: the checkbox's two-cell colour box. `Dot`: a dot and a ring.
- `.disabled(bool)` — cannot be focused or changed.
- `.on_select(|index| msg)` — message for a newly chosen option.

## Behaviour

- Vertical: as wide as the longest option plus four (`Square`, `Mark`, `Box`) or three (`Dot`), one row per option. Horizontal: one row. The mark keeps its width in every state and frame.
- `Square`: always `🬇🬃` (U+1FB07 and U+1FB03). Choosing blends the new square from the quiet tone to the chosen tone over two `motion.step`s while the old one blends back; the shape never changes. `Mark` shapes: small `🬇🬃` and full (two cells of colour), nothing in between; the same blend, and the shape swaps halfway. Reduced motion changes at once.
- Sextants are drawn by kitty, WezTerm, Ghostty and foot themselves; other terminals need a font with Symbols for Legacy Computing, or replace the icons.
- In ASCII the square icon is blank: the mark is a box whose tone blends from the faint box to the chosen box.
- `Box` blends each box between the empty colour and the filled colour over `motion.step` × 3, each option on its own; reduced motion switches at once.
- Up/Down (vertical) or Left/Right (horizontal) choose the neighbour; Home and End the ends; Space or Enter chooses the current or first option; a click chooses the option under the pointer.
- Choosing the option that is already chosen sends nothing.

## Theme keys

- `radio.mark` — `fg` of the mark style: the square, and the colour of the full box when `checked`; states `hover`, `focus`, `checked`, `disabled`.
- `radio.box` — `bg` of the box style, and of marks whose size icon is blank; the same states. The built-in themes give it the same tones as `checkbox`.
- `radio` — `fg` of the dot style; the same states.
- `radio-label` — `fg`, `bold`; the same states.
- `[motion]` — `step` (the mark's size steps and the box blend).
- `[icons]` — `radio-mark-small` (mark style, two cells; a theme can replace it), `dot`, `dot-outline` (dot style).
