## When to use

Use a dialog when the user must finish or dismiss one small task before going on: renaming a project, reviewing a destructive action, editing a short list. If the user should keep working next to it, use a side panel or a popover instead. For a plain yes or no question, `Command::confirm` is shorter (see Confirmation).

## Step by step

1. Keep whether it is open in your state: `rename_open: bool`, or an enum of your dialogs.
2. In `view`, add it while it is open, anywhere in the tree: `ui.add_with(Modal::new().title(t!("rename")), |ui| { … })`. It takes no room where it is added.
3. Make it dismissable: `.on_close(Msg::CloseRename)`, and set the flag back in `update`. Esc and the × at the top right now both send it.
4. Add the buttons, safe action first: `.action(Button::new(t!("cancel")).on_press(Msg::CloseRename))`, then the main one.
5. For destructive dialogs add `.variant("danger")` and give the confirming button the danger variant too.
6. While the dialog must not be left, say during a save, add `.dismissable(false)`: Esc, × and the click outside stop together, and `on_close` stays for later.

## How it works

- **A layer over a dimmed screen.** The dialog is an overlay surface in the middle of the screen; everything behind it is blended towards the canvas colour. There is no frame: tone separates it, and a pillar `▌` runs down its whole left edge, accent-muted, in the danger colour for destructive dialogs.
- **It pops in.** The surface grows a few cells and fades in over the theme's `motion.enter`. With reduced motion it is simply there.
- **Focus stays inside.** When it opens, the first focusable widget inside gets focus. Tab and Shift+Tab cycle only through its widgets, keys never reach the widgets beneath, and application shortcuts (like the showcase's Esc for back) pause.
- **Focus comes back.** When you remove the dialog, focus returns to the widget that had it before, usually the button that opened it.
- **Dialogs stack.** A dialog added inside another one, or later in the view, is drawn on top and owns the keys; Esc closes only the top one.
- **Dismissable means Esc and × together.** A dialog with `on_close` closes with Esc and with the three-cell close mark `×` in its top right corner; the mark lights up under the pointer like a small button. There is never one without the other: `.dismissable(false)` turns both off and hides the mark.
- **The pointer is blocked.** Clicks on the dimmed screen do nothing unless `.close_on_click_outside(true)` is set, and even then only while the dialog is dismissable. Unlike a dropdown, a dialog keeps swallowing clicks: nothing beneath reacts until it closes.
- **Hints tell the keys.** A faint line at the bottom left shows `esc close` when Esc closes and `tab switch` when there is more than one widget to focus.

## Common mistakes

- **Opening dialogs for things that do not block.** A saved message or a finished deploy is a toast, not a dialog.
- **Putting the destructive button first.** The first focusable widget gets focus; make that the safe one so an accidental Enter does no harm.
- **A dialog without a way out.** Without `on_close`, or with `.dismissable(false)`, give it a button that closes it.
- **Wanting Esc without ×, or × without Esc.** They are one option on purpose: someone on the keyboard and someone on the mouse get the same way out.
- **Keeping cursor or scroll state in your app.** The dialog's widgets keep their own state while it is open.
