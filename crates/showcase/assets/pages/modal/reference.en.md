## Methods

- `Modal::new()` — an empty dialog; add its content with `ui.add_with(modal, |ui| …)`.
- `.title(text)` — a bold heading in the first row.
- `.variant(name)` — theme variant; `"danger"` colours the pillar down the left edge with danger.
- `.width(cells)` — width with padding. Default: 56; narrow screens shrink it.
- `.on_close(msg)` — sent when the dialog is dismissed: Esc or a click on the close mark `×`. Without it neither exists.
- `.dismissable(bool)` — whether Esc, the close mark and the click outside close the dialog, all together. Default: `true`. `false` also hides the mark.
- `.close_on_click_outside(bool)` — also send the close message on a click on the dimmed screen, while dismissable. Default: `false`.
- `.action(button)` — a button in the action row at the bottom right, in the order added.

## Keys

- `tab` / `shift tab` cycle through the widgets inside the dialog only.
- `esc` sends the close message while the dialog is dismissable; inside stacked dialogs only the top one hears it.
- Every other key goes to the focused widget inside; it never reaches widgets beneath, and application shortcuts pause while a dialog is open.

## Mouse

- Widgets inside work as usual.
- The close mark `×` takes the top right corner of the surface, on the top padding row above the content: three cells that light up together under the pointer; a click sends the close message. It is drawn only while the dialog is dismissable.
- Clicks, wheel and drags on the dimmed screen are swallowed, or close the dialog with `close_on_click_outside`. Nothing beneath a dialog ever reacts.

## Behaviour

- Takes no space where it is added; drawn after the whole view, centred, over a screen blended towards the canvas.
- A pillar `▌` fills the first padding column on every row of the surface and enters with it; in ASCII mode it is a coloured cell.
- A dismissable dialog keeps at least one row of top padding and three cells of right padding, so the close mark never covers content.
- Enters over `motion.enter`: the surface grows two columns and one row on each side and fades in; at once with reduced motion.
- The first focusable widget inside gets focus on opening; focus returns to the previously focused widget on closing.
- The body is as tall as its content, clipped to the screen; put long content in a scroll view.
- The hint line shows `esc close` while dismissable and `tab switch` when more than one widget can take focus; hints that do not fit beside the actions are dropped.

## Theme keys

- `modal` — `bg`, `padding` (default `[1, 3]`), `pillar` (accent-muted); `modal.danger` — `pillar`.
- `modal-title` — `fg`, `bold`.
- `close-mark` — `fg`, `bg`, `bold`, with `active` and `hover`.
- `layer-backdrop` — `scrim` (colour the screen is blended towards), `strength` (percent).
- `layer-hint-key`, `layer-hint-label` — `fg`, `bold`.
- `[motion]` — `enter`.
- `[icons]` — `pillar`, `close`.
- Language — `quvyta.layer.close`, `quvyta.layer.switch`.
