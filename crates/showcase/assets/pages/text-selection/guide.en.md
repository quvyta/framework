## When to use

Make text selectable where it is content people may want to take with them: a log, a message, a path, an ID, a document, code. In an application built with quvyta-framework nothing is selectable by default, so menus, titles, buttons and empty space never turn into a stray highlight. You opt in per widget, and a selection stays inside the widget it started in.

## Step by step

1. Mark the content: `ui.add_with(ScrollView::new(), |ui| { … }).selectable(true)`. `CodeView`, `Markdown` and `Terminal` are selectable by themselves; `Text` is not, so `ui.add(Text::new(url)).selectable(true)` where a message or an address is worth copying.
2. Drag over it to select. A double press takes a word, a triple press the line.
3. Releasing keeps the selection; nothing is copied yet. `ctrl c` copies it, and a right click on it opens **Copy** and **Raw copy**.
4. Keep secrets out: `.selectable(false)` on a node works inside a selectable area too, for example for a token in a log.
5. In a widget of your own, call `cx.selectable(rect)` while painting for its content, and `cx.decoration(rect)` for cells that are not content, such as a scrollbar or a gutter you draw yourself.
6. To react to copies, implement `App::clipboard` and handle `ClipboardEvent::Copied(text)`.

## How it works

- **Only what asks.** A press starts a selection only inside a selectable area, and only when no widget used the press: buttons, fields, lists, tabs and scrollbars keep working exactly as before.
- **The region is the widget.** The selection stays inside the innermost selectable widget under the press; a scroll view is narrowed to its content, without its scrollbar. Dragging past the edge stops at the edge. Nothing beneath a dialog can be selected.
- **Words and lines.** A word is a run of letters, digits and `_ - . / : @ ~ # % + =`, so paths, URLs, digests and versions come out whole.
- **It follows scrolling.** When the content scrolls, the highlight moves with its text; if the widget goes away, so does the selection.
- **Copy is clean.** Copy and `ctrl c` leave out what is only decoration: the pillar `▌` and the cell after a heading's pillar, scrollbars, line numbers and the padding of code blocks, and the spaces that only pad a line to the edge. Spaces between words and line breaks stay.
- **Raw copy is exact.** It takes every selected cell as it is shown, padding and decoration included, for when the layout itself matters.
- **Quiet confirmation.** A copy brightens the highlight for a moment. Any other key or a press elsewhere clears the selection; while the menu is open, its keys belong to the menu.

## Common mistakes

- **Making whole pages selectable.** Mark the content, not the page: a selection that starts on a menu or a title is noise.
- **Handling a press only to ignore it.** A widget that returns `true` for a mouse press blocks selection there; return `false` when you do not use it.
- **Drawing decoration as text without marking it.** Call `cx.decoration(rect)`, or clean copies pick it up.
- **Expecting releasing to copy.** It does not. If a screen depends on copying, a `CopyValue` or a hint about `ctrl c` and the right click helps.
