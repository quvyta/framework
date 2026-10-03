## When to use

Use a text input for one line of text the user types: names, search terms, paths, codes. The value belongs to your application; everything about editing belongs to the widget.

## Step by step

1. Show your value: `TextInput::new(&self.name)`.
2. Receive every change: `.on_change(Msg::NameChanged)`. Store the new value in `update`; the field shows whatever your state holds.
3. Add `.placeholder(t!("..."))` to say what belongs there.
4. Validate in your code and mark the field: `.invalid(name.len() < 3)`, with a short message below it.
5. Use `.password(true)` for secrets, `.max_length(n)` for limits and `.on_submit(msg)` to act on Enter.
6. In a rename dialog, open with the name selected: `.select_on_focus(0..4)` for `main.rs`, counted in characters. Typing then replaces the name and keeps `.rs`; `.select_all_on_focus()` selects everything.
7. In an address bar or a package search, offer what the person may want: `.suggestions(rows)` and `.on_suggestion(|index| msg)`. You decide which rows belong there; the field shows the list and tells you which one was chosen.

## How it works

- **Real editing.** The cursor moves between characters, never inside a combined letter. Ctrl+arrows jump words, Shift selects, Home and End jump to the edges.
- **Undo that feels right.** Typing a word undoes as one step; deleting a word undoes on its own. Ctrl+Z undoes, Ctrl+Y or Ctrl+Shift+Z redoes.
- **Clipboard.** Ctrl+C and Ctrl+X copy and cut the selection; Ctrl+V pastes from the system clipboard, then the terminal's, then the last copy inside the application. Pasting with the terminal inserts text too, with line breaks turned into spaces.
- **Arrows leave a selection behind.** With a selection, ← and → clear it and move one step on from its left or right end, as in a text editor.
- **Mouse.** Click to place the cursor, drag to select. A right click opens Cut, Copy, Paste and Select all: inside the selection it keeps it, elsewhere it places the cursor first. Cut and Copy are disabled without a selection, Paste while there is nothing to paste. Shift+F10 and the menu key open the same menu.
- **Passwords never copy.** In a password field Cut, Copy, Ctrl+C and Ctrl+X do nothing.
- **The cursor stays solid while typing** and blinks only when you pause, at the theme's `motion.cursor-blink` rate.
- **Long values scroll** horizontally to keep the cursor in view.
- **A selection to start with.** With `select_on_focus` the field selects that part of its text each time it gains focus, with the cursor at the part's end. From then on the selection is the user's: typing replaces it, the arrows drop it. A click that brings focus places the cursor where it lands instead, and a range past the text is cut to it.
- **Outside changes win.** If your application changes the value, the field shows it and puts the cursor at the end.
- **A list of what may come next.** `.suggestions(...)` opens a list under the field as soon as the person types, exactly as wide as the field, so its edges stand where the field's do and a name longer than the field is cut there. The list is a menu: the chosen row is raised in tone with its `▌` pillar beside it, and its note and icon are the menu's own. `max_suggestions(n)` says how many rows it holds, eight by default.
- **Nothing is offered before it is asked for.** Gaining focus opens no list, and a field whose value is already there offers none either; the list opens once the text has moved on. The list needs `.on_suggestion` as well, and a field with neither changes nothing at all.
- **The typed text stands.** `↑` and `↓` move the chosen row, and `Enter` with a row chosen takes it. `↑` from the first row and `↓` from the last bring the typed text back, and `Enter` then submits as it always does. Typing chooses nothing again, `Esc` closes the list and keeps the text, and a press anywhere else closes it and still reaches whatever was pressed.

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
- **Counting the range in bytes.** `select_on_focus` counts characters, so `şğü.txt` selects its name with `0..3`; `str::find` returns bytes, so turn them into a character count first.
- **Using a text input for choices.** When the answers are known, use a select or a list; a text input with suggestions is for the answers that are not known yet.
- **Offering everything at once.** The list holds `max_suggestions` rows and no more; narrow your rows to what the person typed, and let the first ones be the ones you would choose yourself. `qframe::text::fuzzy` is the same matcher the filter, the command palette and the pickers find with.
