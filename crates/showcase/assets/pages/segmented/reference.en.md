## Methods

- `Segmented::new(options)` — segments with the first chosen.
- `.selected(usize)` — the chosen segment.
- `.disabled(bool)` — cannot be focused or changed.
- `.on_select(|index| msg)` — message for a newly chosen segment.

## Behaviour

- Each segment is its label plus the theme's horizontal padding on both sides; one row.
- Left and Right choose the neighbour, Home and End the ends; a click chooses the segment under the pointer.
- Choosing the segment that is already chosen sends nothing.

## Theme keys

- `segment` — `bg`, `fg`, `bold`, `padding`, `pillar` (drawn in the first cell of the hovered segment, or of the chosen one under keyboard focus); states `hover`, `focus`, `checked`, `disabled`.
