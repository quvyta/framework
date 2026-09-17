## Methods

- `SidePanel::new(width)` — an open panel docked left; `.panel(|ui| ...)`, `.body(|ui| ...)`, `.show(ui)`.
- `.side(Side::Left | Side::Right)` — the docked edge.
- `.open(bool)` — whether the panel is open.
- `.on_toggle(|open| msg)` — the edge toggle, Enter on the edge and `toggle-panel` toggle it.
- `.on_resize(|width| msg)` — drag or arrow keys resize.
- `.limits(min, max)` — `max` is a number or `None` for no upper limit; 8 and `None` by default.
- `.strip(icons, |index| msg)` — an activity bar of view icons at the outer edge; the message means "show this view".
- `.active_view(index)` — the view the panel shows; its icon is raised with the pillar while open, and clicking it closes the panel.
- `.closed(Closed::Collapse | Closed::Hide)` — closing keeps the strip (default) or hides it too, leaving one edge column.

## Keys

- Inside the panel or body: `alt+b` (global action `toggle-panel`) toggles.
- On the focused edge: `enter` `space` toggle; `left` `right` resize, `shift` for five; `home` `end` jump to the limits.
- On the focused strip: `up` `down` move, `home` `end` jump to the first and last icon; `enter` `space` act like a click.

## Mouse

- Hover the edge for the toggle; click the toggle to open or close; drag the rest of the edge to resize; drag a closed edge into the body to open it.
- Strip icons: another icon switches the view (and opens a closed panel); the shown view's icon closes the panel.

## Behaviour

- The body keeps at least a quarter of the area, whatever the limits; a `max` below `min` counts as `min`.
- The toggle sits on the edge's middle row: the pillar cell then the arrow, taking the edge cell and one cell of the body. It shows while the edge or the toggle is pointed at, while dragging, and while the edge has focus reached with the keyboard. The pillar `▌` itself shows only while the pointer is on the toggle or keyboard focus is visible; with only the edge pointed at, the first cell stays plain in the toggle's tone.
- Tone ladder: edge pointed at (`side-toggle`) < toggle pointed at (`hover`) < held or just activated (`pressed`); keyboard focus (`focus`) breathes the pillar. A press that starts on the toggle never drags.
- Opening and closing slide over twice `motion.enter`; content keeps its width while it slides.
- Without `on_toggle` and `on_resize` the edge is inert and not focusable.
- The strip is 4 columns: the pillar, a two-cell icon and a spare column facing the panel. It stays at the outer edge while the panel is open; the panel's width does not include it, and the body still keeps a quarter of the area.
- Strip tone ladder: icon (`side-strip-item`) < pointed at (`hover`) < shown view (`selected`, steady pillar). The icon under keyboard focus (`focus`) breathes its pillar; Enter flashes it (`pressed`). The selected look shows only while the panel is open.
- `Closed::Hide`: closing slides the panel and the strip out together; the body takes the area but one edge column that has the body's tone at rest and the `split-handle` look with the toggle under the pointer. A hidden strip hands keyboard focus to the edge.
- A closed edge dragged half the minimum width into the body opens the panel at the dragged width.
- Tab order: the edge, the strip, the panel's content, the body.

## Theme keys

- `side-panel` — `bg`. `side-strip` — `bg`. `side-strip-item` — `fg`, `bg`, `pillar`; states `hover`, `selected`, `focus`, `pressed`.
- `split-handle` — `bg`, `fg`; states `hover`, `focus`, `active`.
- `side-toggle` — `bg`, `fg`, `pillar`; states `hover`, `focus`, `pressed`.
- Icons — `edge-left`, `edge-right`. Keymap — `[global] toggle-panel`; label `quvyta.keys.toggle-panel`.
