## When to use

Use a status line where a person looks to see how things stand: a footer under a container list, a line above a form, a summary under a chart. It stays as long as the state does and the application puts it where it belongs. Use a toast for news that goes away on its own, a badge for a count, and an empty state for an area with nothing in it.

## Step by step

1. Write the sentence: `StatusLine::new("3 of 3 containers healthy")`.
2. Name the status it reports: `.tone(ToastKind::Success)`. The sign and the colour both come from the kind.
3. Offer the way out when there is one: `.action(Button::new("Retry").on_press(Msg::Retry))`.
4. Give it the room it needs. A line in a column takes the width it wants; `.fill_width()` gives it the whole row and lets the sentence wrap.

## How it works

- **One row while there is room.** Sign, a space, the sentence, then the button after a gap of two cells.
- **A narrow area wraps, it never cuts.** The sentence wraps under itself and keeps its column; the sign keeps standing beside the words it belongs to. The button moves to a row of its own rather than being cut, so nothing ever ends with `…`.
- **Meaning is never colour alone.** The sign is the same glyph a toast of that kind shows, in the same colour: `✓` for success, `▲` for warning, `✕` for danger, `ℹ` for info, and one character each in ASCII and Nerd Font too. A line and a toast of the same kind look like one family.
- **The sentence is the application's text,** in the tone's colour, so it reads as a status rather than as a caption. A theme that would rather the sentence read as ordinary text names `status-line` for that kind and the sign keeps the tone.
- **The button is a real button.** It takes focus with tab and answers Enter, Space and the mouse with its own message, and it looks like every other button on the screen.
- **Only the button takes input.** The sentence and the sign are text; a press on them does nothing, so a stray click cannot send a message the user did not aim at.

## Common mistakes

- **A status line for news.** Something that happened and needs no answer is a toast; the line says how things are now.
- **More than one button.** The line is one row. A second action belongs on the content above it, or in a dialog.
- **Colour without a sign.** Writing a sentence in a status colour by hand is what this widget replaces: the sign and the colour cannot come apart, because both come from the kind.
- **A sentence that never ends.** A status line is a short sentence, not a log. Long news goes to a toast, a panel or a log view.
