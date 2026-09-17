## When to use

Use a text input for one line of text the user types: names, search terms, paths, codes. The value belongs to your application; everything about editing belongs to the widget.

## Step by step

1. Show your value: `TextInput::new(&self.name)`.
2. Receive every change: `.on_change(Msg::NameChanged)`. Store the new value in `update`; the field shows whatever your state holds.
3. Add `.placeholder(t!("..."))` to say what belongs there.
4. Validate in your code and mark the field: `.invalid(name.len() < 3)`, with a short message below it.
5. Use `.password(true)` for secrets, `.max_length(n)` for limits and `.on_submit(msg)` to act on Enter.

## How it works

- **Real editing.** The cursor moves between characters, never inside a combined letter. Ctrl+arrows jump words, Shift selects, Home and End jump to the edges.
- **Undo that feels right.** Typing a word undoes as one step; deleting a word undoes on its own. Ctrl+Z undoes, Ctrl+Y or Ctrl+Shift+Z redoes.
- **Clipboard.** Ctrl+C and Ctrl+X copy and cut the selection; Ctrl+V pastes from the system clipboard, then the terminal's, then the last copy inside the application. Pasting with the terminal inserts text too, with line breaks turned into spaces.
- **Arrows leave a selection behind.** With a selection, ← and → clear it and move one step on from its left or right end, as in a text editor.
- **Mouse.** Click to place the cursor, drag to select. A right click opens Cut, Copy, Paste and Select all: inside the selection it keeps it, elsewhere it places the cursor first. Cut and Copy are disabled without a selection, Paste while there is nothing to paste. Shift+F10 and the menu key open the same menu.
- **Passwords never copy.** In a password field Cut, Copy, Ctrl+C and Ctrl+X do nothing.
- **The cursor stays solid while typing** and blinks only when you pause, at the theme's `motion.cursor-blink` rate.
- **Long values scroll** horizontally to keep the cursor in view.
- **Outside changes win.** If your application changes the value, the field shows it and puts the cursor at the end.

## Styling with a theme

```toml
[style."text-input:focus"]
bg = "$active"

[style."text-input:invalid"]
bg = "mix($danger, $surface, 14%)"

[style.text-input-cursor]
bg = "$accent"
fg = "$ink"
```

An invalid field tints its surface; it never draws a red frame.

## Common mistakes

- **Validating on every keystroke with a loud message.** An empty field is not an error yet; show messages once the user typed something.
- **Keeping the cursor in your state.** The runtime keeps it for you.
- **Using a text input for choices.** When the answers are known, use a select or a list.
