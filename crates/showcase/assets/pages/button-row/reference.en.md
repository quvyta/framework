## Methods

- `ButtonRow::new()` — a row with no buttons in it yet.
- `.button(Button)` — adds a button at the end of the row, once for each; the order they are added in is the reading order and the order Tab visits them.
- Plain methods stay: every `Button` method, and the layout methods of a node (`.fill_width()`, `.width(Length::Cells(n))`, `.id(..)`).

## Behaviour

- The buttons are laid out from the left, two cells apart, at the width each button asks for.
- While every button fits, the row is a plain row of buttons and there is no other control in it.
- Where they do not fit, the control that opens the menu stands where the first button that does not fit would have been, and the last buttons, from the end, are listed in it. A row too narrow even for the first button shows the control alone, cut to the width it has.
- Choosing an entry sends the message that button would send when pressed, and closes the menu. A button that is `disabled` is listed like any other and still sends nothing when chosen.
- The row measures as wide as its buttons, or the width it is given where they do not fit, and one row tall (or as tall as the tallest button under a theme with vertical button padding).

## Keys

- `tab` reaches the buttons one after another, from the left, and then the control that opens the menu; the next `tab` leaves the row.
- `enter` or `space` on the control opens the menu. In the menu `up` `down` `home` `end` move, typing a letter jumps to an entry, `enter` takes it and `esc` closes.

## Mouse

- Click a button to press it, the control to open the menu, and an entry in the menu to take that button's message.
- A press on a button beside an open menu closes the menu and still presses the button.

## Theme keys

- `button` and its variants — the buttons, and the control that opens the menu, which takes `hover`, `focus` (the pillar breathes) and `pressed` like any button.
- `popup-menu`, `popup-item` (`hover`), `popup-check` — the menu of the buttons that do not fit.

## Icons

- `chevron-down` before the control's label, whose words are the `quvyta.button-row.more` locale key.
