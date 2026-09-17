## Methods

- `NodeMut::selectable(bool)` — `true` makes the node a selection region; `false` keeps selection out of the node and everything in it, also inside a region. Without it a node is neither, so nothing is selectable by default.
- `PaintCx::selectable(rect)` — marks part of a widget as a selection region; `CodeView`, `Markdown` and `Terminal` call it for their content.
- `PaintCx::unselectable(rect)` — part of a widget where a press never starts a selection.
- `PaintCx::decoration(rect)` — cells a clean copy leaves out; `PaintCx::pillar` marks its own cell.
- `App::clipboard` with `ClipboardEvent::Copied(text)` — hears every copy of a selection.
- `Harness::drag(from, to)`, `Harness::click` twice or three times, `Harness::mouse(MouseKind::Down(MouseButton::Right), x, y)` for the menu, `Harness::clipboard()` for what was copied.

## Rules

1. Nothing is selectable unless a node or a widget asks.
2. A left press goes to widgets first; a widget that uses it keeps the mouse.
3. An unused press inside a region starts a selection, unless it lands in an unselectable area or the region lies beneath a dialog.
4. The region is the innermost one under the press; a focusable region such as a scroll view is narrowed to its child under the press, without the scrollbar.
5. Dragging selects by cells; presses on the same cell within 400 ms select a word, then a line.
6. Releasing keeps the selection and copies nothing; a plain click without moving selects nothing.
7. `copy` (`ctrl c`) copies clean and flashes for `motion.flash`. A right press on the selection opens Copy (clean) and Raw copy; the menu has the keys until it closes.
8. Clean copy: decoration cells are skipped, rows made only of decoration are dropped, spaces at the end of each row are trimmed. Raw copy: every selected cell.
9. The selection moves with the smallest widget under the first press; it clears when that widget or the region disappears, on a press elsewhere, or on any key except `copy`.

## Keys

- `[global] copy = "ctrl+c"`, label `quvyta.keys.copy` — copies the selection clean. A focused text field with its own selection handles `ctrl c` first.
- In the menu: ↑ ↓ move, Enter chooses, Esc closes.

## Theme keys

- `text-selection` (`bg`) and `text-selection:pressed` for the flash after copying.
- The menu draws with `context-menu` and `context-item`.

## Language keys

- `quvyta.edit.copy`, `quvyta.edit.raw-copy`.
