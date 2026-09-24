## When to use

Use a text area for text that runs over several lines: release notes, a commit message, a deploy script, a comment. For a name, a search or anything on one line use a text input; Enter then submits instead of starting a line.

## Step by step

1. Keep the text in your application: `summary: String`.
2. Draw it: `TextArea::new(&state.summary)`. The plain area wraps, scrolls and edits; it measures three to eight rows, growing with its text.
3. Handle changes: `.on_change(|text| Msg::Summary(text))` and store the text in `update`.
4. Give it a width with the layout, and a height too when it should not grow: `.height(Length::Cells(5))`.
5. Add capabilities only when you need them: `.placeholder(..)`, `.max_length(280).counter(true)`, `.line_numbers(true)` for scripts and code, `.variant("plain")` for a page of notes that should look like paper rather than a field.
6. To send the text somewhere, add `.on_submit(|text| Msg::Publish(text))` for Ctrl+Enter and put a button next to it as well.

## How it works

- **The same field, taller.** Surface, focus, invalid tint, cursor, selection colours and undo are those of `TextInput`.
- **Plain is paper.** `.variant("plain")` drops the raised field: the area takes the tone of the panel or page it sits on and keeps it on hover and focus, so a note tool reads as a sheet, not a form. The pillar still shows hover and focus, a danger pillar marks invalid text, and the cursor and the selection look the same.
- **Words wrap, nothing is lost.** Long lines break between words, a word longer than a row breaks between characters, and indentation at the start of a line stays. Wrapped rows are only a view: your text keeps its own line breaks.
- **Enter starts a line.** Submitting is up to you: Ctrl+Enter sends `on_submit` when it is set. Terminals without the kitty keyboard protocol may report Ctrl+Enter as plain Enter, which is why a button belongs next to it.
- **Moving:** ↑ and ↓ move between rows and keep the column, even through shorter rows; Home and End go to the start and end of the row, with Ctrl to the start and end of the text; Page Up and Page Down move a page. Shift with any of them selects. Ctrl+U deletes to the start of the line.
- **Scrolling follows the cursor.** A scrollbar appears on the right once the text is taller than the area; the wheel and dragging the bar scroll without moving the cursor.
- **Limits count characters.** A line break counts as one, pasted text is cut at the limit, and the counter shows `count / limit` below the text.
- **Line numbers** belong to lines, not rows: a wrapped line is numbered once, and the cursor's line number is brighter.
- **Arrows leave a selection behind.** Without Shift, ← → clear a selection and move one step on from its left or right end, ↑ ↓ a row on from its upper or lower end.
- **The same edit menu.** A right click opens Cut, Copy, Paste and Select all, with the rules of `TextInput`; pasted text keeps its line breaks.

## Common mistakes

- **Submitting on Enter.** In a text area people expect Enter to start a new line; use Ctrl+Enter and a button.
- **Relying on Ctrl+Enter alone.** Not every terminal reports it; always offer another way.
- **Growing without limit.** An area that grows with its text pushes the rest of the page away; fix its height when it sits among other controls.
