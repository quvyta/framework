## When to use

Use a context menu for actions on the thing under the pointer: restart a container, copy an id, delete a file. It keeps rows clean, because the actions appear only when asked for. Every action in it should also be reachable another way (a key, a toolbar button), since not everyone right clicks.

## Step by step

1. Build the items: `ContextItem::new(t!("restart"), Msg::Restart)`; add `.icon("…")`, `.shortcut("ctrl r")`, `.detail(t!("no-shell"))`, `.disabled(true)` or `.danger(true)` as needed.
2. Group related actions with `ContextItem::gap()`, and nest with `ContextItem::submenu(t!("move"), [...])`.
3. Wrap the area: `ui.add_with(ContextMenu::new(items), |ui| { … })`.
4. Handle the messages in `update`, like any button's.
5. When the area exists only to show the menu, such as a status bar item that lists sessions, add `.on_left_click(true)`: a left click opens it and a second one closes it.

## How it works

- **Right click opens it at the pointer**, one row under the clicked cell. Shift+F10 or the menu key opens it below the focused widget inside the area, with the first action highlighted.
- **It is a layer on the overlay surface**, unfolding over `motion.enter`, flipping above or to the left when there is no room. Groups are separated by an empty row, never by a line.
- **Rows behave like list rows.** The highlighted row raises its surface, shows the pillar and slides its label one cell; shortcuts and the submenu arrow stay at the right edge in the faint tone.
- **A row can say why.** `.detail(…)` puts a short note on the right in the muted tone, before the shortcut or the arrow when there is one: a disabled row reads `Attach terminal   no shell in image` instead of being grey for no reason. The menu widens to fit the note; where the screen is too narrow the note is cut first, then left out, and the label stays whole.
- **The keyboard stays inside.** ↑ and ↓ move and skip gaps and disabled rows, Home and End jump, a letter jumps to the next row starting with it, →, Enter or Space opens a submenu, ← or Esc closes it, Enter or Space chooses.
- **The pointer works too.** Hovering a submenu row opens it; clicking a row chooses it, and a click on a gap or a disabled row does nothing. A press anywhere else closes the menu and still reaches what it landed on, so one click both dismisses the menu and does what you aimed at. A right click inside the area opens it again at the new place.
- **A left click can open it too.** With `.on_left_click(true)` the area works as a button for its menu: a left click opens the menu at the pointer, exactly where a right click would, and a left click on the area while it is open closes it. The keys and the pointer then work as in any context menu. A child that takes presses itself, such as a button or a list, keeps its left click.
- **Destructive actions are marked** with the danger colour and should carry an icon, so colour is never the only sign.
- **Text brings its own menu.** Text fields open Cut, Copy, Paste and Select all, and selected text opens Copy and Raw copy; both are drawn by this same menu, so they look and move exactly like yours.

## Common mistakes

- **Actions only in the menu.** Give frequent actions a key or a button as well.
- **Expecting the shortcut label to bind the key.** `.shortcut(…)` only shows it; bind it in the keymap.
- **Putting a note in the shortcut slot.** `.shortcut(…)` is for key names; a reason goes in `.detail(…)`, which sits before the key.
- **Deep nesting.** One level of submenus is plenty; more is hard to steer with a pointer in a terminal.
