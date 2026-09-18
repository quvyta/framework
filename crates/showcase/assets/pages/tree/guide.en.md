## When to use

Use a tree for things that contain other things and are browsed one level at a time: folders and files, services and their containers, a document outline. When everything sits on one level, use a list.

## Step by step

1. Build nodes with a stable key and a label: `TreeNode::new("src/lib.rs", "lib.rs")`. The key names the node in every message, so use a path or an id, never a position.
2. Give folders their children with `.children([...])` and say which are open with `.expanded(self.open.contains(key))`. A set of open keys in your state is the simplest way to keep track.
3. Show the selection with `.selected(self.selected.as_deref())` and receive it with `.on_select(|key| ..)`.
4. Open and close with `.on_expand(|key, open| ..)`: add the key to your open set or remove it.
5. Leaves are opened with `.on_activate(|key| ..)`.
6. For children that must be fetched, mark the node `.expandable(true)`; when it opens, return a `Command::perform` that reads them and show `.loading(true)` until the answer arrives.
7. To let people put nodes in their own order, add `.reorderable(|step| Msg::Move(step))` and apply the `TreeMove` to your list of siblings with `step.apply(&mut siblings)`.
8. For actions on a node — rename, archive, move elsewhere — add `.context_menu(|key| vec![ContextItem::new(..), ..])`.

## How it works

- **Indentation is space.** Each level moves two cells right; no guide lines are drawn. Folders carry a small chevron that points down when open.
- **Keys follow the shape.** → opens a folder or steps into it, ← closes it or steps out to its parent, Enter opens and closes folders and opens files.
- **The chevron is its own target.** A click on the chevron only opens or closes; a click on the name also selects.
- **Only the icon and the name slide.** A hovered or selected row rises and moves its icon and name one cell right. The indentation, the chevron and the detail stay where they are, so the chevron never runs away from the pointer.
- **Only the open part exists.** The tree flattens just the open nodes and paints just the rows on screen, so a folder with fifty thousand files scrolls smoothly.
- **Reordering stays among siblings.** A dragged node moves among its brothers and sisters only: they make room as it passes, a ghost row follows the pointer and the dragged node's own children fold away until it lands. Ctrl+Shift+↑/↓ does the same from the keyboard. Held on the top or bottom row, the drag scrolls the tree. With reordering on, a click acts when the button is released, so pressing a folder to drag it does not open it.
- **The menu belongs to a node.** A right click opens the menu of the row under the pointer and keeps that row raised; the menu key or Shift+F10 opens the menu of the selected node below it.
- **Loading has a face.** A folder whose children take longer than 300 ms turns its chevron into a spinner, which then stays at least 500 ms so it never blinks; a quick load never shows it; an unreadable folder can show its error as a faint child with a danger mark.

## Common mistakes

- **Using row positions as keys.** Positions change when folders open; keys must not.
- **Reading the disk in `view`.** Read folders in a `Command::perform` and store the result.
- **Moving a node to another parent by dragging.** A drag never changes a node's parent: dropping into a folder is easy to do by accident and hard to undo. Offer "Move to…" in the context menu instead.
- **Forgetting to close.** Handle `open = false` too, or folders never fold again.
