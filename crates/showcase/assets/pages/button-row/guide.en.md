## When to use

Use a button row for the actions of one place: a small editor's toolbar, a form's own buttons, the
choices at the end of a screen. The row decides what does not fit, so the application does not have
to choose which action to hide when the terminal is narrow, and every action keeps the same place
in the reading order. It is not a tab strip (each button acts at once) and not a dialog's action
row inside a `Modal` or an `EmptyState`, which lay their own buttons out.

## Step by step

1. Build the row and add the buttons, one call each: `ButtonRow::new().button(...)`.
2. Give every button its own message, as a plain `Button` takes: `Button::new("Save").on_press(Msg::Save)`.
3. Give the row the width it may use: `.fill_width()` in a panel, or `.width(Length::Cells(60))`
   when it must not take the whole line.
4. In `update`, do what the message says. A button chosen from the menu sends the same message as
   one pressed in the row, so there is nothing to tell apart.

## How it works

- **The buttons are ordinary buttons.** Each keeps its label, icon, shortcut, variant and disabled
  state, and stands two cells after the one before it, the gap a dialog's action row keeps.
- **A narrow row moves its end into a menu.** While every button fits there is no other control in
  the row. Where they do not fit, the last ones, from the end, move into a `More` control that
  stands where the first of them would have been. Choosing an entry sends that button's message;
  a row too narrow even for one button shows the control alone, cut.
- **The menu is the framework's own.** The same popup a tab strip uses for its hidden tabs and a
  breadcrumb for its hidden levels, so ↑ ↓ Home End, type-ahead, Enter and Esc behave the way they
  do everywhere else.
- **The control is a button like any other.** The same surface, padding and label as `button`,
  with the pillar in its first cell on hover and on focus, and a press that flashes it. The words
  come from the `quvyta.button-row.more` locale key, so they are in the reader's language.
- **Keyboard.** Tab reaches the buttons one after another, from the left, and then the control.
  Enter or Space opens its menu, and the menu takes the keyboard until a choice is made or Esc
  closes it. A press on a button beside an open menu closes the menu and still presses the button,
  so one click never has to be spent closing a menu first.

## Common mistakes

- **Expecting the control at the end of the screen.** It stands where the first button that does
  not fit would have stood, so a row wider than its buttons leaves empty space to its right.
- **Hiding buttons in the application as well.** Let the row do it: a button that is in the menu
  sends the same message as one in the row, so there is one path for every action.
- **Expecting a name for a single button.** A button in a row is not a node of its own, so
  `Command::focus("save")` cannot reach it; focus the row and let Tab walk it.
